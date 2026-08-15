#import <AppKit/AppKit.h>
#import <AVFoundation/AVFoundation.h>
#import <CoreAudio/AudioHardware.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

#include <math.h>
#include <mach/message.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

typedef struct {
    uint32_t mic_permission;
    uint32_t mic_device_available;
    uint32_t mic_running;
    uint32_t system_running;
    uint32_t screen_allowed;
    uint32_t screenshot_width;
    uint32_t screenshot_height;
    int32_t last_error;
    uint64_t mic_frames;
    uint32_t mic_peak_milli;
    uint32_t system_starting;
    uint64_t system_frames;
} SuflyorGate0BSnapshot;

static _Atomic uint32_t gateMicPermission;
static _Atomic uint32_t gateMicDeviceAvailable;
static _Atomic uint32_t gateMicRunning;
static _Atomic uint32_t gateSystemRunning;
static _Atomic uint32_t gateScreenAllowed;
static _Atomic uint32_t gateScreenshotWidth;
static _Atomic uint32_t gateScreenshotHeight;
static _Atomic int32_t gateLastError;
static _Atomic uint64_t gateMicFrames;
static _Atomic uint32_t gateMicPeakMilli;
static _Atomic uint32_t gateSystemStarting;
static _Atomic uint64_t gateSystemFrames;
static NSLock *gateMessageLock;
static NSString *gateMessage;

static void set_message(NSString *message, int32_t code) {
    [gateMessageLock lock];
    gateMessage = [message copy];
    [gateMessageLock unlock];
    atomic_store(&gateLastError, code);
    fprintf(stderr, "[gate0b] %s code=%d\n", message.UTF8String, code);
}

static uint32_t microphone_permission(void) {
    AVAuthorizationStatus status =
        [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
    switch (status) {
        case AVAuthorizationStatusNotDetermined:
            return 0;
        case AVAuthorizationStatusRestricted:
            return 1;
        case AVAuthorizationStatusDenied:
            return 2;
        case AVAuthorizationStatusAuthorized:
            return 3;
    }
}

@interface SuflyorGate0BCaptureController : NSObject
@property(nonatomic, strong) AVAudioEngine *microphoneEngine;
@property(nonatomic) AudioObjectID tapID;
@property(nonatomic) AudioObjectID aggregateDeviceID;
@property(nonatomic) AudioDeviceIOProcID systemIOProc;
@property(nonatomic) dispatch_queue_t systemQueue;
- (void)refresh;
- (void)requestMicrophone;
- (void)startMicrophone;
- (void)stopMicrophone;
- (void)startSystemAudio;
- (void)stopSystemAudio;
- (void)stopSystemAudioLocked;
- (void)captureScreen;
- (void)shutdown;
@end

@implementation SuflyorGate0BCaptureController

- (instancetype)init {
    self = [super init];
    if (self != nil) {
        _tapID = kAudioObjectUnknown;
        _aggregateDeviceID = kAudioObjectUnknown;
        _systemIOProc = NULL;
        _systemQueue = dispatch_queue_create("com.ninitux.suflyor.gate0b.audio", DISPATCH_QUEUE_SERIAL);
    }
    return self;
}

- (void)refresh {
    atomic_store(&gateMicPermission, microphone_permission());
    atomic_store(&gateMicDeviceAvailable,
                 [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeAudio] != nil);
    atomic_store(&gateScreenAllowed, CGPreflightScreenCaptureAccess());
}

- (void)requestMicrophone {
    [self refresh];
    uint32_t state = atomic_load(&gateMicPermission);
    if (state == 3) {
        set_message(@"Microphone permission is already allowed.", 0);
        return;
    }
    if (state == 1 || state == 2) {
        set_message(@"Microphone permission must be changed in Privacy & Security.", 0);
        return;
    }

    set_message(@"Waiting for the microphone permission decision.", 0);
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                             completionHandler:^(BOOL granted) {
        dispatch_async(dispatch_get_main_queue(), ^{
            [self refresh];
            set_message(granted ? @"Microphone permission allowed."
                                : @"Microphone permission denied.",
                        0);
        });
    }];
}

