#import <AppKit/AppKit.h>
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
