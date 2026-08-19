#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#include <stddef.h>
#include <stdint.h>

int32_t suflyor_macos_clipboard_set_text(const void *bytes, size_t length) {
    if (length > 0 && bytes == NULL) {
        return 0;
    }
    // Bytes + explicit length: the C-string convenience APIs stop at the
    // first NUL byte, which would truncate a payload that embeds one.
    NSString *text = [[NSString alloc] initWithBytes:bytes
                                              length:length
                                            encoding:NSUTF8StringEncoding];
    if (text == nil) {
        return 0;
    }
    NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
    [pasteboard clearContents];
    return [pasteboard setString:text forType:NSPasteboardTypeString] ? 1 : 0;
}

size_t suflyor_macos_clipboard_read_text(void *bytes, size_t capacity) {
    NSString *text = [NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString];
    NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding];
    if (data == nil || data.length == 0) {
        return 0;
    }
    if (bytes != NULL && capacity >= data.length) {
        [data getBytes:bytes length:data.length];
    }
    return data.length;
}

void suflyor_macos_clipboard_clear(void) {
    [NSPasteboard.generalPasteboard clearContents];
}

int32_t suflyor_macos_copy_modifiers_released(void) {
    CGEventFlags flags = CGEventSourceFlagsState(kCGEventSourceStateCombinedSessionState);
    CGEventFlags modifiers = kCGEventFlagMaskShift | kCGEventFlagMaskAlternate |
                             kCGEventFlagMaskControl | kCGEventFlagMaskCommand;
    return (flags & modifiers) == 0 ? 1 : 0;
}

int32_t suflyor_macos_send_command_c(void) {
    if (!AXIsProcessTrusted()) {
        return 0;
    }

    CGEventRef down = CGEventCreateKeyboardEvent(NULL, 8, true);
    CGEventRef up = CGEventCreateKeyboardEvent(NULL, 8, false);
    if (down == NULL || up == NULL) {
        if (down != NULL) {
            CFRelease(down);
        }
        if (up != NULL) {
            CFRelease(up);
        }
        return 0;
    }

    CGEventSetFlags(down, kCGEventFlagMaskCommand);
    CGEventSetFlags(up, kCGEventFlagMaskCommand);
    CGEventPost(kCGHIDEventTap, down);
    CGEventPost(kCGHIDEventTap, up);
    CFRelease(down);
    CFRelease(up);
    return 1;
}
