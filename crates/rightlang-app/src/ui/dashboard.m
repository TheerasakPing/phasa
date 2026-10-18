#import "dashboard.h"
#import <ApplicationServices/ApplicationServices.h>

static PhasaPlaygroundCallback g_playground_cb = NULL;
static PhasaModeCallback g_mode_cb = NULL;
static PhasaToggleCallback g_autostart_cb = NULL;
static PhasaToggleCallback g_notify_cb = NULL;
static PhasaActionCallback g_open_config_cb = NULL;
static PhasaActionCallback g_open_access_cb = NULL;
static PhasaActionCallback g_poll_timer_cb = NULL;

@interface PhasaWindowController : NSObject <NSWindowDelegate, NSTextFieldDelegate>
@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) NSTextField *statusBadge;
@property (nonatomic, strong) NSButton *grantAccessButton;
@property (nonatomic, strong) NSTextField *playgroundInput;
@property (nonatomic, strong) NSTextField *playgroundOutput;
@property (nonatomic, strong) NSSegmentedControl *modeControl;
@property (nonatomic, strong) NSButton *autostartCheckbox;
@property (nonatomic, strong) NSButton *notifyCheckbox;
@property (nonatomic, strong) NSTimer *pollTimer;
@end

static PhasaWindowController *g_controller = nil;

@implementation PhasaWindowController

