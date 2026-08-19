// Native Core Audio system-output capture bridge for overlay-backend
// (macOS only).
//
// Pull-based C ABI mirroring mic_capture.m: the Rust worker owns the
// lifecycle — system_capture_start builds a PRIVATE process tap + private
// aggregate device with one preallocated bounded SPSC f32 mono ring,
// system_capture_read drains it, system_capture_stop tears everything down.
// The private tap mirrors all system output without muting playback — no
// BlackHole, no device re-routing.
//
// Realtime rule: the IOProc ONLY downmixes the incoming buffer into the
// ring and updates C11 atomics. No allocation, no locks, no I/O, no
// logging, and no Rust callback inside it.
//
// Teardown order is deterministic: stop audio device, destroy IOProc,
// destroy private aggregate device, destroy process tap, free ring +
// controller. Partial-start failures clean up every already-created object.
//
// WARNING for the wiring packet: AudioDeviceStart on a private tap can hit
// the first-run TCC consent flow and block (or restart the audio stack).
// This packet only compiles and link-tests the seam — nothing calls
// system_capture_start yet.
//
// Compiled with ARC (the gate0b-proven tap path); mic_capture.m stays
// non-ARC.

#import <CoreAudio/AudioHardware.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>
#import <Foundation/Foundation.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct SystemCaptureController {
  AudioObjectID tap;           // private process tap
  AudioObjectID aggregate;     // private aggregate device
  AudioDeviceIOProcID io_proc; // NULL until the callback is attached
  float *ring;                 // owned, capacity is a power of two (samples)
  uint32_t capacity;
  uint32_t channels;      // native tap channel count
  uint32_t bytes_per_frame; // native tap frame stride
  bool interleaved;       // native tap buffer layout
  _Atomic uint64_t head;    // next write slot, producer = IOProc
  _Atomic uint64_t tail;    // next read slot, consumer = Rust worker
  _Atomic uint64_t dropped; // frames the IOProc dropped on ring overflow
} SystemCaptureController;

enum {
  SYS_START_OK = 0,
  SYS_START_TAP = 1,       // process tap creation or UID query refused
  SYS_START_AGGREGATE = 2, // aggregate device creation refused
  SYS_START_FORMAT = 3,    // tap format is not readable float32 PCM
  SYS_START_IO_PROC = 4,   // callback attach refused
  SYS_START_DEVICE = 5,    // AudioDeviceStart refused (incl. TCC timeout)
  SYS_START_NO_MEMORY = 6,
};

// ~1 s of audio at the native rate, at least 8192 frames and at least
// buffer_frames; power of two so the ring can wrap with a mask instead of
// a modulo.
static uint32_t ring_capacity_for(double sample_rate, uint32_t buffer_frames) {
  uint32_t capacity = 8192;
  while ((double)capacity < sample_rate) {
    capacity <<= 1;
  }
  while (capacity < buffer_frames) {
    capacity <<= 1;
  }
  return capacity;
}

// Deterministic teardown used by BOTH partial-start cleanup and
// system_capture_stop. Frees the ring + controller too, so callers must
// not touch `c` afterwards.
static void system_capture_teardown(SystemCaptureController *c) {
  if (c->aggregate != kAudioObjectUnknown && c->io_proc != NULL) {
    AudioDeviceStop(c->aggregate, c->io_proc);
    AudioDeviceDestroyIOProcID(c->aggregate, c->io_proc);
    c->io_proc = NULL;
  }
  if (c->aggregate != kAudioObjectUnknown) {
    AudioHardwareDestroyAggregateDevice(c->aggregate);
    c->aggregate = kAudioObjectUnknown;
  }
  if (c->tap != kAudioObjectUnknown) {
    AudioHardwareDestroyProcessTap(c->tap);
    c->tap = kAudioObjectUnknown;
  }
  free(c->ring);
  c->ring = NULL;
  free(c);
}

