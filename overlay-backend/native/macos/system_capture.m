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
// AudioDeviceStart on a private tap can hit the first-run TCC consent flow
// and block (or restart the audio stack), so Rust starts and rebuilds this
// controller only on its dedicated system-audio worker.
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

@interface SuflyorSystemCaptureState : NSObject {
@public
  float *ring;
  uint32_t capacity;
  uint32_t channels;
  uint32_t bytes_per_frame;
  bool interleaved;
  _Atomic uint64_t head;
  _Atomic uint64_t tail;
  _Atomic uint64_t dropped;
  _Atomic bool route_changed;
}
@end

@implementation SuflyorSystemCaptureState
- (void)dealloc {
  free(ring);
  ring = NULL;
}
@end

typedef struct SystemCaptureController {
  AudioObjectID tap;           // private process tap
  AudioObjectID aggregate;     // private aggregate device
  AudioDeviceIOProcID io_proc; // NULL until the callback is attached
  void *state_ref;             // retained SuflyorSystemCaptureState
  bool default_output_listener;
  bool tap_format_listener;
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

static const AudioObjectPropertyAddress DEFAULT_OUTPUT_ADDRESS = {
    kAudioHardwarePropertyDefaultOutputDevice,
    kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyElementMain,
};

static const AudioObjectPropertyAddress TAP_FORMAT_ADDRESS = {
    kAudioTapPropertyFormat,
    kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyElementMain,
};

static OSStatus route_changed_listener(
    AudioObjectID object, UInt32 address_count,
    const AudioObjectPropertyAddress addresses[], void *context) {
  (void)object;
  (void)address_count;
  (void)addresses;
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)context;
  if (state != nil) {
    atomic_store_explicit(&state->route_changed, true, memory_order_release);
  }
  return noErr;
}

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
// system_capture_stop. The IOProc captures `state`, never the C controller.
// If HAL refuses to unregister a callback, retain that small state instead of
// risking a callback-after-free.
static void system_capture_teardown(SystemCaptureController *c) {
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)c->state_ref;
  bool safe_release = true;
  if (c->default_output_listener) {
    OSStatus status = AudioObjectRemovePropertyListener(
        kAudioObjectSystemObject, &DEFAULT_OUTPUT_ADDRESS,
        route_changed_listener, (__bridge void *)state);
    safe_release = safe_release && status == noErr;
    c->default_output_listener = false;
  }
  if (c->tap_format_listener && c->tap != kAudioObjectUnknown) {
    OSStatus status = AudioObjectRemovePropertyListener(
        c->tap, &TAP_FORMAT_ADDRESS, route_changed_listener,
        (__bridge void *)state);
    safe_release = safe_release && status == noErr;
    c->tap_format_listener = false;
  }
  if (c->aggregate != kAudioObjectUnknown && c->io_proc != NULL) {
    AudioDeviceStop(c->aggregate, c->io_proc);
    OSStatus status = AudioDeviceDestroyIOProcID(c->aggregate, c->io_proc);
    safe_release = safe_release && status == noErr;
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
  if (safe_release && c->state_ref != NULL) {
    CFRelease((CFTypeRef)c->state_ref);
  }
  c->state_ref = NULL;
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
  NSString *aggregate_uid = [NSString
      stringWithFormat:@"com.ninitux.suflyor.system-capture.aggregate.%@",
                       [NSUUID UUID].UUIDString];
  NSDictionary *aggregate_description = @{
    @kAudioAggregateDeviceNameKey : @"Suflyor system capture aggregate",
    @kAudioAggregateDeviceUIDKey : aggregate_uid,
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
  UInt32 format_size = sizeof(format);
  status = AudioObjectGetPropertyData(tap, &TAP_FORMAT_ADDRESS, 0, NULL,
                                      &format_size, &format);
  bool usable = status == noErr && format.mBytesPerFrame > 0 &&
                format.mChannelsPerFrame > 0 && format.mBitsPerChannel == 32 &&
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
  SuflyorSystemCaptureState *state = [[SuflyorSystemCaptureState alloc] init];
  if (state != nil) {
    state->ring = (float *)calloc(capacity, sizeof(float));
  }
  SystemCaptureController *c =
      (SystemCaptureController *)calloc(1, sizeof(SystemCaptureController));
  if (state == nil || state->ring == NULL || c == NULL) {
    free(c);
    AudioHardwareDestroyAggregateDevice(aggregate);
    AudioHardwareDestroyProcessTap(tap);
    if (out_error) {
      *out_error = SYS_START_NO_MEMORY;
    }
    return NULL;
  }
  state->capacity = capacity;
  state->channels = format.mChannelsPerFrame;
  state->bytes_per_frame = format.mBytesPerFrame;
  state->interleaved =
      (format.mFormatFlags & kLinearPCMFormatFlagIsNonInterleaved) == 0;
  atomic_init(&state->head, 0);
  atomic_init(&state->tail, 0);
  atomic_init(&state->dropped, 0);
  atomic_init(&state->route_changed, false);

  c->tap = tap;
  c->aggregate = aggregate;
  c->io_proc = NULL;
  c->state_ref = (void *)CFBridgingRetain(state);
  c->default_output_listener = false;
  c->tap_format_listener = false;

  // 6) Realtime IOProc: downmix into the ring + atomics only. The block
  // retains state independently of the C controller, so a HAL teardown
  // failure cannot turn a late callback into a use-after-free.
  uint32_t channels = state->channels;
  uint32_t bytes_per_frame = state->bytes_per_frame;
  bool interleaved = state->interleaved;
  SuflyorSystemCaptureState *callback_state = state;
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
            atomic_load_explicit(&callback_state->head, memory_order_relaxed);
        uint64_t tail =
            atomic_load_explicit(&callback_state->tail, memory_order_acquire);
        uint64_t free_slots = callback_state->capacity - (head - tail);
        if ((uint64_t)frames > free_slots) {
          // Keep whatever the consumer has not drained yet and drop this
          // callback wholesale; the Rust worker reads the counter and logs
          // outside the realtime path.
          atomic_fetch_add_explicit(&callback_state->dropped, frames,
                                    memory_order_relaxed);
          return;
        }
        uint32_t mask = callback_state->capacity - 1;
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
            callback_state->ring[(head + i) & mask] = acc / (float)channels;
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
            callback_state->ring[(head + i) & mask] = acc / (float)ch;
          }
        }
        atomic_store_explicit(&callback_state->head, head + frames,
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

  status = AudioObjectAddPropertyListener(
      kAudioObjectSystemObject, &DEFAULT_OUTPUT_ADDRESS,
      route_changed_listener, (__bridge void *)state);
  if (status == noErr) {
    c->default_output_listener = true;
    status = AudioObjectAddPropertyListener(
        tap, &TAP_FORMAT_ADDRESS, route_changed_listener,
        (__bridge void *)state);
  }
  if (status == noErr) {
    c->tap_format_listener = true;
  } else {
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
  if (!c || c->state_ref == NULL) {
    return 0;
  }
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)c->state_ref;
  return state->capacity;
}

