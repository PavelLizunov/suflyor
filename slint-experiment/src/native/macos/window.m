#import <AppKit/AppKit.h>
#include <stdint.h>

static NSWindow *suflyor_window_for_view(void *raw_view) {
    NSView *view = (__bridge NSView *)raw_view;
    return view.window;
}

int32_t suflyor_macos_configure_floating_window(void *raw_view) {
    NSWindow *window = suflyor_window_for_view(raw_view);
    if (window == nil) {
        return 0;
    }

    [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    window.level = NSPopUpMenuWindowLevel;
    NSWindowCollectionBehavior behavior = NSWindowCollectionBehaviorCanJoinAllSpaces |
        NSWindowCollectionBehaviorStationary |
        NSWindowCollectionBehaviorIgnoresCycle;
    if (@available(macOS 26.0, *)) {
        // Public in the macOS 26 SDK. Keep older SDKs buildable while preserving
        // the value verified by the isolated prototype (0x40000).
        behavior |= (NSWindowCollectionBehavior)(1UL << 18);
    } else {
        behavior |= NSWindowCollectionBehaviorFullScreenAuxiliary;
    }
    window.collectionBehavior = behavior;
    window.opaque = NO;
    window.backgroundColor = [NSColor clearColor];
    window.hidesOnDeactivate = NO;
    window.movableByWindowBackground = NO;
    window.releasedWhenClosed = NO;
    window.ignoresMouseEvents = NO;
    return 1;
}

int32_t suflyor_macos_begin_window_drag(void *raw_view) {
    NSWindow *window = suflyor_window_for_view(raw_view);
    NSEvent *event = NSApp.currentEvent;
    if (window == nil || event == nil || event.type != NSEventTypeLeftMouseDown) {
        return 0;
    }
    [window performWindowDragWithEvent:event];
    return 1;
}

int32_t suflyor_macos_raise_window_key_front(void *raw_view) {
    NSWindow *window = suflyor_window_for_view(raw_view);
    if (window == nil) {
        return 0;
    }
    // This is an explicit user-triggered overlay presentation. The newer
    // `activate` call can leave accessory apps behind the current app on
    // macOS 14+, while this forceful path reliably transfers keyboard focus.
    [NSApp activateIgnoringOtherApps:YES];
    [window orderFrontRegardless];
    [window makeKeyAndOrderFront:nil];
    return 1;
}

int32_t suflyor_macos_center_window(void *raw_view) {
    NSWindow *window = suflyor_window_for_view(raw_view);
    if (window == nil) {
        return 0;
    }
    NSScreen *screen = [NSScreen mainScreen];
    if (screen != nil) {
        NSRect screenFrame = [screen visibleFrame];
        NSRect windowFrame = [window frame];
        CGFloat x = screenFrame.origin.x + (screenFrame.size.width - windowFrame.size.width) / 2.0;
        CGFloat y = screenFrame.origin.y + (screenFrame.size.height - windowFrame.size.height) / 2.0;
        [window setFrameOrigin:NSMakePoint(x, y)];
    }
    return 1;
}