- (void)setupUIWithMode:(int)initialMode autostart:(bool)isAutostart notify:(bool)isNotify {
    NSRect frame = NSMakeRect(0, 0, 560, 520);
    NSUInteger style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable;
    self.window = [[NSWindow alloc] initWithContentRect:frame
                                              styleMask:style
                                                backing:NSBackingStoreBuffered
                                                  defer:NO];
    self.window.title = @"Phasa (ภาษา) — ควบคุมและตั้งค่า";
    [self.window center];
    self.window.delegate = self;
    self.window.releasedWhenClosed = NO;

    NSView *contentView = [[NSView alloc] initWithFrame:frame];
    self.window.contentView = contentView;

    // 1. Header (Icon + Title + Subtitle)
    CGFloat y = 440;
    NSImageView *iconView = [[NSImageView alloc] initWithFrame:NSMakeRect(28, y - 4, 52, 52)];
    NSString *iconPath = [[NSBundle mainBundle] pathForResource:@"phasa" ofType:@"icns"];
    if (iconPath) {
        iconView.image = [[NSImage alloc] initWithContentsOfFile:iconPath];
    } else {
        iconView.image = [NSImage imageNamed:NSImageNameApplicationIcon];
    }
    [contentView addSubview:iconView];

    NSTextField *titleLabel = [NSTextField labelWithString:@"Phasa (ภาษา)"];
    titleLabel.frame = NSMakeRect(92, y + 18, 400, 28);
    titleLabel.font = [NSFont boldSystemFontOfSize:22];
    [contentView addSubview:titleLabel];

    NSTextField *subtitleLabel = [NSTextField labelWithString:@"สลับภาษาแป้นพิมพ์ ไทย ↔ อังกฤษ อัจฉริยะแบบไร้ดีเลย์ (Zero-latency)"];
    subtitleLabel.frame = NSMakeRect(92, y - 4, 440, 20);
    subtitleLabel.font = [NSFont systemFontOfSize:12];
    subtitleLabel.textColor = [NSColor secondaryLabelColor];
    [contentView addSubview:subtitleLabel];

    // Divider
    y -= 25;
    NSBox *divider1 = [[NSBox alloc] initWithFrame:NSMakeRect(28, y, 504, 1)];
    divider1.boxType = NSBoxSeparator;
    [contentView addSubview:divider1];

    // 2. Status Card (Accessibility)
    y -= 55;
    NSTextField *statusHeader = [NSTextField labelWithString:@"สถานะสิทธิ์การเข้าถึง (Accessibility Permission):"];
    statusHeader.frame = NSMakeRect(28, y + 24, 300, 18);
    statusHeader.font = [NSFont boldSystemFontOfSize:13];
    [contentView addSubview:statusHeader];

    self.statusBadge = [NSTextField labelWithString:@"🟢 พร้อมทำงาน (Granted)"];
    self.statusBadge.frame = NSMakeRect(28, y, 280, 22);
    self.statusBadge.font = [NSFont systemFontOfSize:13];
    self.statusBadge.textColor = [NSColor systemGreenColor];
    [contentView addSubview:self.statusBadge];

    self.grantAccessButton = [NSButton buttonWithTitle:@"เปิดการตั้งค่า System Settings"
                                                target:self
                                                action:@selector(onOpenAccessibility:)];
    self.grantAccessButton.frame = NSMakeRect(320, y - 2, 212, 26);
    self.grantAccessButton.bezelStyle = NSBezelStyleRounded;
    [contentView addSubview:self.grantAccessButton];

    // 3. Live Playground Box
    y -= 135;
    NSBox *playgroundBox = [[NSBox alloc] initWithFrame:NSMakeRect(28, y, 504, 125)];
    playgroundBox.title = @"ทดลองพิมพ์สด (Interactive Playground)";
    playgroundBox.titleFont = [NSFont boldSystemFontOfSize:12];
    [contentView addSubview:playgroundBox];

    NSTextField *guideLabel = [NSTextField labelWithString:@"พิมพ์คำที่ต้องการทดสอบ เช่น '8iy[' หรือ 'g]y'' หรือ 'hello':"];
    guideLabel.frame = NSMakeRect(16, 75, 470, 18);
    guideLabel.font = [NSFont systemFontOfSize:11];
    guideLabel.textColor = [NSColor secondaryLabelColor];
    [playgroundBox.contentView addSubview:guideLabel];

    self.playgroundInput = [[NSTextField alloc] initWithFrame:NSMakeRect(16, 46, 470, 24)];
    self.playgroundInput.placeholderString = @"พิมพ์ทดสอบที่นี่ (แล้วเคาะ Space หรือดูผลสด)...";
    self.playgroundInput.delegate = self;
    [playgroundBox.contentView addSubview:self.playgroundInput];

    self.playgroundOutput = [NSTextField labelWithString:@"ผลการแปลง: (รอรับข้อความทดสอบ)"];
    self.playgroundOutput.frame = NSMakeRect(16, 12, 470, 24);
    self.playgroundOutput.font = [NSFont boldSystemFontOfSize:13];
    self.playgroundOutput.textColor = [NSColor systemBlueColor];
    [playgroundBox.contentView addSubview:self.playgroundOutput];

    // 4. Settings Controls
    y -= 85;
    NSTextField *modeLabel = [NSTextField labelWithString:@"โหมดการทำงาน:"];
    modeLabel.frame = NSMakeRect(28, y + 24, 140, 20);
    modeLabel.font = [NSFont boldSystemFontOfSize:13];
    [contentView addSubview:modeLabel];

    self.modeControl = [NSSegmentedControl segmentedControlWithLabels:@[@"อัตโนมัติ (Auto)", @"เฉพาะที่เลือก (Manual)", @"ปิดชั่วคราว"]
                                                         trackingMode:NSSegmentSwitchTrackingSelectOne
                                                               target:self
                                                               action:@selector(onModeChanged:)];
    self.modeControl.frame = NSMakeRect(150, y + 22, 382, 26);
    self.modeControl.selectedSegment = initialMode;
    [contentView addSubview:self.modeControl];

    self.autostartCheckbox = [NSButton checkboxWithTitle:@"เปิดใช้งานอัตโนมัติเมื่อเปิดเครื่อง (Start at Login)"
                                                  target:self
                                                  action:@selector(onAutostartToggled:)];
    self.autostartCheckbox.frame = NSMakeRect(28, y - 5, 400, 22);
    self.autostartCheckbox.state = isAutostart ? NSControlStateValueOn : NSControlStateValueOff;
    [contentView addSubview:self.autostartCheckbox];

    self.notifyCheckbox = [NSButton checkboxWithTitle:@"แสดงการแจ้งเตือนเมื่อแปลงข้อความสำเร็จ"
                                               target:self
                                               action:@selector(onNotifyToggled:)];
    self.notifyCheckbox.frame = NSMakeRect(28, y - 30, 400, 22);
    self.notifyCheckbox.state = isNotify ? NSControlStateValueOn : NSControlStateValueOff;
    [contentView addSubview:self.notifyCheckbox];

    // 5. Footer Buttons
    y -= 85;
    NSBox *divider2 = [[NSBox alloc] initWithFrame:NSMakeRect(28, y + 35, 504, 1)];
    divider2.boxType = NSBoxSeparator;
    [contentView addSubview:divider2];

    NSTextField *statsLabel = [NSTextField labelWithString:@"📚 คลังคำศัพท์: ไทย 24,647 คำ | อังกฤษ 14,225 คำ | แรม: ~19 MB"];
    statsLabel.frame = NSMakeRect(28, y + 8, 360, 20);
    statsLabel.font = [NSFont systemFontOfSize:11];
    statsLabel.textColor = [NSColor secondaryLabelColor];
    [contentView addSubview:statsLabel];

    NSButton *openConfigBtn = [NSButton buttonWithTitle:@"โฟลเดอร์ตั้งค่า"
                                                 target:self
                                                 action:@selector(onOpenConfig:)];
    openConfigBtn.frame = NSMakeRect(320, y + 4, 100, 26);
    openConfigBtn.bezelStyle = NSBezelStyleRounded;
    [contentView addSubview:openConfigBtn];

    NSButton *hideWindowBtn = [NSButton buttonWithTitle:@"ซ่อนหน้าต่าง"
                                                 target:self
                                                 action:@selector(onHideWindow:)];
    hideWindowBtn.frame = NSMakeRect(425, y + 4, 107, 26);
    hideWindowBtn.bezelStyle = NSBezelStyleRounded;
    [contentView addSubview:hideWindowBtn];

    // Native periodic polling timer running on main runloop in NSRunLoopCommonModes
    self.pollTimer = [NSTimer scheduledTimerWithTimeInterval:0.25
                                                      target:self
                                                    selector:@selector(onPollTimer:)
                                                    userInfo:nil
                                                     repeats:YES];
    [[NSRunLoop mainRunLoop] addTimer:self.pollTimer forMode:NSRunLoopCommonModes];

    // Immediately re-check when the user switches focus back to Phasa (e.g. from System Settings)
    [[NSNotificationCenter defaultCenter] addObserver:self
                                             selector:@selector(onAppBecameActive:)
                                                 name:NSApplicationDidBecomeActiveNotification
                                               object:nil];
}

