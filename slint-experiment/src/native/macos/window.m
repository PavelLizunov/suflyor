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
    window.level = NSStatusWindowLevel;
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
