#import <Foundation/Foundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>
#import <CoreGraphics/CoreGraphics.h>
#import <AppKit/AppKit.h>
#import <Vision/Vision.h>

int32_t suflyor_macos_capture_display_bgra(
    uint32_t *out_width,
    uint32_t *out_height,
    uint8_t **out_bytes,
    size_t *out_len
) {
    if (!out_width || !out_height || !out_bytes || !out_len) {
        return -1;
    }
    *out_width = 0;
    *out_height = 0;
    *out_bytes = NULL;
    *out_len = 0;

    dispatch_semaphore_t sema = dispatch_semaphore_create(0);
    __block CGImageRef captured_image = NULL;
    __block NSError *captured_error = nil;

    [SCShareableContent getShareableContentExcludingDesktopWindows:NO
                                              onScreenWindowsOnly:YES
                                                completionHandler:^(SCShareableContent *content, NSError *error) {
        if (error != nil || content == nil || content.displays.count == 0) {
            captured_error = error;
            dispatch_semaphore_signal(sema);
            return;
        }

        SCDisplay *display = content.displays.firstObject;
        pid_t self_pid = getpid();
        NSMutableArray<SCWindow *> *own_windows = [NSMutableArray new];
        for (SCWindow *window in content.windows) {
            if (window.owningApplication.processID == self_pid) {
                [own_windows addObject:window];
            }
        }

        SCContentFilter *filter = [[SCContentFilter alloc] initWithDisplay:display excludingWindows:own_windows];
        SCStreamConfiguration *config = [SCStreamConfiguration new];
        config.width = display.width;
        config.height = display.height;
        config.showsCursor = NO;
        config.pixelFormat = kCVPixelFormatType_32BGRA;

        [SCScreenshotManager captureImageWithFilter:filter
                                      configuration:config
                                  completionHandler:^(CGImageRef image, NSError *cap_err) {
            if (image != NULL) {
                captured_image = CGImageRetain(image);
            }
            captured_error = cap_err;
            dispatch_semaphore_signal(sema);
        }];
    }];

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(sema, timeout) != 0) {
        return -2;
    }

    if (captured_image == NULL) {
        return captured_error ? (int32_t)captured_error.code : -3;
    }

    size_t w = CGImageGetWidth(captured_image);
    size_t h = CGImageGetHeight(captured_image);
    size_t bytes_per_row = w * 4;
    size_t total_bytes = bytes_per_row * h;

    uint8_t *buffer = (uint8_t *)malloc(total_bytes);
    if (!buffer) {
        CGImageRelease(captured_image);
        return -4;
    }

    CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
    CGContextRef context = CGBitmapContextCreate(
        buffer,
        w,
        h,
        8,
        bytes_per_row,
        color_space,
        kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little
    );

    if (!context) {
        free(buffer);
        CGColorSpaceRelease(color_space);
        CGImageRelease(captured_image);
        return -5;
    }

    CGRect rect = CGRectMake(0, 0, w, h);
    CGContextDrawImage(context, rect, captured_image);

    CGContextRelease(context);
    CGColorSpaceRelease(color_space);
    CGImageRelease(captured_image);

    *out_width = (uint32_t)w;
    *out_height = (uint32_t)h;
    *out_bytes = buffer;
    *out_len = total_bytes;
    return 0;
}

void suflyor_macos_free_screenshot_buffer(uint8_t *ptr) {
    if (ptr) {
        free(ptr);
    }
}

int32_t suflyor_macos_ocr_bgra(
    const uint8_t *bgra,
    uint32_t width,
    uint32_t height,
    char **out_text
) {
    if (!bgra || width == 0 || height == 0 || !out_text) {
        return -1;
    }
    *out_text = NULL;

    size_t bytes_per_row = width * 4;
    CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
    CGDataProviderRef provider = CGDataProviderCreateWithData(
        NULL,
        bgra,
        bytes_per_row * height,
        NULL
    );

    if (!provider) {
        CGColorSpaceRelease(color_space);
        return -2;
    }

    CGImageRef image = CGImageCreate(
        width,
        height,
        8,
        32,
        bytes_per_row,
        color_space,
        kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little,
        provider,
        NULL,
        NO,
        kCGRenderingIntentDefault
    );

    CGDataProviderRelease(provider);
    CGColorSpaceRelease(color_space);

    if (!image) {
        return -3;
    }

    dispatch_semaphore_t sema = dispatch_semaphore_create(0);
    __block NSString *recognized_text = nil;
    __block NSError *ocr_error = nil;

    VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] initWithCompletionHandler:^(VNRequest *req, NSError *err) {
        if (err != nil) {
            ocr_error = err;
            dispatch_semaphore_signal(sema);
            return;
        }
        NSMutableString *full_text = [NSMutableString new];
        for (VNRecognizedTextObservation *obs in req.results) {
            VNRecognizedText *top = [[obs topCandidates:1] firstObject];
            if (top) {
                if (full_text.length > 0) [full_text appendString:@"\n"];
                [full_text appendString:top.string];
            }
        }
        recognized_text = full_text;
        dispatch_semaphore_signal(sema);
    }];
    request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
    request.usesLanguageCorrection = YES;

    VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:image options:@{}];
    NSError *exec_err = nil;
    BOOL ok = [handler performRequests:@[request] error:&exec_err];
    CGImageRelease(image);

    if (!ok) {
        return exec_err ? (int32_t)exec_err.code : -4;
    }

    dispatch_time_t timeout = dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC);
    if (dispatch_semaphore_wait(sema, timeout) != 0) {
        return -5;
    }

    if (!recognized_text) {
        return ocr_error ? (int32_t)ocr_error.code : -6;
    }

    const char *utf8 = [recognized_text UTF8String];
    if (utf8) {
        *out_text = strdup(utf8);
    }
    return 0;
}

void suflyor_macos_free_string(char *ptr) {
    if (ptr) {
        free(ptr);
    }
}