- (void)onPollTimer:(NSTimer *)timer {
    if (g_poll_timer_cb) {
        g_poll_timer_cb();
    }
}

- (void)onAppBecameActive:(NSNotification *)note {
    if (g_poll_timer_cb) {
        g_poll_timer_cb();
    }
}

- (void)dealloc {
    [[NSNotificationCenter defaultCenter] removeObserver:self];
    [self.pollTimer invalidate];
    self.pollTimer = nil;
}

#pragma mark - Actions

- (void)onOpenAccessibility:(id)sender {
    NSDictionary *options = @{(__bridge id)kAXTrustedCheckOptionPrompt: @YES};
    AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);

    if (g_open_access_cb) {
        g_open_access_cb();
    } else {
        [[NSWorkspace sharedWorkspace] openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"]];
    }
    if (g_poll_timer_cb) {
        g_poll_timer_cb();
    }
}

- (void)onOpenConfig:(id)sender {
    if (g_open_config_cb) {
        g_open_config_cb();
    }
}

- (void)onHideWindow:(id)sender {
    [self.window orderOut:nil];
}

- (void)onModeChanged:(id)sender {
    if (g_mode_cb) {
        g_mode_cb((int)self.modeControl.selectedSegment);
    }
}

- (void)onAutostartToggled:(id)sender {
    bool enabled = (self.autostartCheckbox.state == NSControlStateValueOn);
    if (g_autostart_cb) {
        g_autostart_cb(enabled);
    }
}

- (void)onNotifyToggled:(id)sender {
    bool enabled = (self.notifyCheckbox.state == NSControlStateValueOn);
    if (g_notify_cb) {
        g_notify_cb(enabled);
    }
}

#pragma mark - NSTextFieldDelegate

- (void)controlTextDidChange:(NSNotification *)obj {
    NSTextField *field = (NSTextField *)obj.object;
    if (field == self.playgroundInput) {
        NSString *text = field.stringValue;
        if (g_playground_cb) {
            g_playground_cb([text UTF8String]);
        }
    }
}

- (BOOL)windowShouldClose:(NSWindow *)sender {
    [sender orderOut:nil];
    return NO; // Hide instead of terminate
}