// Consumer side of the SPSC ring. Returns the number of mono f32 frames
// copied into dst (at most max_frames).
uint32_t system_capture_read(SystemCaptureController *c, float *dst,
                             uint32_t max_frames) {
  if (!c || c->state_ref == NULL || !dst || max_frames == 0) {
    return 0;
  }
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)c->state_ref;
  uint64_t tail = atomic_load_explicit(&state->tail, memory_order_relaxed);
  uint64_t head = atomic_load_explicit(&state->head, memory_order_acquire);
  uint64_t available = head - tail;
  uint32_t n = available < max_frames ? (uint32_t)available : max_frames;
  uint32_t mask = state->capacity - 1;
  for (uint32_t i = 0; i < n; i++) {
    dst[i] = state->ring[(tail + i) & mask];
  }
  atomic_store_explicit(&state->tail, tail + n, memory_order_release);
  return n;
}

// Take-and-reset the IOProc overflow counter so the Rust worker can log it.
uint64_t system_capture_take_dropped(SystemCaptureController *c) {
  if (!c || c->state_ref == NULL) {
    return 0;
  }
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)c->state_ref;
  return atomic_exchange_explicit(&state->dropped, 0, memory_order_relaxed);
}

uint32_t system_capture_take_route_change(SystemCaptureController *c) {
  if (!c || c->state_ref == NULL) {
    return 0;
  }
  SuflyorSystemCaptureState *state =
      (__bridge SuflyorSystemCaptureState *)c->state_ref;
  return atomic_exchange_explicit(&state->route_changed, false,
                                  memory_order_acq_rel)
             ? 1
             : 0;
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
