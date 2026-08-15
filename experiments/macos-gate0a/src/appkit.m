#import <AppKit/AppKit.h>
#include <stdio.h>

static NSStatusItem *gateStatusItem;
static NSWindow *gateOverlayWindow;

@interface SuflyorGate0AStatusController : NSObject
- (void)restoreOverlay:(id)sender;
- (void)quitGate:(id)sender;
@end

@implementation SuflyorGate0AStatusController
- (void)restoreOverlay:(id)sender {
    (void)sender;
    [NSApp activateIgnoringOtherApps:YES];
    [gateOverlayWindow orderFrontRegardless];
}

- (void)quitGate:(id)sender {
    (void)sender;
    [NSApp terminate:nil];
}
@end

static SuflyorGate0AStatusController *gateStatusController;

static NSWindow *window_for_view(void *rawView) {
    NSView *view = (__bridge NSView *)rawView;
    return view.window;
}

static NSWindow *configure_floating_window(void *rawView) {
    NSWindow *window = window_for_view(rawView);
    if (window == nil) {
        fprintf(stderr, "[gate0a] AppKit view has no window\n");
        return nil;
    }

    [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    window.level = NSStatusWindowLevel;
    NSWindowCollectionBehavior behavior = NSWindowCollectionBehaviorCanJoinAllSpaces |
        NSWindowCollectionBehaviorFullScreenAuxiliary |
        NSWindowCollectionBehaviorStationary |
        NSWindowCollectionBehaviorIgnoresCycle;
    if (@available(macOS 26.0, *)) {
        behavior |= NSWindowCollectionBehaviorCanJoinAllApplications;
    }
    window.collectionBehavior = behavior;
    window.opaque = NO;
    window.backgroundColor = [NSColor clearColor];
    window.hidesOnDeactivate = NO;
    window.movableByWindowBackground = NO;
    window.releasedWhenClosed = NO;
    window.ignoresMouseEvents = NO;

    fprintf(stderr,
            "[gate0a] configured floating window level=%ld behavior=0x%lx sharing=default\n",
            (long)window.level,
            (unsigned long)window.collectionBehavior);
    return window;
}

static void ensure_status_item(NSWindow *overlayWindow) {
    gateOverlayWindow = overlayWindow;
    if (gateStatusItem != nil) {
        return;
    }

    gateStatusController = [SuflyorGate0AStatusController new];
    gateStatusItem = [[NSStatusBar systemStatusBar]
        statusItemWithLength:NSVariableStatusItemLength];
    gateStatusItem.button.title = @"Suflyor";
    gateStatusItem.button.toolTip = @"Suflyor Gate 0A recovery";

    NSMenu *menu = [NSMenu new];
    NSMenuItem *restore = [[NSMenuItem alloc]
        initWithTitle:@"Restore Gate 0A overlay"
                action:@selector(restoreOverlay:)
         keyEquivalent:@""];
    restore.target = gateStatusController;
    [menu addItem:restore];
    [menu addItem:[NSMenuItem separatorItem]];

    NSMenuItem *quit = [[NSMenuItem alloc]
        initWithTitle:@"Quit Gate 0A"
                action:@selector(quitGate:)
         keyEquivalent:@"q"];
    quit.target = gateStatusController;
    [menu addItem:quit];
    gateStatusItem.menu = menu;
}

void suflyor_gate0a_configure_overlay(void *rawView) {
    NSWindow *window = configure_floating_window(rawView);
    if (window == nil) {
        return;
    }
    ensure_status_item(window);
}

void suflyor_gate0a_configure_tile(void *rawView) {
    (void)configure_floating_window(rawView);
}

void suflyor_gate0a_configure_settings(void *rawView) {
    NSWindow *window = window_for_view(rawView);
    if (window == nil) {
        fprintf(stderr, "[gate0a] settings AppKit view has no window\n");
        return;
    }

    window.level = NSNormalWindowLevel;
    window.collectionBehavior = NSWindowCollectionBehaviorManaged;
    [NSApp activateIgnoringOtherApps:YES];
    [window makeKeyAndOrderFront:nil];
    fprintf(stderr, "[gate0a] configured normal settings window\n");
}

void suflyor_gate0a_drag_window(void *rawView) {
    NSWindow *window = window_for_view(rawView);
    NSEvent *event = NSApp.currentEvent;
    if (window == nil || event == nil || event.type != NSEventTypeLeftMouseDown) {
        return;
    }
    [window performWindowDragWithEvent:event];
}

void suflyor_gate0a_log_displays(void) {
    NSArray<NSScreen *> *screens = NSScreen.screens;
    fprintf(stderr, "[gate0a] screens=%lu\n", (unsigned long)screens.count);
    for (NSUInteger index = 0; index < screens.count; index++) {
        NSScreen *screen = screens[index];
        NSRect frame = screen.frame;
        NSRect visible = screen.visibleFrame;
        fprintf(stderr,
                "[gate0a] screen=%lu scale=%.2f frame=%.0f,%.0f %.0fx%.0f visible=%.0f,%.0f %.0fx%.0f\n",
                (unsigned long)index,
                screen.backingScaleFactor,
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
                visible.origin.x,
                visible.origin.y,
                visible.size.width,
                visible.size.height);
    }
}
