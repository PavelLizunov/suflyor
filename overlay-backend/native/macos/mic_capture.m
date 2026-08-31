// Native AVAudioEngine microphone bridge for overlay-backend (macOS only).
//
// Pull-based C ABI: the Rust worker owns the lifecycle — mic_capture_start
// creates one controller with a preallocated bounded SPSC f32 mono ring,
// mic_capture_read drains it, mic_capture_stop tears everything down.
//
// Realtime rule: the installTap block ONLY downmixes the incoming buffer
// into the ring and updates C11 atomics. No allocation, no locks, no I/O,
// no logging, and no Rust callback inside the tap.
//
// Compiled without ARC (manual retain/release) so the controller stays a
// plain C struct with explicit ownership.

#import <AVFoundation/AVFoundation.h>
#import <CoreAudio/CoreAudio.h>
#import <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

@interface SuflyorMicRouteFlag : NSObject {
@public
  _Atomic bool changed;
}
@end

@implementation SuflyorMicRouteFlag
@end

typedef struct MicController {
  AVAudioEngine *engine; // owned, released in mic_capture_stop
  id route_observer;     // AVAudioEngine configuration-change observer
  SuflyorMicRouteFlag *route_flag; // owned, retained by the observer block too
  float *ring;           // owned, capacity is a power of two (samples)
  uint32_t capacity;
  _Atomic uint64_t head;    // next write slot, producer = audio tap
  _Atomic uint64_t tail;    // next read slot, consumer = Rust worker
  _Atomic uint64_t dropped; // frames the tap dropped on ring overflow
} MicController;

enum {
  MIC_START_OK = 0,
  MIC_START_PERMISSION = 1, // not determined / denied / restricted — never prompts
  MIC_START_NO_INPUT = 2,
  MIC_START_ENGINE = 3,
};

enum {
  MIC_PERMISSION_NOT_DETERMINED = 0,
  MIC_PERMISSION_RESTRICTED = 1,
  MIC_PERMISSION_DENIED = 2,
  MIC_PERMISSION_AUTHORIZED = 3,
};

typedef void (*MicPermissionCallback)(uint32_t status, void *context);

// ~1 s of audio at the native rate, at least 8192 frames; power of two so
// the ring can wrap with a mask instead of a modulo.
static uint32_t ring_capacity_for(double sample_rate) {
  uint32_t capacity = 8192;
  while ((double)capacity < sample_rate) {
    capacity <<= 1;
  }
  return capacity;
}