SystemCaptureController *system_capture_start(uint32_t buffer_frames,
                                              double *out_sample_rate,
                                              int32_t *out_error) {
  if (out_error) {
    *out_error = SYS_START_OK;
  }
  if (out_sample_rate) {
    *out_sample_rate = 0.0;
  }

  // 1) Private, unmuted process tap over all system output.
  CATapDescription *description =
      [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
  description.name = @"Suflyor system audio capture";
  description.privateTap = YES;
  description.muteBehavior = CATapUnmuted;

  AudioObjectID tap = kAudioObjectUnknown;
  OSStatus status = AudioHardwareCreateProcessTap(description, &tap);
  if (status != noErr) {
    if (out_error) {
      *out_error = SYS_START_TAP;
    }
    return NULL;
  }

  // 2) Tap UID for the aggregate device description.
  AudioObjectPropertyAddress uid_address = {
      kAudioTapPropertyUID,
      kAudioObjectPropertyScopeGlobal,
      kAudioObjectPropertyElementMain,
  };
  CFStringRef tap_uid = NULL;
  UInt32 uid_size = sizeof(tap_uid);
  status =
      AudioObjectGetPropertyData(tap, &uid_address, 0, NULL, &uid_size, &tap_uid);
  if (status != noErr || tap_uid == NULL) {
    if (tap_uid != NULL) {
      CFRelease(tap_uid);
    }
    AudioHardwareDestroyProcessTap(tap);
    if (out_error) {
      *out_error = SYS_START_TAP;
    }
    return NULL;
  }

  // 3) Private aggregate device wrapping only the tap (gate0b dictionary).
  NSDictionary *tap_entry = @{@kAudioSubTapUIDKey : (__bridge NSString *)tap_uid};
  NSDictionary *aggregate_description = @{
    @kAudioAggregateDeviceNameKey : @"Suflyor system capture aggregate",
    @kAudioAggregateDeviceUIDKey :
        @"com.ninitux.suflyor.system-capture.aggregate",
    @kAudioAggregateDeviceIsPrivateKey : @YES,
    @kAudioAggregateDeviceTapListKey : @[ tap_entry ],
  };
  AudioObjectID aggregate = kAudioObjectUnknown;
  status = AudioHardwareCreateAggregateDevice(
      (__bridge CFDictionaryRef)aggregate_description, &aggregate);
  CFRelease(tap_uid);
  if (status != noErr) {
    AudioHardwareDestroyProcessTap(tap);
    if (out_error) {
      *out_error = SYS_START_AGGREGATE;
    }
    return NULL;
  }

  // 4) The tap must expose readable float32 PCM.
  AudioStreamBasicDescription format = {0};
  AudioObjectPropertyAddress format_address = {
      kAudioTapPropertyFormat,
      kAudioObjectPropertyScopeGlobal,
      kAudioObjectPropertyElementMain,
  };
  UInt32 format_size = sizeof(format);
  status = AudioObjectGetPropertyData(tap, &format_address, 0, NULL,
                                      &format_size, &format);
  bool usable = status == noErr && format.mBytesPerFrame > 0 &&
                format.mChannelsPerFrame > 0 &&
                format.mFormatID == kAudioFormatLinearPCM &&
                (format.mFormatFlags & kLinearPCMFormatFlagIsFloat) != 0;
  if (!usable) {
    AudioHardwareDestroyAggregateDevice(aggregate);
    AudioHardwareDestroyProcessTap(tap);
    if (out_error) {
      *out_error = SYS_START_FORMAT;
    }
    return NULL;
  }

  // 5) Ring + controller, allocated before the IOProc so teardown covers
  //    both on the remaining failure paths.
  uint32_t capacity = ring_capacity_for(format.mSampleRate, buffer_frames);
  float *ring = (float *)calloc(capacity, sizeof(float));
  SystemCaptureController *c = NULL;
  if (ring != NULL) {
    c = (SystemCaptureController *)calloc(1, sizeof(SystemCaptureController));
  }
  if (c == NULL) {
    free(ring);
    AudioHardwareDestroyAggregateDevice(aggregate);
    AudioHardwareDestroyProcessTap(tap);
    if (out_error) {
      *out_error = SYS_START_NO_MEMORY;
    }
    return NULL;
  }
  c->tap = tap;
  c->aggregate = aggregate;
  c->io_proc = NULL;
  c->ring = ring;
  c->capacity = capacity;
  c->channels = format.mChannelsPerFrame;
  c->bytes_per_frame = format.mBytesPerFrame;
  c->interleaved =
      (format.mFormatFlags & kLinearPCMFormatFlagIsNonInterleaved) == 0;
  atomic_init(&c->head, 0);
  atomic_init(&c->tail, 0);
  atomic_init(&c->dropped, 0);

  // 6) Realtime IOProc: downmix into the ring + atomics only.
  uint32_t channels = c->channels;
  uint32_t bytes_per_frame = c->bytes_per_frame;
  bool interleaved = c->interleaved;
  SystemCaptureController *cc = c;
  status = AudioDeviceCreateIOProcIDWithBlock(
      &c->io_proc, aggregate, NULL,
      ^(const AudioTimeStamp *now,
        const AudioBufferList *input,
        const AudioTimeStamp *input_time,
        AudioBufferList *output,
        const AudioTimeStamp *output_time) {
        (void)now;
        (void)input_time;
        (void)output;
        (void)output_time;
        if (input == NULL || input->mNumberBuffers == 0) {
          return;
        }
        uint32_t frames = 0;
        for (UInt32 index = 0; index < input->mNumberBuffers; index++) {
          uint32_t buffer_frames =
              input->mBuffers[index].mDataByteSize / bytes_per_frame;
          frames = frames > buffer_frames ? frames : buffer_frames;
        }
        if (frames == 0) {
          return;
        }
        uint64_t head =
            atomic_load_explicit(&cc->head, memory_order_relaxed);
        uint64_t tail =
            atomic_load_explicit(&cc->tail, memory_order_acquire);
        uint64_t free_slots = cc->capacity - (head - tail);
        if ((uint64_t)frames > free_slots) {
          // Keep whatever the consumer has not drained yet and drop this
          // callback wholesale; the Rust worker reads the counter and logs
          // outside the realtime path.
          atomic_fetch_add_explicit(&cc->dropped, frames,
                                    memory_order_relaxed);
          return;
        }
        uint32_t mask = cc->capacity - 1;
        if (interleaved) {
          const float *src = (const float *)input->mBuffers[0].mData;
          if (src == NULL) {
            return;
          }
          for (uint32_t i = 0; i < frames; i++) {
            float acc = 0.0f;
            for (uint32_t ch = 0; ch < channels; ch++) {
              acc += src[i * channels + ch];
            }
            cc->ring[(head + i) & mask] = acc / (float)channels;
          }
        } else {
          uint32_t ch = input->mNumberBuffers < channels
                            ? input->mNumberBuffers
                            : channels;
          if (ch == 0) {
            return;
          }
          for (uint32_t i = 0; i < frames; i++) {
            float acc = 0.0f;
            for (uint32_t channel = 0; channel < ch; channel++) {
              const float *samples =
                  (const float *)input->mBuffers[channel].mData;
              if (samples != NULL) {
                acc += samples[i];
              }
            }
            cc->ring[(head + i) & mask] = acc / (float)ch;
          }
        }
        atomic_store_explicit(&cc->head, head + frames,
                              memory_order_release);
      });
  if (status != noErr) {
    system_capture_teardown(c);
    if (out_error) {
      *out_error = SYS_START_IO_PROC;
    }
    return NULL;
  }

  // 7) Start the aggregate device (first run may block on TCC consent —
  //    see the header warning).
  status = AudioDeviceStart(aggregate, c->io_proc);
  if (status != noErr) {
    system_capture_teardown(c);
    if (out_error) {
      *out_error = SYS_START_DEVICE;
    }
    return NULL;
  }

  if (out_sample_rate) {
    *out_sample_rate = format.mSampleRate;
  }
  return c;
}

uint32_t system_capture_ring_capacity(const SystemCaptureController *c) {
  return c ? c->capacity : 0;
}

// Consumer side of the SPSC ring. Returns the number of mono f32 frames
// copied into dst (at most max_frames).
uint32_t system_capture_read(SystemCaptureController *c, float *dst,
                             uint32_t max_frames) {
  if (!c || !dst || max_frames == 0) {
    return 0;
  }
  uint64_t tail = atomic_load_explicit(&c->tail, memory_order_relaxed);
  uint64_t head = atomic_load_explicit(&c->head, memory_order_acquire);
  uint64_t available = head - tail;
  uint32_t n = available < max_frames ? (uint32_t)available : max_frames;
  uint32_t mask = c->capacity - 1;
  for (uint32_t i = 0; i < n; i++) {
    dst[i] = c->ring[(tail + i) & mask];
  }
  atomic_store_explicit(&c->tail, tail + n, memory_order_release);
  return n;
}

// Take-and-reset the IOProc overflow counter so the Rust worker can log it.
uint64_t system_capture_take_dropped(SystemCaptureController *c) {
  if (!c) {
    return 0;
  }
  return atomic_exchange_explicit(&c->dropped, 0, memory_order_relaxed);
}

// Synchronous, NULL-safe teardown — see system_capture_teardown for the
// deterministic order. The Rust worker is the only caller and calls this
// exactly once per successfully started controller.
void system_capture_stop(SystemCaptureController *c) {
  if (!c) {
    return;
  }
  system_capture_teardown(c);
}