- (void)startMicrophone {
    [self refresh];
    if (atomic_load(&gateMicPermission) != 3) {
        set_message(@"Allow microphone access before starting input.", 0);
        return;
    }
    if (atomic_load(&gateMicRunning) != 0) {
        set_message(@"Microphone input is already running.", 0);
        return;
    }
    if (atomic_load(&gateMicDeviceAvailable) == 0) {
        set_message(@"No default microphone input is available.", 0);
        return;
    }

    AVAudioEngine *engine = [AVAudioEngine new];
    AVAudioInputNode *input = engine.inputNode;
    AVAudioFormat *format = [input outputFormatForBus:0];
    if (format.channelCount == 0 || format.sampleRate <= 0) {
        set_message(@"The default input has no readable audio channels.", 0);
        return;
    }

    atomic_store(&gateMicFrames, 0);
    atomic_store(&gateMicPeakMilli, 0);
    [input installTapOnBus:0
                bufferSize:1024
                    format:format
                     block:^(AVAudioPCMBuffer *buffer, AVAudioTime *when) {
        (void)when;
        atomic_fetch_add(&gateMicFrames, buffer.frameLength);
        float peak = 0.0f;
        float *const *channels = buffer.floatChannelData;
        if (channels != NULL) {
            for (AVAudioChannelCount channel = 0; channel < buffer.format.channelCount; channel++) {
                float *samples = channels[channel];
                if (samples == NULL) {
                    continue;
                }
                for (AVAudioFrameCount frame = 0; frame < buffer.frameLength; frame++) {
                    peak = fmaxf(peak, fabsf(samples[frame]));
                }
            }
        }
        atomic_store(&gateMicPeakMilli, (uint32_t)fminf(1000.0f, peak * 1000.0f));
    }];

    NSError *error = nil;
    if (![engine startAndReturnError:&error]) {
        [input removeTapOnBus:0];
        fprintf(stderr, "[gate0b] microphone start failed code=%ld\n", (long)error.code);
        set_message(@"The default microphone input failed to start.", (int32_t)error.code);
        return;
    }

    self.microphoneEngine = engine;
    atomic_store(&gateMicRunning, 1);
    set_message(@"Microphone input started.", 0);
}

- (void)stopMicrophone {
    AVAudioEngine *engine = self.microphoneEngine;
    if (engine != nil) {
        [engine.inputNode removeTapOnBus:0];
        [engine stop];
        self.microphoneEngine = nil;
    }
    atomic_store(&gateMicRunning, 0);
    set_message(@"Microphone input stopped.", 0);
}

