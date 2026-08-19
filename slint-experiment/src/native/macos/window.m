#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>
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

int32_t suflyor_macos_get_window_rect(
    void *raw_view,
    int32_t *out_x,
    int32_t *out_y,
    int32_t *out_width,
    int32_t *out_height
) {
    NSWindow *window = suflyor_window_for_view(raw_view);
    if (window == nil || out_x == NULL || out_y == NULL || out_width == NULL || out_height == NULL) {
        return 0;
    }
    NSRect frame = window.frame;
    CGRect primary_bounds = CGDisplayBounds(CGMainDisplayID());
    *out_x = (int32_t)NSMinX(frame);
    *out_y = (int32_t)(CGRectGetMaxY(primary_bounds) - NSMaxY(frame));
    *out_width = (int32_t)NSWidth(frame);
    *out_height = (int32_t)NSHeight(frame);
    return *out_width > 0 && *out_height > 0;
}
