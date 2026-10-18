#ifndef PHASA_DASHBOARD_H
#define PHASA_DASHBOARD_H

#import <Cocoa/Cocoa.h>

#ifdef __cplusplus
extern "C" {
#endif

// Callbacks from Objective-C UI to Rust
typedef void (*PhasaPlaygroundCallback)(const char* input);
typedef void (*PhasaModeCallback)(int mode);
typedef void (*PhasaToggleCallback)(bool enabled);
typedef void (*PhasaActionCallback)(void);

void phasa_ui_init(int initial_mode, bool is_autostart, bool is_notify);
void phasa_ui_show_window(void);
void phasa_ui_hide_window(void);
bool phasa_ui_is_window_visible(void);
void phasa_ui_set_accessibility_status(bool granted);
void phasa_ui_set_playground_result(const char* result_text);
void phasa_ui_set_mode(int mode);
void phasa_ui_set_autostart(bool enabled);
void phasa_ui_run_app(void);
void phasa_ui_stop_app(void);

// Register Rust callbacks
void phasa_ui_register_callbacks(
    PhasaPlaygroundCallback playground_cb,
    PhasaModeCallback mode_cb,
    PhasaToggleCallback autostart_cb,
    PhasaToggleCallback notify_cb,
    PhasaActionCallback open_config_cb,
    PhasaActionCallback open_access_cb,
    PhasaActionCallback poll_timer_cb
);

#ifdef __cplusplus
}
#endif

#endif // PHASA_DASHBOARD_H