uint32_t mic_capture_permission_status(void) {
  AVAuthorizationStatus status =
      [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
  switch (status) {
  case AVAuthorizationStatusNotDetermined:
    return MIC_PERMISSION_NOT_DETERMINED;
  case AVAuthorizationStatusRestricted:
    return MIC_PERMISSION_RESTRICTED;
  case AVAuthorizationStatusDenied:
    return MIC_PERMISSION_DENIED;
  case AVAuthorizationStatusAuthorized:
    return MIC_PERMISSION_AUTHORIZED;
  }
  return MIC_PERMISSION_NOT_DETERMINED;
}

void mic_capture_request_permission(MicPermissionCallback callback,
                                    void *context) {
  if (!callback) {
    return;
  }
  // A process without an attributable bundle identity and a non-empty
  // purpose string cannot own a microphone TCC prompt, so the bridge
  // answers restricted synchronously instead of forwarding the request.
  NSBundle *bundle = [NSBundle mainBundle];
  NSString *purpose =
      bundle.bundleIdentifier != nil
          ? [bundle objectForInfoDictionaryKey:@"NSMicrophoneUsageDescription"]
          : nil;
  if (purpose.length == 0) {
    callback(MIC_PERMISSION_RESTRICTED, context);
    return;
  }
  uint32_t status = mic_capture_permission_status();
  if (status != MIC_PERMISSION_NOT_DETERMINED) {
    callback(status, context);
    return;
  }
  [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                           completionHandler:^(BOOL granted) {
                             (void)granted;
                             callback(mic_capture_permission_status(), context);
                           }];
}

MicController *mic_capture_start(uint32_t buffer_frames,
                                 double *out_sample_rate,
                                 int32_t *out_error) {
  if (out_error) {
    *out_error = MIC_START_OK;
  }
  uint32_t perm = mic_capture_permission_status();
  if (perm == MIC_PERMISSION_DENIED || perm == MIC_PERMISSION_RESTRICTED) {
    if (out_error) {
      *out_error = MIC_START_PERMISSION;
    }
    return NULL;
  }

  AVAudioEngine *engine = [[AVAudioEngine alloc] init];
  AVAudioInputNode *input = engine.inputNode;
  AVAudioFormat *format = [input outputFormatForBus:0];
  if (format.sampleRate <= 0.0 || format.channelCount == 0) {
    [engine release];
    if (out_error) {
      *out_error = MIC_START_NO_INPUT;
    }
    return NULL;
  }

  uint32_t capacity = ring_capacity_for(format.sampleRate);
  float *ring = (float *)calloc(capacity, sizeof(float));
  if (!ring) {
    [engine release];
    if (out_error) {
      *out_error = MIC_START_ENGINE;
    }
    return NULL;
  }
  MicController *c = (MicController *)calloc(1, sizeof(MicController));
  SuflyorMicRouteFlag *route_flag = [[SuflyorMicRouteFlag alloc] init];
  if (!c || !route_flag) {
    [route_flag release];
    free(c);
    free(ring);
    [engine release];
    if (out_error) {
      *out_error = MIC_START_ENGINE;
    }
    return NULL;
  }
  c->engine = engine; // transfer ownership into the controller
  c->route_observer = nil;
  c->route_flag = route_flag;
  c->ring = ring;
  c->capacity = capacity;
  atomic_init(&c->head, 0);
  atomic_init(&c->tail, 0);
  atomic_init(&c->dropped, 0);
  atomic_init(&route_flag->changed, false);

  uint32_t channels = format.channelCount;
  [input installTapOnBus:0
              bufferSize:buffer_frames
                  format:nil
                   block:^(AVAudioPCMBuffer *buffer, AVAudioTime *timestamp) {
                     (void)timestamp;
                     uint32_t frames = buffer.frameLength;
                     float *const *data = buffer.floatChannelData;
                     if (!data || frames == 0) {
                       return;
                     }
                     uint64_t head =
                         atomic_load_explicit(&c->head, memory_order_relaxed);
                     uint64_t tail =
                         atomic_load_explicit(&c->tail, memory_order_acquire);
                     uint64_t free_slots = c->capacity - (head - tail);
                     if (frames > free_slots) {
                       // Keep whatever the consumer has not drained yet and
                       // drop this callback wholesale; the Rust worker reads
                       // the counter and logs outside the realtime path.
                       atomic_fetch_add_explicit(&c->dropped, frames,
                                                 memory_order_relaxed);
                       return;
                     }
                     uint32_t mask = c->capacity - 1;
                     if (channels == 1) {
                       const float *src = data[0];
                       for (uint32_t i = 0; i < frames; i++) {
                         c->ring[(head + i) & mask] = src[i];
                       }
                     } else {
                       for (uint32_t i = 0; i < frames; i++) {
                         float acc = 0.0f;
                         for (uint32_t ch = 0; ch < channels; ch++) {
                           acc += data[ch][i];
                         }
                         c->ring[(head + i) & mask] = acc / (float)channels;
                       }
                     }
                     atomic_store_explicit(&c->head, head + frames,
                                           memory_order_release);
                   }];

  NSError *error = nil;
  if (![engine startAndReturnError:&error]) {
    [input removeTapOnBus:0];
    [c->route_flag release];
    c->route_flag = nil;
    free(c->ring);
    [engine release];
    free(c);
    if (out_error) {
      *out_error = MIC_START_ENGINE;
    }
    return NULL;
  }

  c->route_observer =
      [[NSNotificationCenter defaultCenter]
          addObserverForName:AVAudioEngineConfigurationChangeNotification
                      object:engine
                       queue:nil
                  usingBlock:^(NSNotification *notification) {
                    (void)notification;
                    atomic_store_explicit(&route_flag->changed, true,
                                          memory_order_release);
                  }];

  if (out_sample_rate) {
    *out_sample_rate = format.sampleRate;
  }
  return c;
}

uint32_t mic_capture_ring_capacity(const MicController *c) {
  return c ? c->capacity : 0;
}

// Consumer side of the SPSC ring. Returns the number of mono f32 frames
// copied into dst (at most max_frames).
uint32_t mic_capture_read(MicController *c, float *dst, uint32_t max_frames) {
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

// Take-and-reset the tap overflow counter so the Rust worker can log it.
uint64_t mic_capture_take_dropped(MicController *c) {
  if (!c) {
    return 0;
  }
  return atomic_exchange_explicit(&c->dropped, 0, memory_order_relaxed);
}

uint32_t mic_capture_take_route_change(MicController *c) {
  if (!c || !c->route_flag) {
    return 0;
  }
  return atomic_exchange_explicit(&c->route_flag->changed, false,
                                  memory_order_acq_rel)
             ? 1
             : 0;
}

// Synchronous teardown: remove the tap, stop the engine, then release the
// engine and free the ring + controller. The
// Rust worker is the only caller and must join before freeing anything else.
void mic_capture_stop(MicController *c) {
  if (!c) {
    return;
  }
  if (c->route_observer) {
    [[NSNotificationCenter defaultCenter] removeObserver:c->route_observer];
    c->route_observer = nil;
  }
  if (c->engine) {
    [c->engine stop];
    [c->engine.inputNode removeTapOnBus:0];
    [c->engine release];
    c->engine = nil;
  }
  [c->route_flag release];
  c->route_flag = nil;
  free(c->ring);
  c->ring = NULL;
  free(c);
}

#ifndef kAudioObjectPropertyElementMain
#define kAudioObjectPropertyElementMain kAudioObjectPropertyElementMaster
#endif

static char *copy_default_device_name(AudioObjectPropertySelector selector) {
  @autoreleasepool {
    AudioObjectID device_id = kAudioObjectUnknown;
    UInt32 data_size = sizeof(device_id);
    AudioObjectPropertyAddress address = {
        .mSelector = selector,
        .mScope = kAudioObjectPropertyScopeGlobal,
        .mElement = kAudioObjectPropertyElementMain};

    OSStatus status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject, &address, 0, NULL, &data_size, &device_id);
    if (status != noErr || device_id == kAudioObjectUnknown) {
      return NULL;
    }

    CFStringRef cf_name = NULL;
    data_size = sizeof(cf_name);
    AudioObjectPropertyAddress name_address = {
        .mSelector = kAudioObjectPropertyName,
        .mScope = kAudioObjectPropertyScopeGlobal,
        .mElement = kAudioObjectPropertyElementMain};

    status = AudioObjectGetPropertyData(device_id, &name_address, 0, NULL,
                                        &data_size, &cf_name);
    if (status != noErr || !cf_name) {
      return NULL;
    }

    NSString *name = (NSString *)cf_name;
    const char *utf8 = [name UTF8String];
    char *result = utf8 && utf8[0] != '\0' ? strdup(utf8) : NULL;
    [name release];
    return result;
  }
}

char *mic_capture_copy_default_input_name(void) {
  return copy_default_device_name(kAudioHardwarePropertyDefaultInputDevice);
}

char *mic_capture_copy_default_output_name(void) {
  return copy_default_device_name(kAudioHardwarePropertyDefaultOutputDevice);
}

void mic_capture_free_string(char *s) {
  if (s) {
    free(s);
  }
}
