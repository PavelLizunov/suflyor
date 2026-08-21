#import <AppKit/AppKit.h>
#include <stdbool.h>
#include <stdint.h>

extern int32_t suflyor_macos_configure_floating_window(void *raw_view);

typedef void (*SuflyorStatusQuitCallback)(void);
typedef void (*SuflyorStatusVisibilityCallback)(bool visible);

@interface SuflyorStatusController : NSObject <NSMenuDelegate>
- (void)toggleOverlay:(id)sender;
- (void)quitSuflyor:(id)sender;
@end

static NSStatusItem *suflyor_status_item;
static NSView *suflyor_status_overlay_view;
static SuflyorStatusController *suflyor_status_controller;
static SuflyorStatusQuitCallback suflyor_status_quit_callback;
static SuflyorStatusVisibilityCallback suflyor_status_visibility_callback;

@implementation SuflyorStatusController
- (void)toggleOverlay:(id)sender {
    (void)sender;
    NSWindow *window = suflyor_status_overlay_view.window;
    if (window == nil) {
        return;
    }

    if (window.isVisible) {
        [window orderOut:nil];
    } else {
        (void)suflyor_macos_configure_floating_window(
            (__bridge void *)suflyor_status_overlay_view);
        [NSApp activateIgnoringOtherApps:YES];
        [window orderFrontRegardless];
    }
    if (suflyor_status_visibility_callback != NULL) {
        suflyor_status_visibility_callback(window.isVisible ? true : false);
    }
}

- (void)menuNeedsUpdate:(NSMenu *)menu {
    NSWindow *window = suflyor_status_overlay_view.window;
    NSMenuItem *toggle = menu.itemArray.firstObject;
    if (toggle != nil) {
        toggle.title = window.isVisible ? @"Hide Suflyor" : @"Show Suflyor";
    }
}

- (void)quitSuflyor:(id)sender {
    (void)sender;
    if (suflyor_status_quit_callback != NULL) {
        suflyor_status_quit_callback();
    }
}
@end

int32_t suflyor_macos_status_install(void *raw_view,
                                     SuflyorStatusQuitCallback on_quit,
                                     SuflyorStatusVisibilityCallback on_visibility) {
    if (![NSThread isMainThread] || raw_view == NULL || on_quit == NULL ||
        on_visibility == NULL || suflyor_status_item != nil) {
        return 0;
    }

    NSView *view = (__bridge NSView *)raw_view;
    NSWindow *window = view.window;
    if (window == nil) {
        return 0;
    }

    suflyor_status_overlay_view = view;
    suflyor_status_quit_callback = on_quit;
    suflyor_status_visibility_callback = on_visibility;
    suflyor_status_controller = [SuflyorStatusController new];
    suflyor_status_item = [[NSStatusBar systemStatusBar]
        statusItemWithLength:NSVariableStatusItemLength];
    suflyor_status_item.button.title = @"Suflyor";
    suflyor_status_item.button.toolTip = @"Suflyor overlay";

    NSMenu *menu = [NSMenu new];
    menu.delegate = suflyor_status_controller;
    NSString *toggle_title = window.isVisible ? @"Hide Suflyor" : @"Show Suflyor";
    NSMenuItem *toggle = [[NSMenuItem alloc]
        initWithTitle:toggle_title
               action:@selector(toggleOverlay:)
        keyEquivalent:@""];
    toggle.target = suflyor_status_controller;
    [menu addItem:toggle];
    [menu addItem:[NSMenuItem separatorItem]];

    NSMenuItem *quit = [[NSMenuItem alloc]
        initWithTitle:@"Quit Suflyor"
               action:@selector(quitSuflyor:)
        keyEquivalent:@"q"];
    quit.target = suflyor_status_controller;
    [menu addItem:quit];
    suflyor_status_item.menu = menu;
    on_visibility(window.isVisible ? true : false);
    return 1;
}

void suflyor_macos_status_remove(void) {
    if (![NSThread isMainThread]) {
        return;
    }
    if (suflyor_status_item != nil) {
        suflyor_status_item.menu.delegate = nil;
        suflyor_status_item.menu = nil;
        [[NSStatusBar systemStatusBar] removeStatusItem:suflyor_status_item];
    }
    suflyor_status_item = nil;
    suflyor_status_overlay_view = nil;
    suflyor_status_controller = nil;
    suflyor_status_quit_callback = NULL;
    suflyor_status_visibility_callback = NULL;
}