- (void)startSystemAudio {
    if (atomic_load(&gateSystemRunning) != 0 ||
        atomic_exchange(&gateSystemStarting, 1) != 0) {
        set_message(@"The Core Audio Tap stream is already running or starting.", 0);
        return;
    }

    set_message(@"Starting the private Core Audio Tap stream.", 0);
    dispatch_async(self.systemQueue, ^{
        CATapDescription *description =
            [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
        description.name = @"Suflyor Gate 0B system audio";
        description.privateTap = YES;
        description.muteBehavior = CATapUnmuted;

        fprintf(stderr, "[gate0b] tap-create begin\n");
        OSStatus status = AudioHardwareCreateProcessTap(description, &_tapID);
        fprintf(stderr, "[gate0b] tap-create end code=%d\n", status);
        if (status != noErr) {
            set_message(@"Core Audio refused to create the system tap.", status);
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        AudioObjectPropertyAddress uidAddress = {
            kAudioTapPropertyUID,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        };
        CFStringRef tapUID = NULL;
        UInt32 uidSize = sizeof(tapUID);
        fprintf(stderr, "[gate0b] tap-uid begin\n");
        status = AudioObjectGetPropertyData(_tapID, &uidAddress, 0, NULL, &uidSize, &tapUID);
        fprintf(stderr, "[gate0b] tap-uid end code=%d\n", status);
        if (status != noErr || tapUID == NULL) {
            set_message(@"Core Audio did not return a tap identifier.", status);
            [self stopSystemAudioLocked];
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        NSDictionary *tapEntry = @{@kAudioSubTapUIDKey : (__bridge NSString *)tapUID};
        NSDictionary *aggregateDescription = @{
            @kAudioAggregateDeviceNameKey : @"Suflyor Gate 0B private aggregate",
            @kAudioAggregateDeviceUIDKey : @"com.ninitux.suflyor.gate0b.aggregate",
            @kAudioAggregateDeviceIsPrivateKey : @YES,
            @kAudioAggregateDeviceTapListKey : @[ tapEntry ],
        };
        fprintf(stderr, "[gate0b] aggregate-create begin\n");
        status = AudioHardwareCreateAggregateDevice(
            (__bridge CFDictionaryRef)aggregateDescription, &_aggregateDeviceID);
        fprintf(stderr, "[gate0b] aggregate-create end code=%d\n", status);
        CFRelease(tapUID);
        if (status != noErr) {
            set_message(@"Core Audio refused to create the private aggregate device.", status);
            [self stopSystemAudioLocked];
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        AudioStreamBasicDescription format = {0};
        AudioObjectPropertyAddress formatAddress = {
            kAudioTapPropertyFormat,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        };
        UInt32 formatSize = sizeof(format);
        fprintf(stderr, "[gate0b] tap-format begin\n");
        status = AudioObjectGetPropertyData(
            _tapID, &formatAddress, 0, NULL, &formatSize, &format);
        fprintf(stderr, "[gate0b] tap-format end code=%d bytes_per_frame=%u\n",
                status, format.mBytesPerFrame);
        if (status != noErr || format.mBytesPerFrame == 0) {
            set_message(@"Core Audio did not provide a readable tap format.", status);
            [self stopSystemAudioLocked];
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        atomic_store(&gateSystemFrames, 0);
        UInt32 bytesPerFrame = format.mBytesPerFrame;
        fprintf(stderr, "[gate0b] io-proc-create begin\n");
        status = AudioDeviceCreateIOProcIDWithBlock(
            &_systemIOProc,
            _aggregateDeviceID,
            NULL,
            ^(const AudioTimeStamp *now,
              const AudioBufferList *input,
              const AudioTimeStamp *inputTime,
              AudioBufferList *output,
              const AudioTimeStamp *outputTime) {
                (void)now;
                (void)inputTime;
                (void)output;
                (void)outputTime;
                uint64_t frames = 0;
                if (input != NULL) {
                    for (UInt32 index = 0; index < input->mNumberBuffers; index++) {
                        uint64_t bufferFrames =
                            input->mBuffers[index].mDataByteSize / bytesPerFrame;
                        frames = MAX(frames, bufferFrames);
                    }
                }
                atomic_fetch_add(&gateSystemFrames, frames);
            });
        fprintf(stderr, "[gate0b] io-proc-create end code=%d\n", status);
        if (status != noErr) {
            set_message(@"Core Audio refused to attach the tap stream callback.", status);
            [self stopSystemAudioLocked];
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        fprintf(stderr, "[gate0b] device-start begin\n");
        status = AudioDeviceStart(_aggregateDeviceID, _systemIOProc);
        fprintf(stderr, "[gate0b] device-start end code=%d\n", status);
        if (status != noErr) {
            if (status == (OSStatus)MACH_RCV_TIMED_OUT) {
                set_message(@"System audio permission was not granted before macOS timed out; allow it in Privacy & Security and restart the app.", status);
            } else {
                set_message(@"The Core Audio Tap stream failed to start.", status);
            }
            [self stopSystemAudioLocked];
            atomic_store(&gateSystemStarting, 0);
            return;
        }

        atomic_store(&gateSystemRunning, 1);
        atomic_store(&gateSystemStarting, 0);
        set_message(@"Core Audio Tap stream started; play a short sound to verify frames.", 0);
    });
}

- (void)stopSystemAudioLocked {
    if (_aggregateDeviceID != kAudioObjectUnknown && _systemIOProc != NULL) {
        AudioDeviceStop(_aggregateDeviceID, _systemIOProc);
        AudioDeviceDestroyIOProcID(_aggregateDeviceID, _systemIOProc);
        _systemIOProc = NULL;
    }
    if (_aggregateDeviceID != kAudioObjectUnknown) {
        AudioHardwareDestroyAggregateDevice(_aggregateDeviceID);
        _aggregateDeviceID = kAudioObjectUnknown;
    }
    if (_tapID != kAudioObjectUnknown) {
        AudioHardwareDestroyProcessTap(_tapID);
        _tapID = kAudioObjectUnknown;
    }
    atomic_store(&gateSystemRunning, 0);
}

- (void)stopSystemAudio {
    if (atomic_load(&gateSystemStarting) != 0) {
        set_message(@"The Core Audio Tap start is still waiting for macOS permission.", 0);
        return;
    }
    dispatch_async(self.systemQueue, ^{
        [self stopSystemAudioLocked];
        set_message(@"Core Audio Tap stream stopped and native objects were destroyed.", 0);
    });
}

- (void)captureScreen {
    if (!CGPreflightScreenCaptureAccess()) {
        BOOL allowed = CGRequestScreenCaptureAccess();
        atomic_store(&gateScreenAllowed, allowed);
        if (!allowed) {
            set_message(@"Screen access is not allowed; change it in Privacy & Security and restart if requested.", 0);
            return;
        }
    }

    atomic_store(&gateScreenAllowed, 1);
    set_message(@"Capturing this Gate 0B window with ScreenCaptureKit.", 0);
    [SCShareableContent
        getShareableContentExcludingDesktopWindows:YES
                               onScreenWindowsOnly:YES
                                  completionHandler:^(SCShareableContent *content, NSError *error) {
        if (content == nil || error != nil) {
            fprintf(stderr, "[gate0b] shareable content failed code=%ld\n",
                    (long)error.code);
            set_message(@"ScreenCaptureKit could not enumerate shareable content.",
                        (int32_t)error.code);
            return;
        }

        SCWindow *target = nil;
        pid_t processID = getpid();
        for (SCWindow *window in content.windows) {
            if (window.owningApplication.processID == processID && window.windowLayer == 0) {
                target = window;
                break;
            }
        }
        if (target == nil) {
            set_message(@"ScreenCaptureKit did not find this app window.", 0);
            return;
        }

        SCContentFilter *filter =
            [[SCContentFilter alloc] initWithDesktopIndependentWindow:target];
        SCStreamConfiguration *configuration = [SCStreamConfiguration new];
        CGFloat scale = NSScreen.mainScreen.backingScaleFactor;
        configuration.width = MAX(1, (size_t)llround(target.frame.size.width * scale));
        configuration.height = MAX(1, (size_t)llround(target.frame.size.height * scale));
        configuration.showsCursor = NO;
        [SCScreenshotManager
            captureImageWithFilter:filter
                     configuration:configuration
                 completionHandler:^(CGImageRef image, NSError *captureError) {
            if (image == NULL || captureError != nil) {
                fprintf(stderr, "[gate0b] screenshot failed code=%ld\n",
                        (long)captureError.code);
                set_message(@"ScreenCaptureKit failed to capture this app window.",
                            (int32_t)captureError.code);
                return;
            }
            size_t width = CGImageGetWidth(image);
            size_t height = CGImageGetHeight(image);
            atomic_store(&gateScreenshotWidth, (uint32_t)width);
            atomic_store(&gateScreenshotHeight, (uint32_t)height);
            fprintf(stderr, "[gate0b] screenshot dimensions width=%zu height=%zu\n",
                    width, height);
            set_message(@"ScreenCaptureKit returned an in-memory image; no file was saved.", 0);
        }];
    }];
}

- (void)shutdown {
    [self stopMicrophone];
    if (atomic_load(&gateSystemStarting) != 0) {
        set_message(@"Exiting while macOS permission is pending; private HAL objects are process-scoped.", 0);
        return;
    }
    dispatch_sync(self.systemQueue, ^{
        [self stopSystemAudioLocked];
    });
    set_message(@"Capture resources were released.", 0);
}

@end

static SuflyorGate0BCaptureController *gateController;

void suflyor_gate0b_initialize(void) {
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        gateMessageLock = [NSLock new];
        gateMessage = @"Ready. Permission prompts run only after a button click.";
        gateController = [SuflyorGate0BCaptureController new];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        [[NSNotificationCenter defaultCenter]
            addObserverForName:NSApplicationWillTerminateNotification
                        object:nil
                         queue:nil
                    usingBlock:^(NSNotification *notification) {
            (void)notification;
            [gateController shutdown];
        }];
    });
    [gateController refresh];
}

void suflyor_gate0b_refresh(void) {
    [gateController refresh];
}

void suflyor_gate0b_request_microphone(void) {
    [gateController requestMicrophone];
}

void suflyor_gate0b_start_microphone(void) {
    [gateController startMicrophone];
}

void suflyor_gate0b_stop_microphone(void) {
    [gateController stopMicrophone];
}

void suflyor_gate0b_start_system_audio(void) {
    [gateController startSystemAudio];
}

void suflyor_gate0b_stop_system_audio(void) {
    [gateController stopSystemAudio];
}

void suflyor_gate0b_capture_screen(void) {
    [gateController captureScreen];
}

void suflyor_gate0b_open_privacy_settings(uint32_t section) {
    (void)section;
    NSURL *url = [NSWorkspace.sharedWorkspace
        URLForApplicationWithBundleIdentifier:@"com.apple.systempreferences"];
    if (url == nil) {
        set_message(@"System Settings could not be located.", 0);
        return;
    }
    NSWorkspaceOpenConfiguration *configuration =
        [NSWorkspaceOpenConfiguration configuration];
    [NSWorkspace.sharedWorkspace
        openApplicationAtURL:url
               configuration:configuration
           completionHandler:^(NSRunningApplication *application, NSError *error) {
        (void)application;
        if (error != nil) {
            set_message(@"System Settings could not be opened.", (int32_t)error.code);
            return;
        }
        set_message(@"System Settings opened; choose Privacy & Security.", 0);
    }];
}

void suflyor_gate0b_snapshot(SuflyorGate0BSnapshot *snapshot) {
    if (snapshot == NULL) {
        return;
    }
    snapshot->mic_permission = atomic_load(&gateMicPermission);
    snapshot->mic_device_available = atomic_load(&gateMicDeviceAvailable);
    snapshot->mic_running = atomic_load(&gateMicRunning);
    snapshot->system_running = atomic_load(&gateSystemRunning);
    snapshot->screen_allowed = atomic_load(&gateScreenAllowed);
    snapshot->screenshot_width = atomic_load(&gateScreenshotWidth);
    snapshot->screenshot_height = atomic_load(&gateScreenshotHeight);
    snapshot->last_error = atomic_load(&gateLastError);
    snapshot->mic_frames = atomic_load(&gateMicFrames);
    snapshot->mic_peak_milli = atomic_load(&gateMicPeakMilli);
    snapshot->system_starting = atomic_load(&gateSystemStarting);
    snapshot->system_frames = atomic_load(&gateSystemFrames);
}

void suflyor_gate0b_copy_message(char *buffer, size_t capacity) {
    if (buffer == NULL || capacity == 0) {
        return;
    }
    [gateMessageLock lock];
    const char *message = gateMessage.UTF8String;
    if (message == NULL) {
        message = "";
    }
    snprintf(buffer, capacity, "%s", message);
    [gateMessageLock unlock];
}

void suflyor_gate0b_shutdown(void) {
    [gateController shutdown];
}