@end

#pragma mark - C API implementations

void phasa_ui_init(int initial_mode, bool is_autostart, bool is_notify) {
    if (![NSThread isMainThread]) {
        dispatch_sync(dispatch_get_main_queue(), ^{
            phasa_ui_init(initial_mode, is_autostart, is_notify);
        });
        return;
    }
    if (!g_controller) {
        [NSApplication sharedApplication];
        g_controller = [[PhasaWindowController alloc] init];
        [g_controller setupUIWithMode:initial_mode autostart:is_autostart notify:is_notify];
    }
}

void phasa_ui_show_window(void) {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_show_window();
        });
        return;
    }
    if (g_controller && g_controller.window) {
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        [g_controller.window makeKeyAndOrderFront:nil];
        [NSApp activateIgnoringOtherApps:YES];
        [g_controller.window makeFirstResponder:g_controller.playgroundInput];
    }
}

void phasa_ui_hide_window(void) {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_hide_window();
        });
        return;
    }
    if (g_controller && g_controller.window) {
        [g_controller.window orderOut:nil];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    }
}

bool phasa_ui_is_window_visible(void) {
    if (g_controller && g_controller.window) {
        return [g_controller.window isVisible];
    }
    return false;
}

void phasa_ui_set_accessibility_status(bool granted) {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_set_accessibility_status(granted);
        });
        return;
    }
    if (g_controller && g_controller.statusBadge) {
        if (granted) {
            g_controller.statusBadge.stringValue = @"🟢 สิทธิ์พร้อมทำงาน (Granted & Active)";
            g_controller.statusBadge.textColor = [NSColor systemGreenColor];
            g_controller.grantAccessButton.hidden = YES;
        } else {
            g_controller.statusBadge.stringValue = @"🔴 ยังไม่ได้รับสิทธิ์ (Waiting for Permission)";
            g_controller.statusBadge.textColor = [NSColor systemRedColor];
            g_controller.grantAccessButton.hidden = NO;
        }
    }
}

void phasa_ui_set_playground_result(const char* result_text) {
    if (!result_text) return;
    NSString *res = [NSString stringWithUTF8String:result_text];
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_set_playground_result(result_text);
        });
        return;
    }
    if (g_controller && g_controller.playgroundOutput) {
        g_controller.playgroundOutput.stringValue = [NSString stringWithFormat:@"ผลการแปลง: %@", res];
    }
}

void phasa_ui_set_mode(int mode) {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_set_mode(mode);
        });
        return;
    }
    if (g_controller && g_controller.modeControl) {
        g_controller.modeControl.selectedSegment = mode;
    }
}

void phasa_ui_set_autostart(bool enabled) {
    if (![NSThread isMainThread]) {
        dispatch_async(dispatch_get_main_queue(), ^{
            phasa_ui_set_autostart(enabled);
        });
        return;
    }
    if (g_controller && g_controller.autostartCheckbox) {
        g_controller.autostartCheckbox.state = enabled ? NSControlStateValueOn : NSControlStateValueOff;
    }
}

void phasa_ui_run_app(void) {
    [NSApp run];
}

void phasa_ui_stop_app(void) {
    [NSApp stop:nil];
    NSEvent *event = [NSEvent otherEventWithType:NSEventTypeApplicationDefined
                                        location:NSMakePoint(0, 0)
                                   modifierFlags:0
                                       timestamp:0
                                    windowNumber:0
                                         context:nil
                                         subtype:0
                                           data1:0
                                           data2:0];
    [NSApp postEvent:event atStart:YES];
}

void phasa_ui_register_callbacks(
    PhasaPlaygroundCallback playground_cb,
    PhasaModeCallback mode_cb,
    PhasaToggleCallback autostart_cb,
    PhasaToggleCallback notify_cb,
    PhasaActionCallback open_config_cb,
    PhasaActionCallback open_access_cb,
    PhasaActionCallback poll_timer_cb
) {
    g_playground_cb = playground_cb;
    g_mode_cb = mode_cb;
    g_autostart_cb = autostart_cb;
    g_notify_cb = notify_cb;
    g_open_config_cb = open_config_cb;
    g_open_access_cb = open_access_cb;
    g_poll_timer_cb = poll_timer_cb;
}
