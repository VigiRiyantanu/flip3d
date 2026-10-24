// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::cell::RefCell;
use std::sync::atomic::Ordering;

use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HWND, LPARAM, LRESULT, LUID, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE,
    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteTextFormat, IDWriteTextLayout, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
    DWRITE_TEXT_METRICS, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dwm::{
    DwmFlush, DwmGetCompositionTimingInfo, DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWM_TIMING_INFO,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Dxgi::DXGI_PRESENT;
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW, GetMonitorInfoW, MonitorFromPoint, MonitorFromRect, MonitorFromWindow,
    PtInRect, DEVMODEW, ENUM_CURRENT_SETTINGS, HMONITOR, MONITORINFO, MONITORINFOEXW,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::{
    CreateWaitableTimerExW, SetWaitableTimer, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
    TIMER_ALL_ACCESS,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    IsWindowEnabled, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_SHIFT,
    MOD_WIN, VK_TAB,
};
use windows::Win32::UI::Shell::VirtualDesktopManager;
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCursorPos, GetForegroundWindow, GetLastActivePopup, GetWindowLongPtrW, GetWindowTextW,
    IsIconic, IsWindow, KillTimer, LoadCursorW, MsgWaitForMultipleObjects,
    MsgWaitForMultipleObjectsEx, PeekMessageW, PostMessageW, PostThreadMessageW, RegisterClassExW,
    RegisterWindowMessageW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, ShowWindowAsync, TranslateMessage, UnregisterClassW, GWL_EXSTYLE,
    HWND_TOPMOST, IDC_ARROW, MA_NOACTIVATE, MSG, MSGFLT_ALLOW, MWMO_INPUTAVAILABLE, PM_REMOVE,
    QS_ALLINPUT, SC_CLOSE, SC_RESTORE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE,
    WHEEL_DELTA, WM_DISPLAYCHANGE, WM_HOTKEY, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEACTIVATE,
    WM_QUIT, WM_SETTINGCHANGE, WM_SYSCOMMAND, WM_TIMER, WNDCLASSEXW, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::background::BackgroundManager;
use crate::capture::{poll_frames, start_capture, stop_capture, Item, PROXY_CLASS_NAME};
use crate::geometry::{
    now_seconds, positive_mod_f32, positive_mod_i64, quad_contains, Pose, Quad, Tween, D2D_POINT_2F,
};
use crate::gfx::{get_monitor_adapter_luid, GraphicsDeviceContext};
use crate::hooks::{is_key_down, send_dummy_key_press};
use crate::icons::IconManager;
use crate::layout::{
    build_draw_list, compute_layout_metrics, compute_panel_layout, has_black_floor, is_itab_style,
    is_panel_style, selection_cell, shows_app_name, CloseButton, DrawEntry, LayoutMetrics,
    PanelLayout, SLIDE_ANIMATION_SECONDS,
};
use crate::render::{draw_card, draw_floor, draw_reflections, floor_screen_y, get_accent_color};
use crate::settings::{
    AnimationStyle, BackgroundMode, LiveContent, MonitorMode, Settings, Shortcut, TitlePosition,
    VirtualDesktops,
};
use crate::shortcuts::{
    apply_shortcut_settings, end_switching, set_overlay_hwnd, HoldKey, G_HOLD_KEY,
    G_HOTKEY_MESSAGE, G_READY, G_SESSION_SHORTCUT, G_SWITCHING, G_TAKE_OVER, START_BACKWARDS,
    START_FROM_HOTKEY, START_SHORTCUT_SHIFT, WM_APP_CANCEL, WM_APP_COMMIT, WM_APP_MOVE_DEVICE,
    WM_APP_REFRESH_ICONS, WM_APP_SETTINGS, WM_APP_START, WM_APP_STEP, WM_APP_WHEEL, WM_HOOK_ENGAGE,
    WM_HOOK_UPDATE,
};
use crate::windows_list::{collect_windows, windows_shows_all_desktops, WindowFilter};

pub const OVERLAY_CLASS_NAME: &str = "WindhawkFlip3DSwitcherOverlay";
pub const OVERLAY_TITLE_IDLE: &str = "Flip 3D";
pub const OVERLAY_TITLE_READY: &str = "Flip 3D (ready)";
pub const HOTKEY_MESSAGE_NAME: &str = "WindhawkFlip3DSwitcher_Hotkey";

pub const RESTORE_TIMER_ID: usize = 1;
pub const RESTORE_CHECK_MS: u32 = 60;
pub const MAX_RESTORE_TRIES: i32 = 3;

pub const DEVICE_RETRY_TIMER_ID: usize = 2;
pub const DEVICE_RETRY_MS: u32 = 2000;

pub const MIN_CLOSE_BUTTON_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Pending,
    Open,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameKey {
    pub t: f32,
    pub slide: f32,
    pub scroll: f64,
    pub items: usize,
    pub close_requests: i32,
    pub hover_index: i32,
    pub hover_on_button: bool,
}

impl Default for FrameKey {
    fn default() -> Self {
        Self {
            t: -1.0,
            slide: -1.0,
            scroll: -1.0,
            items: 0,
            close_requests: -1,
            hover_index: -1,
            hover_on_button: false,
        }
    }
}

pub struct ClosingItem {
    pub item: Item,
    pub start: f64,
}

pub struct Switcher {
    pub settings: Settings,
    pub hwnd: HWND,
    pub gfx: Option<GraphicsDeviceContext>,
    pub icons: IconManager,
    pub background_mgr: BackgroundManager,

    pub borderless_requested: bool,
    pub frame_interval: f64,
    pub refresh_interval: f64,
    pub next_frame_at: f64,
    pub frame_timer: Option<HANDLE>,
    pub last_render_error: windows::core::HRESULT,

    pub restore_wnd: HWND,
    pub restore_tries: i32,

    pub state: State,
    pub items: Vec<Item>,
    pub closing_items: Vec<ClosingItem>,
    pub slide_start: f64,
    pub draw_list: Vec<DrawEntry>,
    pub monitor: RECT,
    pub dpi_scale: f32,
    pub show_at: f64,
    pub capturing: bool,
    pub capture_deadline: f64,
    pub last_frame: f64,
    pub alt_released_at: f64,
    pub open_tween: Tween,
    pub commit: bool,
    pub activated_early: bool,
    pub session_shortcut: Shortcut,
    pub filter: WindowFilter,
    pub selected_on_top_when_flat: bool,
    pub has_focus: bool,
    pub previous_foreground: HWND,
    pub target: i64,
    pub scroll: f64,
    pub velocity: f64,
    pub wheel_remainder: i32,
    pub hotkeys_registered: bool,
    pub title_index: i32,
    pub title_format: Option<IDWriteTextFormat>,
    pub title_layout: Option<IDWriteTextLayout>,
    pub hover_index: i32,
    pub hover_button: CloseButton,
    pub hover_on_button: bool,
    pub last_frame_key: FrameKey,
    pub redraw: bool,
    pub accent: D2D1_COLOR_F,

    pub metrics: LayoutMetrics,
    pub floor_light: Option<windows::Win32::Graphics::Direct2D::ID2D1RadialGradientBrush>,
    pub reflection_fade: Option<windows::Win32::Graphics::Direct2D::ID2D1LinearGradientBrush>,
    pub app_name_format: Option<IDWriteTextFormat>,
    pub app_name_layout: Option<IDWriteTextLayout>,

    pub panel_layout: PanelLayout,
    pub label_format: Option<IDWriteTextFormat>,
}

thread_local! {
    static SWITCHER_INSTANCE: RefCell<Option<*mut Switcher>> = const { RefCell::new(None) };
}

/// # Safety
/// Wrapper around Win32 DefWindowProcW for proxy window class.
pub unsafe extern "system" fn def_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// # Safety
/// Win32 window procedure callback for the main Flip 3D overlay window.
pub unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr_opt = SWITCHER_INSTANCE.with(|cell| *cell.borrow());
    if let Some(ptr) = ptr_opt {
        if !ptr.is_null() {
            return (*ptr).handle_message(hwnd, msg, wparam, lparam);
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

impl Default for Switcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Switcher {
    pub fn new() -> Self {
        Self {
            settings: Settings::load(),
            hwnd: HWND(std::ptr::null_mut()),
            gfx: None,
            icons: IconManager::new(),
            background_mgr: BackgroundManager::new(HWND(std::ptr::null_mut())),
            borderless_requested: false,
            frame_interval: 0.0,
            refresh_interval: 1.0 / 60.0,
            next_frame_at: 0.0,
            frame_timer: None,
            last_render_error: windows::core::HRESULT(0),
            restore_wnd: HWND(std::ptr::null_mut()),
            restore_tries: 0,
            state: State::Idle,
            items: Vec::new(),
            closing_items: Vec::new(),
            slide_start: 0.0,
            draw_list: Vec::new(),
            monitor: RECT::default(),
            dpi_scale: 1.0,
            show_at: 0.0,
            capturing: false,
            capture_deadline: 0.0,
            last_frame: 0.0,
            alt_released_at: 0.0,
            open_tween: Tween::default(),
            commit: false,
            activated_early: false,
            session_shortcut: Shortcut::AltTab,
            filter: WindowFilter::default(),
            selected_on_top_when_flat: false,
            has_focus: false,
            previous_foreground: HWND(std::ptr::null_mut()),
            target: 0,
            scroll: 0.0,
            velocity: 0.0,
            wheel_remainder: 0,
            hotkeys_registered: false,
            title_index: -1,
            title_format: None,
            title_layout: None,
            hover_index: -1,
            hover_button: CloseButton::default(),
            hover_on_button: false,
            last_frame_key: FrameKey::default(),
            redraw: true,
            accent: D2D1_COLOR_F {
                r: 0.3,
                g: 0.76,
                b: 1.0,
                a: 1.0,
            },
            metrics: LayoutMetrics::default(),
            floor_light: None,
            reflection_fade: None,
            app_name_format: None,
            app_name_layout: None,
            panel_layout: PanelLayout::default(),
            label_format: None,
        }
    }

    pub fn create_overlay(&mut self) -> bool {
        self.settings = Settings::load();
        G_TAKE_OVER.store(
            self.settings.alt_tab_style != AnimationStyle::Native,
            Ordering::SeqCst,
        );
        apply_shortcut_settings(&self.settings);

        let class_name_w: Vec<u16> = format!("{}\0", OVERLAY_CLASS_NAME).encode_utf16().collect();
        let proxy_class_w: Vec<u16> = format!("{}\0", PROXY_CLASS_NAME).encode_utf16().collect();
        let idle_title_w: Vec<u16> = format!("{}\0", OVERLAY_TITLE_IDLE).encode_utf16().collect();

        // SAFETY: Register window classes for overlay and proxy windows.
        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wnd_proc),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: PCWSTR::from_raw(class_name_w.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);

            let proxy_wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(def_window_proc),
                lpszClassName: PCWSTR::from_raw(proxy_class_w.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&proxy_wc);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
                PCWSTR::from_raw(class_name_w.as_ptr()),
                PCWSTR::from_raw(idle_title_w.as_ptr()),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            );

            let overlay_hwnd = match hwnd {
                Ok(h) if !h.0.is_null() => h,
                _ => {
                    log::error!("CreateWindowEx failed for overlay: {:?}", GetLastError());
                    return false;
                }
            };

            self.hwnd = overlay_hwnd;
            set_overlay_hwnd(overlay_hwnd);

            let hotkey_msg_name: Vec<u16> = format!("{}\0", HOTKEY_MESSAGE_NAME)
                .encode_utf16()
                .collect();
            let hotkey_msg = RegisterWindowMessageW(PCWSTR::from_raw(hotkey_msg_name.as_ptr()));
            G_HOTKEY_MESSAGE.store(hotkey_msg, Ordering::SeqCst);

            let _ = ChangeWindowMessageFilterEx(overlay_hwnd, hotkey_msg, MSGFLT_ALLOW, None);

            let mut disable_transitions = BOOL(1);
            let _ = DwmSetWindowAttribute(
                overlay_hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &mut disable_transitions as *mut _ as *mut _,
                std::mem::size_of::<BOOL>() as u32,
            );

            let corners = DWMWCP_DONOTROUND;
            let _ = DwmSetWindowAttribute(
                overlay_hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corners as *const _ as *const _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            );

            // Create backdrop window
            let empty_title: Vec<u16> = "\0".encode_utf16().collect();
            if let Ok(backdrop) = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
                PCWSTR::from_raw(proxy_class_w.as_ptr()),
                PCWSTR::from_raw(empty_title.as_ptr()),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            ) {
                let _ = DwmSetWindowAttribute(
                    backdrop,
                    DWMWA_TRANSITIONS_FORCEDISABLED,
                    &mut disable_transitions as *mut _ as *mut _,
                    std::mem::size_of::<BOOL>() as u32,
                );
                let _ = DwmSetWindowAttribute(
                    backdrop,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corners as *const _ as *const _,
                    std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                );
                self.background_mgr.backdrop = backdrop;
            }
        }

        true
    }

    pub fn activate(&mut self) {
        if !G_TAKE_OVER.load(Ordering::SeqCst) || self.gfx.is_some() {
            self.publish_state();
            return;
        }

        let primary_mon =
            unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if let Ok(gfx) = GraphicsDeviceContext::new(primary_mon, self.hwnd) {
            self.gfx = Some(gfx);
            unsafe {
                let _ = KillTimer(Some(self.hwnd), DEVICE_RETRY_TIMER_ID);
            }
        } else {
            self.release_device_resources();
            unsafe {
                let _ = SetTimer(
                    Some(self.hwnd),
                    DEVICE_RETRY_TIMER_ID,
                    DEVICE_RETRY_MS,
                    None,
                );
            }
            self.publish_state();
            return;
        }

        if !self.borderless_requested {
            self.borderless_requested = true;
            // WinRT GraphicsCaptureAccess::RequestAccessAsync(Borderless)
            #[cfg(windows)]
            {
                use windows::Graphics::Capture::GraphicsCaptureAccess;
                use windows::Graphics::Capture::GraphicsCaptureAccessKind;
                let _ = GraphicsCaptureAccess::RequestAccessAsync(
                    GraphicsCaptureAccessKind::Borderless,
                );
            }
        }

        G_READY.store(true, Ordering::SeqCst);
        self.publish_state();
    }

    pub fn deactivate(&mut self) {
        end_switching();
        self.hide_overlay();
        self.teardown();
        self.state = State::Idle;
        self.release_device_resources();
        self.icons.cache.clear();
        self.publish_state();
    }

    pub fn publish_state(&self) {
        if !self.hwnd.0.is_null() {
            let title = if G_READY.load(Ordering::SeqCst) && G_TAKE_OVER.load(Ordering::SeqCst) {
                OVERLAY_TITLE_READY
            } else {
                OVERLAY_TITLE_IDLE
            };
            let wide: Vec<u16> = format!("{}\0", title).encode_utf16().collect();
            // SAFETY: SetWindowTextW sets the overlay title.
            unsafe {
                let _ = SetWindowTextW(self.hwnd, PCWSTR::from_raw(wide.as_ptr()));
            }
        }
        self.update_hotkeys();
    }

    pub fn update_hotkeys(&self) {
        let wanted = !self.hwnd.0.is_null()
            && G_READY.load(Ordering::SeqCst)
            && G_TAKE_OVER.load(Ordering::SeqCst)
            && (crate::shortcuts::G_SHORTCUTS.load(Ordering::SeqCst)
                & (1 << (Shortcut::WinCtrlTab as usize)))
                != 0;

        for shift in 0..2 {
            // SAFETY: Register/Unregister hotkey for Win+Ctrl+Tab.
            unsafe {
                if !wanted {
                    let _ = UnregisterHotKey(Some(self.hwnd), shift);
                } else {
                    let mod_flags = MOD_WIN
                        | MOD_CONTROL
                        | if shift != 0 {
                            MOD_SHIFT
                        } else {
                            HOT_KEY_MODIFIERS::default()
                        };
                    let _ = RegisterHotKey(Some(self.hwnd), shift, mod_flags, VK_TAB.0 as u32);
                }
            }
        }
    }

    pub fn release_device_resources(&mut self) {
        G_READY.store(false, Ordering::SeqCst);
        self.title_layout = None;
        self.title_format = None;
        self.title_index = -1;
        self.app_name_layout = None;
        self.app_name_format = None;
        self.reflection_fade = None;
        self.floor_light = None;
        self.background_mgr.background = None;
        self.background_mgr.background_key.clear();

        for entry in self.icons.cache.values_mut() {
            entry.bitmap = None;
        }

        if let Some(gfx) = self.gfx.as_mut() {
            gfx.release_device_resources();
        }
        self.gfx = None;
    }

    pub fn recreate_device(&mut self, monitor: HMONITOR) {
        self.release_device_resources();
        if let Ok(gfx) = GraphicsDeviceContext::new(monitor, self.hwnd) {
            self.gfx = Some(gfx);
            G_READY.store(true, Ordering::SeqCst);
            unsafe {
                let _ = KillTimer(Some(self.hwnd), DEVICE_RETRY_TIMER_ID);
            }
        } else {
            self.release_device_resources();
            unsafe {
                let _ = SetTimer(
                    Some(self.hwnd),
                    DEVICE_RETRY_TIMER_ID,
                    DEVICE_RETRY_MS,
                    None,
                );
            }
        }
        self.publish_state();
    }

    pub fn move_device_to_monitor(&mut self) {
        if self.state != State::Idle || self.gfx.is_none() {
            return;
        }
        let monitor = unsafe { MonitorFromRect(&self.monitor, MONITOR_DEFAULTTOPRIMARY) };
        let gfx = self.gfx.as_ref().unwrap();
        if monitor == gfx.checked_monitor {
            return;
        }
        let mut adapter = LUID::default();
        if get_monitor_adapter_luid(monitor, &mut adapter)
            && (adapter.LowPart != gfx.adapter_luid.LowPart
                || adapter.HighPart != gfx.adapter_luid.HighPart)
        {
            log::info!("Moving to the monitor's GPU");
            self.recreate_device(monitor);
        }
    }

    pub fn handle_device_lost(&mut self) {
        log::warn!("Graphics device lost, recreating");
        end_switching();
        self.hide_overlay();
        self.teardown();
        self.state = State::Idle;
        let mon = unsafe { MonitorFromRect(&self.monitor, MONITOR_DEFAULTTOPRIMARY) };
        self.recreate_device(mon);
    }

    pub fn update_frame_rate(&mut self) {
        self.frame_interval = 0.0;
        self.next_frame_at = 0.0;
        let mut refresh = 0.0;

        // SAFETY: Query monitor refresh rate.
        unsafe {
            let mon = MonitorFromRect(&self.monitor, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            let mut mode = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                ..Default::default()
            };

            if GetMonitorInfoW(mon, &mut info as *mut _ as *mut _).as_bool()
                && EnumDisplaySettingsW(
                    PCWSTR::from_raw(info.szDevice.as_ptr()),
                    ENUM_CURRENT_SETTINGS,
                    &mut mode,
                )
                .as_bool()
                && mode.dmDisplayFrequency > 1
            {
                refresh = mode.dmDisplayFrequency as f64;
            } else {
                let mut timing = DWM_TIMING_INFO {
                    cbSize: std::mem::size_of::<DWM_TIMING_INFO>() as u32,
                    ..Default::default()
                };
                if DwmGetCompositionTimingInfo(HWND(std::ptr::null_mut()), &mut timing).is_ok()
                    && timing.rateRefresh.uiDenominator > 0
                {
                    refresh = timing.rateRefresh.uiNumerator as f64
                        / timing.rateRefresh.uiDenominator as f64;
                }
            }
        }

        if refresh <= 0.0 {
            return;
        }

        let limited = self.settings.max_fps > 0 && (self.settings.max_fps as f64) < refresh * 0.95;
        self.frame_interval = 1.0
            / (if limited {
                self.settings.max_fps as f64
            } else {
                refresh + self.settings.fps_above_screen as f64
            });
        self.refresh_interval = 1.0 / refresh;

        if self.frame_timer.is_none() {
            // SAFETY: Create high-resolution waitable timer.
            unsafe {
                let timer = CreateWaitableTimerExW(
                    None,
                    PCWSTR::null(),
                    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                    TIMER_ALL_ACCESS.0,
                );
                if let Ok(t) = timer {
                    self.frame_timer = Some(t);
                }
            }
        }
    }

    pub fn selected_index(&self) -> i32 {
        if self.items.is_empty() {
            return -1;
        }
        positive_mod_i64(self.target, self.items.len() as i32)
    }

    pub fn on_hotkey(&mut self, flags: usize) {
        if !G_READY.load(Ordering::SeqCst) || !G_TAKE_OVER.load(Ordering::SeqCst) {
            return;
        }
        let backwards = (flags & START_BACKWARDS) != 0;
        if G_SWITCHING.load(Ordering::SeqCst) {
            self.on_step(if backwards { -1 } else { 1 });
            return;
        }
        let shortcut_val = flags >> START_SHORTCUT_SHIFT;
        self.session_shortcut = Shortcut::from(shortcut_val as u8);
        G_SESSION_SHORTCUT.store(self.session_shortcut as u8, Ordering::SeqCst);
        G_HOLD_KEY.store(
            crate::shortcuts::hold_key_for(self.session_shortcut) as u8,
            Ordering::SeqCst,
        );

        if G_SWITCHING.swap(true, Ordering::SeqCst) {
            self.on_step(if backwards { -1 } else { 1 });
            return;
        }

        let hook_thread_id = crate::shortcuts::G_HOOK_THREAD_ID.load(Ordering::SeqCst);
        if hook_thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(hook_thread_id, WM_HOOK_ENGAGE, WPARAM(1), LPARAM(0));
            }
        }

        self.on_start(START_FROM_HOTKEY | if backwards { START_BACKWARDS } else { 0 });
    }

    pub fn on_start(&mut self, flags: usize) {
        if !G_READY.load(Ordering::SeqCst) {
            end_switching();
            return;
        }
        let backwards = (flags & START_BACKWARDS) != 0;
        let from_hotkey = (flags & START_FROM_HOTKEY) != 0;

        if self.state == State::Closing {
            self.finish_close();
        } else if self.state != State::Idle {
            self.hide_overlay();
            self.teardown();
            self.state = State::Idle;
        }

        self.previous_foreground = unsafe { GetForegroundWindow() };

        let monitor = unsafe {
            let mut cursor = POINT::default();
            if self.settings.monitor == MonitorMode::Cursor && GetCursorPos(&mut cursor).is_ok() {
                MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY)
            } else {
                MonitorFromWindow(self.previous_foreground, MONITOR_DEFAULTTOPRIMARY)
            }
        };

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() } {
            end_switching();
            return;
        }
        self.monitor = info.rcMonitor;

        self.stop_restore_check();

        if let Some(gfx) = &self.gfx {
            if gfx.device_removed() {
                self.recreate_device(monitor);
                if !G_READY.load(Ordering::SeqCst) {
                    end_switching();
                    return;
                }
            }
        }

        self.session_shortcut = Shortcut::from(G_SESSION_SHORTCUT.load(Ordering::SeqCst));
        self.settings.style = self.settings.shortcut_style[self.session_shortcut as usize];

        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_err() }
        {
            dpi_y = 96;
        }
        let new_scale = dpi_y as f32 / 96.0;
        if (self.dpi_scale - new_scale).abs() > 1e-4 {
            self.dpi_scale = new_scale;
            self.title_format = None;
            self.app_name_format = None;
        }

        self.collect_windows_list();
        if self.items.is_empty() {
            log::info!("Shortcut {:?}: no windows to show", self.session_shortcut);
            self.teardown();
            end_switching();
            return;
        }

        let count = self.items.len();
        self.target = if count > 1 {
            if backwards {
                count as i64 - 1
            } else {
                1
            }
        } else {
            0
        };
        self.scroll = self.target as f64;
        self.velocity = 0.0;
        self.wheel_remainder = 0;
        self.alt_released_at = 0.0;
        self.title_index = -1;
        self.hover_index = -1;
        self.commit = false;
        self.selected_on_top_when_flat = false;

        self.compute_layout();

        self.show_at = now_seconds() + self.settings.show_delay_ms as f64 / 1000.0;
        self.capturing = false;
        self.state = State::Pending;

        if from_hotkey {
            self.take_focus();
        }
    }

    pub fn take_focus(&mut self) {
        let width = (self.monitor.right - self.monitor.left) as u32;
        let height = (self.monitor.bottom - self.monitor.top) as u32;
        if let Some(gfx) = self.gfx.as_mut() {
            if gfx.ensure_swap_chain(width, height).is_err() {
                return;
            }
            unsafe {
                gfx.ctx.SetTarget(gfx.target_bitmap.as_ref().unwrap());
                gfx.ctx.BeginDraw();
                gfx.ctx.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }));
                let _ = gfx.ctx.EndDraw(None, None);
                gfx.ctx.SetTarget(None);
                let _ = gfx.swap_chain.as_ref().unwrap().Present(0, DXGI_PRESENT(0));
            }
        }

        unsafe {
            let ex_style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                self.hwnd,
                GWL_EXSTYLE,
                ex_style & !(WS_EX_NOACTIVATE.0 as isize),
            );
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                self.monitor.left,
                self.monitor.top,
                width as i32,
                height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            self.has_focus = SetForegroundWindow(self.hwnd).as_bool();
        }
    }

    pub fn hide_overlay(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
            let ex_style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                self.hwnd,
                GWL_EXSTYLE,
                ex_style | (WS_EX_NOACTIVATE.0 as isize),
            );
        }
        self.has_focus = false;
        self.background_mgr.hide_backdrop();
    }

    pub fn hold_released(&mut self, now: f64) -> bool {
        let held = match G_HOLD_KEY.load(Ordering::SeqCst) {
            0 => is_key_down(windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU.0 as i32),
            1 => {
                is_key_down(windows::Win32::UI::Input::KeyboardAndMouse::VK_LWIN.0 as i32)
                    || is_key_down(windows::Win32::UI::Input::KeyboardAndMouse::VK_RWIN.0 as i32)
            }
            _ => true,
        };

        if !G_SWITCHING.load(Ordering::SeqCst) || held {
            self.alt_released_at = 0.0;
            return false;
        }
        if self.alt_released_at == 0.0 {
            self.alt_released_at = now;
        }
        now - self.alt_released_at > 0.15
    }

    pub fn on_step(&mut self, step: i32) {
        if (self.state != State::Pending && self.state != State::Open) || self.items.len() < 2 {
            return;
        }
        self.target += step as i64;
        if self.state == State::Pending {
            self.scroll = self.target as f64;
        }
    }

    pub fn on_commit(&mut self, index: i32) {
        if self.state == State::Idle || self.state == State::Closing {
            return;
        }

        if index >= 0 && !self.items.len() == 0 {
            let count = self.items.len() as i32;
            let mut delta = index - self.selected_index();
            if delta > count / 2 {
                delta -= count;
            } else if delta < -count / 2 {
                delta += count;
            }
            self.target += delta as i64;
        }

        if self.state == State::Pending {
            let selected = self.selected_index();
            let target = if selected >= 0 {
                self.items[selected as usize].hwnd
            } else {
                HWND(std::ptr::null_mut())
            };
            if !target.0.is_null() {
                self.activate_window(target);
            }
            self.end_session();
            return;
        }

        self.begin_close(true);
    }

    pub fn on_cancel(&mut self) {
        if self.state == State::Pending {
            if self.has_focus && !self.previous_foreground.0.is_null() {
                self.activate_window(self.previous_foreground);
            }
            self.end_session();
        } else if self.state == State::Open {
            self.begin_close(false);
        }
    }

    pub fn on_click(&mut self, pt: POINT) {
        if self.state != State::Open {
            return;
        }
        let point = D2D_POINT_2F {
            x: pt.x as f32,
            y: pt.y as f32,
        };
        let mut button = CloseButton::default();
        let index = self.hit_test(point, &mut button);
        if index < 0 {
            let on_panel = is_panel_style(self.settings.style)
                && point.x >= self.panel_layout.panel.left
                && point.x < self.panel_layout.panel.right
                && point.y >= self.panel_layout.panel.top
                && point.y < self.panel_layout.panel.bottom;

            if G_HOLD_KEY.load(Ordering::SeqCst) == HoldKey::None as u8
                && !on_panel
                && end_switching()
            {
                self.on_cancel();
            }
            return;
        }
        if self.settings.close_button && button.contains(point) {
            self.close_item(index as usize);
            return;
        }
        end_switching();
        self.on_commit(index);
    }

    pub fn on_middle_click(&mut self, pt: POINT) {
        if self.state != State::Open || !self.settings.middle_click_close {
            return;
        }
        let mut button = CloseButton::default();
        let index = self.hit_test(
            D2D_POINT_2F {
                x: pt.x as f32,
                y: pt.y as f32,
            },
            &mut button,
        );
        if index >= 0 {
            self.close_item(index as usize);
        }
    }

    pub fn hit_test(&self, point: D2D_POINT_2F, button: &mut CloseButton) -> i32 {
        if is_panel_style(self.settings.style) {
            for (i, cell) in self.panel_layout.cells.iter().enumerate() {
                if point.x >= cell.left
                    && point.x < cell.right
                    && point.y >= cell.top
                    && point.y < cell.bottom
                {
                    *button = self.close_button_for_cell(cell);
                    return i as i32;
                }
            }
            return -1;
        }

        for entry in self.draw_list.iter().rev() {
            if entry.pose.opacity < 0.5 || !quad_contains(&entry.quad, point) {
                continue;
            }
            if entry.item_index < self.items.len() {
                *button = self.close_button_for_quad(&entry.quad);
                return entry.item_index as i32;
            }
        }
        -1
    }

    pub fn close_button_for_quad(&self, quad: &Quad) -> CloseButton {
        let length = |a: D2D_POINT_2F, b: D2D_POINT_2F| -> f32 {
            ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
        };
        let shortest = length(quad.p[0], quad.p[1]).min(length(quad.p[1], quad.p[2]));
        let radius_raw = (13.0 * self.dpi_scale).round().min(shortest * 0.12);
        let radius = if radius_raw < MIN_CLOSE_BUTTON_RADIUS {
            0.0
        } else {
            radius_raw
        };
        let mut button = CloseButton {
            radius,
            ..Default::default()
        };

        let corner = quad.p[1];
        let center_x = (quad.p[0].x + quad.p[1].x + quad.p[2].x + quad.p[3].x) / 4.0;
        let center_y = (quad.p[0].y + quad.p[1].y + quad.p[2].y + quad.p[3].y) / 4.0;
        let to_center = length(
            corner,
            D2D_POINT_2F {
                x: center_x,
                y: center_y,
            },
        )
        .max(1.0);
        let inset = (button.radius * 1.35 * 1.414).min(to_center);
        button.center = D2D_POINT_2F {
            x: corner.x + (center_x - corner.x) / to_center * inset,
            y: corner.y + (center_y - corner.y) / to_center * inset,
        };
        button
    }

    pub fn close_button_for_cell(&self, cell: &D2D_RECT_F) -> CloseButton {
        let radius_raw = (11.0 * self.dpi_scale)
            .round()
            .min((cell.right - cell.left).min(cell.bottom - cell.top) * 0.16);
        let radius = if radius_raw < MIN_CLOSE_BUTTON_RADIUS {
            0.0
        } else {
            radius_raw
        };
        let mut button = CloseButton {
            radius,
            ..Default::default()
        };
        let inset = (button.radius * 1.3).round();
        button.center = D2D_POINT_2F {
            x: cell.right - inset,
            y: cell.top + inset,
        };
        button
    }

    pub fn close_item(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        let item = &mut self.items[index];
        if item.close_requested_at > 0.0 {
            return;
        }
        item.close_requested_at = now_seconds();
        // SAFETY: Post SC_CLOSE to target window.
        unsafe {
            let ok = PostMessageW(
                Some(item.hwnd),
                WM_SYSCOMMAND,
                WPARAM(SC_CLOSE as usize),
                LPARAM(0),
            );
            if ok.is_err() {
                item.close_requested_at = 0.0;
            }
        }
    }

    pub fn remove_closed_items(&mut self, now: f64) {
        if !self.items.is_empty() {
            let selected = self.selected_index();
            self.scroll -= (self.target - selected as i64) as f64;
            self.target = selected as i64;
        }

        let mut removed = false;
        let mut i = 0;
        while i < self.items.len() {
            let item = &self.items[i];
            if item.close_requested_at == 0.0 {
                i += 1;
                continue;
            }

            let mut hidden = false;
            // SAFETY: Check if window still exists.
            let exists = unsafe { IsWindow(Some(item.hwnd)).as_bool() };
            if exists
                && crate::windows_list::is_switchable_window(item.hwnd, &self.filter, &mut hidden)
            {
                if now - item.close_requested_at > 3.0 {
                    self.items[i].close_requested_at = 0.0;
                }
                i += 1;
                continue;
            }

            if (i as i32) < self.selected_index() {
                self.target -= 1;
                self.scroll -= 1.0;
            }

            let mut removed_item = self.items.remove(i);
            stop_capture(&mut removed_item);
            if removed_item.has_last_pose {
                self.closing_items.push(ClosingItem {
                    item: removed_item,
                    start: now,
                });
            }
            removed = true;
        }

        if !removed {
            return;
        }

        for item in &mut self.items {
            item.slide_from = item.last_pose;
            item.slides = item.has_last_pose;
        }
        self.slide_start = now;

        self.draw_list.clear();
        self.hover_index = -1;
        self.title_index = -1;
        self.title_layout = None;
        self.compute_layout();

        if self.items.is_empty() {
            end_switching();
            self.begin_close(false);
        }
    }

    pub fn update_hover(&mut self) {
        self.hover_index = -1;
        self.hover_on_button = false;
        if !self.settings.close_button || self.state != State::Open {
            return;
        }
        let mut cursor = POINT::default();
        if !unsafe { GetCursorPos(&mut cursor).is_ok() }
            || !unsafe { PtInRect(&self.monitor, cursor).as_bool() }
        {
            return;
        }
        let point = D2D_POINT_2F {
            x: (cursor.x - self.monitor.left) as f32,
            y: (cursor.y - self.monitor.top) as f32,
        };
        let mut button = CloseButton::default();
        let hit = self.hit_test(point, &mut button);
        self.hover_index = hit;
        self.hover_button = button;
        self.hover_on_button = self.hover_index >= 0 && self.hover_button.contains(point);
    }

    pub fn draw_close_button(&self, gfx: &GraphicsDeviceContext, t: f32) {
        if self.hover_index < 0
            || self.hover_index >= self.items.len() as i32
            || self.items[self.hover_index as usize].close_requested_at > 0.0
            || t < 0.01
        {
            return;
        }
        let r = self.hover_button.radius;
        if r <= 0.0 {
            return;
        }

        let circle = D2D1_ELLIPSE {
            point: self.hover_button.center.into(),
            radiusX: r,
            radiusY: r,
        };

        // SAFETY: Direct2D drawing for close X button.
        unsafe {
            if self.hover_on_button {
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.77,
                    g: 0.17,
                    b: 0.11,
                    a: 0.95 * t,
                });
            } else {
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.12,
                    g: 0.12,
                    b: 0.13,
                    a: 0.88 * t,
                });
            }
            gfx.ctx.FillEllipse(&circle, &gfx.brush);

            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.22 * t,
            });
            gfx.ctx.DrawEllipse(&circle, &gfx.brush, 1.0, None);

            let arm = r * 0.36;
            let stroke = (r * 0.14).max(1.5);
            let c = self.hover_button.center;
            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: t,
            });
            gfx.ctx.DrawLine(
                D2D_POINT_2F {
                    x: c.x - arm,
                    y: c.y - arm,
                }
                .into(),
                D2D_POINT_2F {
                    x: c.x + arm,
                    y: c.y + arm,
                }
                .into(),
                &gfx.brush,
                stroke,
                None,
            );
            gfx.ctx.DrawLine(
                D2D_POINT_2F {
                    x: c.x - arm,
                    y: c.y + arm,
                }
                .into(),
                D2D_POINT_2F {
                    x: c.x + arm,
                    y: c.y - arm,
                }
                .into(),
                &gfx.brush,
                stroke,
                None,
            );
        }
    }

    pub fn collect_windows_list(&mut self) {
        self.filter = WindowFilter {
            desktops: None,
            other_desktops: self.settings.virtual_desktops == VirtualDesktops::All
                || (self.settings.virtual_desktops == VirtualDesktops::SameAsWindows
                    && windows_shows_all_desktops()),
            manager_hidden: self.settings.manager_hidden_windows,
        };

        if self.filter.other_desktops || self.filter.manager_hidden {
            // SAFETY: CoCreateInstance for VirtualDesktopManager.
            unsafe {
                if let Ok(vdm) = CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL) {
                    self.filter.desktops = Some(vdm);
                }
            }
        }

        let infos = collect_windows(&self.settings, &self.filter);
        self.items.clear();

        for info in infos {
            let mut rect = RECT::default();
            let is_min = unsafe { IsIconic(info.hwnd).as_bool() };
            if !crate::windows_list::get_window_bounds(info.hwnd, is_min, &mut rect) {
                continue;
            }

            let mut title_buf = [0u16; 256];
            let len = unsafe { GetWindowTextW(info.hwnd, &mut title_buf) };
            let title = if len > 0 {
                String::from_utf16_lossy(&title_buf[..len as usize])
            } else {
                String::new()
            };

            let mut item = Item::new(info.hwnd, title, rect, is_min, info.hidden);
            if let Some(gfx) = &self.gfx {
                item.icon = self.icons.get_icon_bitmap(
                    info.hwnd,
                    &gfx.ctx,
                    &gfx.wic_factory,
                    self.dpi_scale,
                );
            }
            self.items.push(item);
        }
    }

    pub fn start_captures(&mut self) {
        self.capturing = true;
        let gfx = match &self.gfx {
            Some(g) => g,
            None => return,
        };
        for item in &mut self.items {
            start_capture(
                item,
                &gfx.capture_device,
                &gfx.d3d_device,
                &gfx.ctx,
                self.settings.minimized_content,
            );
        }
    }

    pub fn first_frames_ready(&mut self) -> bool {
        let gfx = match &self.gfx {
            Some(g) => g,
            None => return false,
        };
        let mut ready = true;
        for item in &mut self.items {
            poll_frames(
                item,
                &gfx.d3d_device,
                &gfx.d3d_context,
                &gfx.ctx,
                &gfx.capture_device,
            );
            if item.frame_pool.is_some() && item.content.is_none() {
                ready = false;
            }
        }
        ready
    }

    pub fn teardown(&mut self) {
        for item in &mut self.items {
            stop_capture(item);
        }
        self.draw_list.clear();
        self.items.clear();
        self.closing_items.clear();
        self.title_layout = None;
        self.title_index = -1;
        self.capturing = false;
        self.filter = WindowFilter::default();
    }

    pub fn show_overlay(&mut self) {
        let width = (self.monitor.right - self.monitor.left) as u32;
        let height = (self.monitor.bottom - self.monitor.top) as u32;

        if let Some(gfx) = self.gfx.as_mut() {
            if gfx.ensure_swap_chain(width, height).is_err() {
                self.handle_device_lost();
                return;
            }
        }

        if self.settings.background == BackgroundMode::LiveDesktop
            && !has_black_floor(self.settings.style)
        {
            self.background_mgr.show_backdrop(self.monitor, self.hwnd);
        }

        if let Some(gfx) = &self.gfx {
            self.background_mgr.update_background(
                &self.settings,
                self.monitor,
                width,
                height,
                &gfx.ctx,
                &gfx.wic_factory,
            );
        }

        self.update_frame_rate();
        if !self.capturing {
            self.start_captures();
        }

        if let Some(gfx) = &self.gfx {
            for item in &mut self.items {
                if item.content.is_none() && item.placeholder.is_none() {
                    item.placeholder = crate::icons::create_placeholder(
                        &gfx.ctx,
                        &gfx.brush,
                        item.width(),
                        item.height(),
                        item.icon.as_ref(),
                    );
                }
            }
        }

        if shows_app_name(self.settings.style) {
            let hwnds: Vec<HWND> = self.items.iter().map(|i| i.hwnd).collect();
            self.icons.lookup_app_names(&hwnds);
        }

        self.accent = get_accent_color();
        self.redraw = true;

        let now = now_seconds();
        self.open_tween = Tween::new(
            0.0,
            1.0,
            now,
            self.settings.animation_duration_ms as f64 / 1000.0,
            false,
        );
        self.last_frame = now;
        self.state = State::Open;

        self.render_frame();
        if self.state != State::Open {
            return;
        }

        // SAFETY: Show topmost overlay window.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                self.monitor.left,
                self.monitor.top,
                width as i32,
                height as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn begin_close(&mut self, commit: bool) {
        let now = now_seconds();
        let current = self.open_tween.value(now);
        self.open_tween = Tween::new(
            current,
            0.0,
            now,
            self.settings.animation_duration_ms as f64 * 0.85 / 1000.0 * current as f64,
            true,
        );
        self.commit = commit;
        self.selected_on_top_when_flat = commit;
        self.state = State::Closing;

        self.activated_early = false;
        let target = self.commit_target();
        let selected = self.selected_index();
        let hidden = self.commit && selected >= 0 && self.items[selected as usize].hidden;

        if !target.0.is_null() && !hidden {
            // SAFETY: Check iconic and topmost before early activation.
            unsafe {
                let iconic = IsIconic(target).as_bool();
                let topmost =
                    (GetWindowLongPtrW(target, GWL_EXSTYLE) & WS_EX_TOPMOST.0 as isize) != 0;
                if !iconic && !topmost {
                    self.activate_window(target);
                    self.activated_early = true;
                }
            }
        }
    }

    pub fn commit_target(&self) -> HWND {
        if self.commit {
            let selected = self.selected_index();
            if selected >= 0 && (selected as usize) < self.items.len() {
                return self.items[selected as usize].hwnd;
            }
            return HWND(std::ptr::null_mut());
        }
        if self.has_focus {
            self.previous_foreground
        } else {
            HWND(std::ptr::null_mut())
        }
    }

    pub fn finish_close(&mut self) {
        let target = if self.activated_early {
            HWND(std::ptr::null_mut())
        } else {
            self.commit_target()
        };
        self.activated_early = false;
        if !target.0.is_null() {
            self.activate_window(target);
            // SAFETY: Give DWM a frame to bring window up.
            unsafe {
                let _ = DwmFlush();
            }
        }
        self.end_session();
    }

    pub fn end_session(&mut self) {
        self.hide_overlay();
        self.teardown();
        self.state = State::Idle;
        // SAFETY: Post move device and refresh icons to overlay window.
        unsafe {
            let _ = PostMessageW(Some(self.hwnd), WM_APP_MOVE_DEVICE, WPARAM(0), LPARAM(0));
            let _ = PostMessageW(Some(self.hwnd), WM_APP_REFRESH_ICONS, WPARAM(0), LPARAM(0));
        }
    }

    pub fn activate_window(&mut self, hwnd: HWND) {
        // SAFETY: Restore and activate window.
        unsafe {
            if !IsWindow(Some(hwnd)).as_bool() {
                return;
            }

            let mut popup = GetLastActivePopup(hwnd);
            if popup.0.is_null()
                || !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(popup).as_bool()
                || !IsWindowEnabled(popup).as_bool()
            {
                popup = hwnd;
            }

            send_dummy_key_press();
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindowAsync(hwnd, SW_RESTORE);
                self.restore_wnd = hwnd;
                self.restore_tries = 0;
                let _ = SetTimer(Some(self.hwnd), RESTORE_TIMER_ID, RESTORE_CHECK_MS, None);
            }

            if !SetForegroundWindow(popup).as_bool() {
                log::warn!("SetForegroundWindow failed for {:?}", popup);
            }
        }
    }

    pub fn stop_restore_check(&mut self) {
        if !self.restore_wnd.0.is_null() {
            unsafe {
                let _ = KillTimer(Some(self.hwnd), RESTORE_TIMER_ID);
            }
            self.restore_wnd = HWND(std::ptr::null_mut());
        }
    }

    pub fn check_restored(&mut self) {
        let hwnd = self.restore_wnd;
        unsafe {
            if hwnd.0.is_null()
                || !IsWindow(Some(hwnd)).as_bool()
                || !IsIconic(hwnd).as_bool()
                || self.restore_tries >= MAX_RESTORE_TRIES
            {
                self.stop_restore_check();
                return;
            }

            self.restore_tries += 1;
            send_dummy_key_press();
            if self.restore_tries == 1 {
                let _ = PostMessageW(
                    Some(hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(SC_RESTORE as usize),
                    LPARAM(0),
                );
            } else {
                let _ = ShowWindowAsync(hwnd, SW_RESTORE);
            }

            let mut popup = GetLastActivePopup(hwnd);
            if popup.0.is_null()
                || !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(popup).as_bool()
                || !IsWindowEnabled(popup).as_bool()
            {
                popup = hwnd;
            }
            let _ = SetForegroundWindow(popup);
        }
    }

    pub fn compute_layout(&mut self) {
        self.redraw = true;
        let width = (self.monitor.right - self.monitor.left) as f32;
        let height = (self.monitor.bottom - self.monitor.top) as f32;
        self.metrics = compute_layout_metrics(
            self.settings.style,
            &self.settings,
            width,
            height,
            self.items.len(),
        );

        if is_panel_style(self.settings.style) {
            let item_sizes: Vec<(f32, f32)> =
                self.items.iter().map(|i| (i.width(), i.height())).collect();
            self.panel_layout = compute_panel_layout(
                self.settings.style,
                &self.settings,
                &item_sizes,
                width,
                height,
                self.dpi_scale,
            );
        }
    }

    pub fn ensure_title_layout(
        &mut self,
        gfx: &GraphicsDeviceContext,
        index: usize,
        font_size: f32,
    ) -> bool {
        if self.title_format.is_none() {
            let font_name: Vec<u16> = "Segoe UI Variable Display\0".encode_utf16().collect();
            let empty_str: Vec<u16> = "\0".encode_utf16().collect();

            // SAFETY: Create DirectWrite text format for title with ellipsis trimming.
            let format_res = unsafe {
                gfx.dwrite_factory.CreateTextFormat(
                    PCWSTR::from_raw(font_name.as_ptr()),
                    None,
                    DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    font_size,
                    PCWSTR::from_raw(empty_str.as_ptr()),
                )
            };

            let format = match format_res {
                Ok(f) => f,
                Err(_) => {
                    // Win10 fallback: Segoe UI
                    let fallback_font: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
                    match unsafe {
                        gfx.dwrite_factory.CreateTextFormat(
                            PCWSTR::from_raw(fallback_font.as_ptr()),
                            None,
                            DWRITE_FONT_WEIGHT_SEMI_BOLD,
                            DWRITE_FONT_STYLE_NORMAL,
                            DWRITE_FONT_STRETCH_NORMAL,
                            font_size,
                            PCWSTR::from_raw(empty_str.as_ptr()),
                        )
                    } {
                        Ok(f) => f,
                        Err(_) => return false,
                    }
                }
            };

            unsafe {
                let _ = format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                let trimming = DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter: 0,
                    delimiterCount: 0,
                };
                if let Ok(ellipsis) = gfx.dwrite_factory.CreateEllipsisTrimmingSign(&format) {
                    let _ = format.SetTrimming(&trimming, Some(&ellipsis));
                }
            }

            self.title_format = Some(format);
            self.title_layout = None;
        }

        if index as i32 != self.title_index || self.title_layout.is_none() {
            self.title_layout = None;
            self.app_name_layout = None;
            self.title_index = index as i32;

            if index < self.items.len() {
                let title = &self.items[index].title;
                let title_w: Vec<u16> = format!("{}\0", title).encode_utf16().collect();
                // SAFETY: CreateTextLayout for active title.
                unsafe {
                    let layout = gfx
                        .dwrite_factory
                        .CreateTextLayout(
                            &title_w[..title.encode_utf16().count()],
                            self.title_format.as_ref().unwrap(),
                            self.metrics.width * 0.6,
                            font_size * 2.0,
                        )
                        .ok();
                    self.title_layout = layout;
                }
            }
        }

        self.title_layout.is_some()
    }

    pub fn ensure_app_name_layout(
        &mut self,
        gfx: &GraphicsDeviceContext,
        index: usize,
        font_size: f32,
    ) -> bool {
        if self.app_name_format.is_none() {
            let font_name: Vec<u16> = "Segoe UI Variable Text\0".encode_utf16().collect();
            let empty_str: Vec<u16> = "\0".encode_utf16().collect();
            // SAFETY: Create TextFormat for app name.
            let format_res = unsafe {
                gfx.dwrite_factory.CreateTextFormat(
                    PCWSTR::from_raw(font_name.as_ptr()),
                    None,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    (font_size * 0.8).round(),
                    PCWSTR::from_raw(empty_str.as_ptr()),
                )
            };
            if let Ok(f) = format_res {
                unsafe {
                    let _ = f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                }
                self.app_name_format = Some(f);
            }
        }

        if index < self.items.len() && self.app_name_layout.is_none() {
            let key = self.items[index].hwnd.0 as isize;
            let name = self.icons.app_names.get(&key).cloned().unwrap_or_default();
            let name_w: Vec<u16> = format!("{}\0", name).encode_utf16().collect();
            if let Some(fmt) = &self.app_name_format {
                // SAFETY: Create TextLayout for app name.
                let layout = unsafe {
                    gfx.dwrite_factory
                        .CreateTextLayout(
                            &name_w[..name.encode_utf16().count()],
                            fmt,
                            self.metrics.width * 0.6,
                            font_size * 2.0,
                        )
                        .ok()
                };
                self.app_name_layout = layout;
            }
        }
        self.app_name_layout.is_some()
    }

    pub fn ensure_label_format(&mut self, gfx: &GraphicsDeviceContext, classic: bool) {
        if self.label_format.is_none() {
            let font_name: Vec<u16> = if classic {
                "Tahoma\0".encode_utf16().collect()
            } else {
                "Segoe UI Variable Text\0".encode_utf16().collect()
            };
            let empty_str: Vec<u16> = "\0".encode_utf16().collect();
            let weight = if classic {
                DWRITE_FONT_WEIGHT_BOLD
            } else {
                DWRITE_FONT_WEIGHT_NORMAL
            };

            // SAFETY: Create TextFormat for panel labels.
            unsafe {
                let fmt_res = gfx.dwrite_factory.CreateTextFormat(
                    PCWSTR::from_raw(font_name.as_ptr()),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    self.panel_layout.label_font_size,
                    PCWSTR::from_raw(empty_str.as_ptr()),
                );
                if let Ok(f) = fmt_res {
                    let _ = f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                    let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                    self.label_format = Some(f);
                }
            }
        }
    }

    pub fn draw_title(&mut self, gfx: &GraphicsDeviceContext, t: f32) {
        if !self.settings.show_title || self.items.is_empty() {
            return;
        }

        let nearest = self.scroll.round();
        let settle = 1.0 - (((self.scroll - nearest).abs() as f32) * 3.0).clamp(0.0, 1.0);
        let opacity = t * t * settle;
        if opacity < 0.01 {
            return;
        }

        let index = positive_mod_i64(nearest as i64, self.items.len() as i32) as usize;
        let font_size = (21.0 * self.dpi_scale).round();
        if !self.ensure_title_layout(gfx, index, font_size) {
            return;
        }

        let mut metrics = DWRITE_TEXT_METRICS::default();
        let layout = self.title_layout.as_ref().unwrap();
        // SAFETY: Query text metrics for title layout.
        unsafe {
            let _ = layout.GetMetrics(&mut metrics);
        }

        let has_icon = self.items[index].icon.is_some();
        let icon_size = if has_icon {
            (font_size * 1.6).round()
        } else {
            0.0
        };
        let gap = if has_icon {
            (font_size * 0.6).round()
        } else {
            0.0
        };
        let total_width = icon_size + gap + metrics.width;

        let mut center_x = self.metrics.width / 2.0;
        let mut center_y = (self.metrics.height * 0.91).round();

        if self.settings.title_position == TitlePosition::BelowWindow {
            for candidate in &self.draw_list {
                if candidate.item_index == index {
                    let mut min_x = candidate.quad.p[0].x;
                    let mut max_x = min_x;
                    let mut bottom = candidate.quad.p[0].y;
                    for pt in &candidate.quad.p {
                        min_x = min_x.min(pt.x);
                        max_x = max_x.max(pt.x);
                        bottom = bottom.max(pt.y);
                    }
                    center_x = (min_x + max_x) / 2.0;
                    center_y = (bottom + icon_size / 2.0 + font_size)
                        .min(self.metrics.height - font_size * 1.5)
                        .round();
                    break;
                }
            }
        }

        let margin = font_size.round();
        let left = (center_x - total_width / 2.0)
            .clamp(
                margin,
                margin.max(self.metrics.width - total_width - margin),
            )
            .round();

        // SAFETY: Render title icon and text with drop shadow.
        unsafe {
            if let Some(ic) = &self.items[index].icon {
                let icon_rect = D2D_RECT_F {
                    left,
                    top: center_y - icon_size / 2.0,
                    right: left + icon_size,
                    bottom: center_y + icon_size / 2.0,
                };
                gfx.ctx.DrawBitmap(
                    ic,
                    Some(&icon_rect),
                    opacity,
                    D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                    None,
                    None,
                );
            }

            let origin = D2D_POINT_2F {
                x: left + icon_size + gap - metrics.left,
                y: center_y - metrics.height / 2.0 - metrics.top,
            };

            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.55 * opacity,
            });
            gfx.ctx.DrawTextLayout(
                D2D_POINT_2F {
                    x: origin.x,
                    y: origin.y + 2.0,
                }
                .into(),
                layout,
                &gfx.brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );

            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: opacity,
            });
            gfx.ctx.DrawTextLayout(
                origin.into(),
                layout,
                &gfx.brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
    }

    pub fn draw_caption(&mut self, gfx: &GraphicsDeviceContext, t: f32) {
        let count = self.items.len();
        let opacity = t * t;
        if count == 0 || opacity < 0.01 {
            return;
        }

        let font_size = (21.0 * self.dpi_scale).round();
        let floor_y_screen = floor_screen_y(
            self.metrics.height,
            self.metrics.floor_y,
            self.settings.window_size,
        );
        let bar_y = (floor_y_screen + self.metrics.height * 0.29)
            .min(self.metrics.height - font_size * 1.5)
            .round();

        let nearest = self.scroll.round();
        let settle = 1.0 - (((self.scroll - nearest).abs() as f32) * 3.0).clamp(0.0, 1.0);
        let text_opacity = opacity * settle;
        let index = positive_mod_i64(nearest as i64, count as i32) as usize;

        let app_name_enabled = shows_app_name(self.settings.style);
        let title_ready = if self.settings.show_title && text_opacity >= 0.01 {
            if self.ensure_title_layout(gfx, index, font_size) {
                if app_name_enabled {
                    self.ensure_app_name_layout(gfx, index, font_size);
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        // SAFETY: Render iTab scroll bar and caption.
        unsafe {
            if count > 1 {
                let bar_width = (self.metrics.width * 0.34).round();
                let bar_height = (10.0 * self.dpi_scale).round();
                let left = ((self.metrics.width - bar_width) / 2.0).round();
                let radius = bar_height / 2.0;

                let track = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left,
                        top: bar_y - radius,
                        right: left + bar_width,
                        bottom: bar_y + radius,
                    },
                    radiusX: radius,
                    radiusY: radius,
                };

                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.08 * opacity,
                });
                gfx.ctx.FillRoundedRectangle(&track, &gfx.brush);

                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.28 * opacity,
                });
                gfx.ctx
                    .DrawRoundedRectangle(&track, &gfx.brush, self.dpi_scale.max(1.0), None);

                let inset = (2.0 * self.dpi_scale).round();
                let room = bar_width - inset * 2.0;
                let thumb_width = (room / count as f32).max(bar_height * 3.0);
                let position = (positive_mod_f32(self.scroll as f32, count as f32)
                    / (count - 1) as f32)
                    .min(1.0);
                let thumb_left = left + inset + (room - thumb_width) * position;

                let thumb = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: thumb_left,
                        top: bar_y - radius + inset,
                        right: thumb_left + thumb_width,
                        bottom: bar_y + radius - inset,
                    },
                    radiusX: radius - inset,
                    radiusY: radius - inset,
                };

                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.62 * opacity,
                });
                gfx.ctx.FillRoundedRectangle(&thumb, &gfx.brush);
            }

            if !title_ready {
                return;
            }

            let line_gap = (font_size * 1.35).round();
            let title_y =
                bar_y - (font_size * 1.6).round() - if app_name_enabled { line_gap } else { 0.0 };

            let draw_centered = |ctx: &ID2D1DeviceContext,
                                 brush: &ID2D1SolidColorBrush,
                                 layout: &IDWriteTextLayout,
                                 center_y: f32,
                                 mut color: D2D1_COLOR_F| {
                let mut m = DWRITE_TEXT_METRICS::default();
                let _ = layout.GetMetrics(&mut m);
                let origin = D2D_POINT_2F {
                    x: (self.metrics.width / 2.0 - m.width / 2.0 - m.left).round(),
                    y: (center_y - m.height / 2.0 - m.top).round(),
                };
                brush.SetColor(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.55 * text_opacity,
                });
                ctx.DrawTextLayout(
                    D2D_POINT_2F {
                        x: origin.x,
                        y: origin.y + 2.0,
                    }
                    .into(),
                    layout,
                    brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                );
                color.a *= text_opacity;
                brush.SetColor(&color);
                ctx.DrawTextLayout(origin.into(), layout, brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
            };

            if let Some(t_layout) = &self.title_layout {
                draw_centered(
                    &gfx.ctx,
                    &gfx.brush,
                    t_layout,
                    title_y,
                    D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                );
            }

            if app_name_enabled {
                if let Some(a_layout) = &self.app_name_layout {
                    draw_centered(
                        &gfx.ctx,
                        &gfx.brush,
                        a_layout,
                        title_y + line_gap,
                        D2D1_COLOR_F {
                            r: 0.72,
                            g: 0.72,
                            b: 0.74,
                            a: 1.0,
                        },
                    );
                }
            }
        }
    }

    pub fn draw_panel_back(&self, gfx: &GraphicsDeviceContext, t: f32) {
        if self.panel_layout.cells.is_empty() {
            return;
        }

        // SAFETY: Render panel background and selection highlight.
        unsafe {
            if self.settings.style == AnimationStyle::Classic {
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.925,
                    g: 0.914,
                    b: 0.847,
                    a: t,
                });
                gfx.ctx.FillRectangle(&self.panel_layout.panel, &gfx.brush);
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.24,
                    b: 0.65,
                    a: t,
                });
                gfx.ctx
                    .DrawRectangle(&self.panel_layout.panel, &gfx.brush, 2.0, None);
                return;
            }

            let radius = if self.settings.style == AnimationStyle::Thumbnails {
                0.0
            } else {
                (self.metrics.height / 135.0).round()
            };

            let rounded_panel = D2D1_ROUNDED_RECT {
                rect: self.panel_layout.panel,
                radiusX: radius,
                radiusY: radius,
            };

            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 0.11,
                g: 0.11,
                b: 0.12,
                a: 0.82 * t,
            });
            gfx.ctx.FillRoundedRectangle(&rounded_panel, &gfx.brush);

            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.08 * t,
            });
            gfx.ctx
                .DrawRoundedRectangle(&rounded_panel, &gfx.brush, 1.0, None);

            let (cell, opacity) = selection_cell(&self.panel_layout.cells, self.scroll);
            let rounded_cell = D2D1_ROUNDED_RECT {
                rect: cell,
                radiusX: radius,
                radiusY: radius,
            };
            gfx.brush.SetColor(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.07 * t * opacity,
            });
            gfx.ctx.FillRoundedRectangle(&rounded_cell, &gfx.brush);
        }
    }

    pub fn draw_panel_front(&mut self, gfx: &GraphicsDeviceContext, t: f32) {
        if self.panel_layout.cells.is_empty() || self.items.is_empty() {
            return;
        }
        let classic = self.settings.style == AnimationStyle::Classic;
        let (cell, opacity) = selection_cell(&self.panel_layout.cells, self.scroll);

        self.ensure_label_format(gfx, classic);
        let label_format = self.label_format.clone();

        // SAFETY: Render panel border, icons, labels, classic caption box.
        unsafe {
            if classic {
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.19,
                    g: 0.42,
                    b: 0.77,
                    a: 0.25 * t * opacity,
                });
                gfx.ctx.FillRectangle(&cell, &gfx.brush);
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.19,
                    g: 0.42,
                    b: 0.77,
                    a: t * opacity,
                });
                gfx.ctx.DrawRectangle(&cell, &gfx.brush, 2.0, None);
            } else {
                let radius = if self.settings.style == AnimationStyle::Thumbnails {
                    0.0
                } else {
                    (self.metrics.height / 135.0).round()
                };
                let border = (self.metrics.height / 360.0).round().max(2.0);
                let rounded = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: cell.left + border / 2.0,
                        top: cell.top + border / 2.0,
                        right: cell.right - border / 2.0,
                        bottom: cell.bottom - border / 2.0,
                    },
                    radiusX: radius,
                    radiusY: radius,
                };
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: self.accent.r,
                    g: self.accent.g,
                    b: self.accent.b,
                    a: t * opacity,
                });
                gfx.ctx
                    .DrawRoundedRectangle(&rounded, &gfx.brush, border, None);
            }

            let label_opacity = t * t;
            let centered = self.settings.style == AnimationStyle::IconRow;
            let label_color = D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.92 * label_opacity,
            };

            for i in 0..self.items.len() {
                let icon_rect = self.panel_layout.icons[i];
                let label_rect = self.panel_layout.labels[i];
                let item = &mut self.items[i];

                if let Some(ic) = &item.icon {
                    if icon_rect.right > icon_rect.left {
                        gfx.ctx.DrawBitmap(
                            ic,
                            Some(&icon_rect),
                            t,
                            D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                            None,
                            None,
                        );
                    }
                }
                if !classic && label_opacity > 0.01 {
                    if let Some(fmt) = &label_format {
                        Self::draw_item_label(gfx, fmt, item, label_rect, centered, label_color);
                    }
                }
            }

            if classic && label_opacity > 0.01 {
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.98,
                    g: 0.98,
                    b: 0.96,
                    a: t,
                });
                gfx.ctx
                    .FillRectangle(&self.panel_layout.caption, &gfx.brush);
                gfx.brush.SetColor(&D2D1_COLOR_F {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: t,
                });
                gfx.ctx
                    .DrawRectangle(&self.panel_layout.caption, &gfx.brush, 1.0, None);

                let selected =
                    positive_mod_i64(self.scroll.round() as i64, self.items.len() as i32) as usize;
                let pad = (self.panel_layout.label_font_size * 0.5).round();
                let rect = D2D_RECT_F {
                    left: self.panel_layout.caption.left + pad,
                    top: self.panel_layout.caption.top,
                    right: self.panel_layout.caption.right - pad,
                    bottom: self.panel_layout.caption.bottom,
                };
                if let Some(fmt) = &label_format {
                    Self::draw_item_label(
                        gfx,
                        fmt,
                        &mut self.items[selected],
                        rect,
                        false,
                        D2D1_COLOR_F {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: label_opacity,
                        },
                    );
                }
            }
        }
    }

    pub fn draw_item_label(
        gfx: &GraphicsDeviceContext,
        fmt: &IDWriteTextFormat,
        item: &mut Item,
        rect: D2D_RECT_F,
        centered: bool,
        color: D2D1_COLOR_F,
    ) {
        let width = (rect.right - rect.left).max(1.0);
        let height = (rect.bottom - rect.top).max(1.0);

        if item.label.is_none() || (item.label_width - width).abs() > 1.0 {
            item.label_width = width;
            let title_w: Vec<u16> = format!("{}\0", item.title).encode_utf16().collect();
            // SAFETY: CreateTextLayout for item.
            unsafe {
                let layout = gfx
                    .dwrite_factory
                    .CreateTextLayout(
                        &title_w[..item.title.encode_utf16().count()],
                        fmt,
                        width,
                        height,
                    )
                    .ok();
                item.label = layout;
            }
        }

        if let Some(lbl) = &item.label {
            // SAFETY: Draw label layout clipped.
            unsafe {
                let align = if centered {
                    DWRITE_TEXT_ALIGNMENT_CENTER
                } else {
                    DWRITE_TEXT_ALIGNMENT_LEADING
                };
                let _ = lbl.SetTextAlignment(align);
                let _ = lbl.SetMaxHeight(height);
                gfx.brush.SetColor(&color);
                gfx.ctx.DrawTextLayout(
                    D2D_POINT_2F {
                        x: rect.left,
                        y: rect.top,
                    }
                    .into(),
                    lbl,
                    &gfx.brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                );
            }
        }
    }

    pub fn render_frame(&mut self) {
        let now = now_seconds();
        if self.frame_interval > 0.0 {
            let slack = self.frame_interval.min(self.refresh_interval) / 2.0;
            if now < self.next_frame_at - slack {
                let due = -((self.next_frame_at - slack - now) * 1e7) as i64;
                if let Some(timer) = self.frame_timer {
                    // SAFETY: Wait on timer until frame due.
                    unsafe {
                        if SetWaitableTimer(timer, &due, 0, None, None, false).is_ok() {
                            MsgWaitForMultipleObjects(
                                Some(&[timer]),
                                false,
                                windows::Win32::System::Threading::INFINITE,
                                QS_ALLINPUT,
                            );
                        } else if DwmFlush().is_err() {
                            MsgWaitForMultipleObjects(None, false, 1, QS_ALLINPUT);
                        }
                    }
                } else {
                    unsafe {
                        if DwmFlush().is_err() {
                            MsgWaitForMultipleObjects(None, false, 1, QS_ALLINPUT);
                        }
                    }
                }
                return;
            }
            self.next_frame_at =
                self.next_frame_at.max(now - self.frame_interval) + self.frame_interval;
        }

        let dt = (now - self.last_frame).clamp(0.0, 0.05);
        self.last_frame = now;

        if self.state == State::Open && self.hold_released(now) && end_switching() {
            self.begin_close(true);
        }
        self.remove_closed_items(now);

        // Spring physics
        let omega = 15.0 * self.settings.flip_speed as f64;
        const STEP: f64 = 1.0 / 480.0;
        let mut remaining = dt;
        while remaining > 0.0 {
            let h = remaining.min(STEP);
            let accel =
                omega * omega * (self.target as f64 - self.scroll) - 2.0 * omega * self.velocity;
            self.velocity += accel * h;
            self.scroll += self.velocity * h;
            remaining -= STEP;
        }
        if (self.target as f64 - self.scroll).abs() < 1e-4 && self.velocity.abs() < 1e-3 {
            self.scroll = self.target as f64;
            self.velocity = 0.0;
        }

        let count = self.items.len();
        if count > 0 && self.target.abs() > count as i64 * 1000 {
            let shift = self.target - positive_mod_i64(self.target, count as i32) as i64;
            self.target -= shift;
            self.scroll -= shift as f64;
        }

        let t = self.open_tween.value(now);
        let selected = self.selected_index();

        let mut new_content = false;
        let mut close_requests = 0;
        let gfx = match self.gfx.as_ref() {
            Some(g) => g,
            None => return,
        };

        for i in 0..self.items.len() {
            let live = self.settings.live_content == LiveContent::All
                || (self.settings.live_content == LiveContent::Selected && i as i32 == selected);
            if live || self.items[i].content.is_none() {
                new_content |= poll_frames(
                    &mut self.items[i],
                    &gfx.d3d_device,
                    &gfx.d3d_context,
                    &gfx.ctx,
                    &gfx.capture_device,
                );
            }
            if self.items[i].close_requested_at > 0.0 {
                close_requests += 1;
            }
        }

        // Build draw list
        let mut item_poses: Vec<(Option<Pose>, Option<Pose>, bool)> = self
            .items
            .iter()
            .map(|it| (Some(it.last_pose), Some(it.slide_from), it.slides))
            .collect();

        let closing_tuples: Vec<(usize, Pose, f64, f32, f32)> = self
            .closing_items
            .iter()
            .enumerate()
            .map(|(idx, ci)| {
                (
                    idx,
                    ci.item.last_pose,
                    ci.start,
                    ci.item.width(),
                    ci.item.height(),
                )
            })
            .collect();

        let item_tuples: Vec<(f32, f32, RECT, bool)> = self
            .items
            .iter()
            .map(|it| (it.width(), it.height(), it.rect, it.minimized || it.hidden))
            .collect();

        self.draw_list = build_draw_list(
            self.settings.style,
            &self.settings,
            &self.metrics,
            &item_tuples,
            self.scroll,
            t,
            self.monitor,
            self.selected_on_top_when_flat,
            self.slide_start,
            &mut item_poses,
            &closing_tuples,
            &self.panel_layout.thumbnails,
        );

        for (i, p) in item_poses.into_iter().enumerate() {
            if i < self.items.len() {
                if let Some(lp) = p.0 {
                    self.items[i].last_pose = lp;
                    self.items[i].has_last_pose = true;
                }
            }
        }

        self.update_hover();

        let slide = ((now - self.slide_start) / SLIDE_ANIMATION_SECONDS).clamp(0.0, 1.0) as f32;
        let key = FrameKey {
            t,
            slide,
            scroll: self.scroll,
            items: self.items.len(),
            close_requests,
            hover_index: self.hover_index,
            hover_on_button: self.hover_on_button,
        };

        if !self.redraw
            && !new_content
            && self.closing_items.is_empty()
            && key == self.last_frame_key
        {
            // SAFETY: Wait for DWM flush to maintain presentation pace.
            unsafe {
                if DwmFlush().is_err() {
                    MsgWaitForMultipleObjects(None, false, 1, QS_ALLINPUT);
                }
            }
            if self.state == State::Closing && self.open_tween.done(now) {
                self.finish_close();
            }
            return;
        }

        self.redraw = false;
        self.last_frame_key = key;

        let gfx = match self.gfx.take() {
            Some(g) => g,
            None => return,
        };

        // SAFETY: Render full frame.
        let present_res = unsafe {
            gfx.ctx.SetTarget(gfx.target_bitmap.as_ref().unwrap());
            gfx.ctx.BeginDraw();
            gfx.ctx.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            let screen = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: self.metrics.width,
                bottom: self.metrics.height,
            };

            let cover = 1.0 - (1.0 - t).powi(3);
            if has_black_floor(self.settings.style) {
                draw_floor(
                    &gfx.ctx,
                    &gfx.brush,
                    self.floor_light.as_ref(),
                    self.metrics.width,
                    self.metrics.height,
                    self.metrics.floor_y,
                    cover,
                );
            } else {
                if let Some(bg) = &self.background_mgr.background {
                    gfx.ctx.DrawBitmap(
                        bg,
                        Some(&screen),
                        cover,
                        D2D1_INTERPOLATION_MODE_LINEAR,
                        None,
                        None,
                    );
                }
                if !self.background_mgr.desktop_thumbnails.is_empty() {
                    self.background_mgr.set_backdrop_opacity(cover);
                }
                if self.settings.dim_opacity > 0.0 {
                    gfx.brush.SetColor(&D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: self.settings.dim_opacity * t,
                    });
                    gfx.ctx.FillRectangle(&screen, &gfx.brush);
                }
            }

            let panel = is_panel_style(self.settings.style);
            let itab = is_itab_style(self.settings.style);

            if panel {
                self.draw_panel_back(&gfx, t);
            }
            if itab {
                draw_reflections(
                    &gfx.ctx,
                    &self.draw_list,
                    &mut self.items,
                    self.reflection_fade.as_ref(),
                    self.metrics.width,
                    self.metrics.height,
                    self.metrics.floor_y,
                    t,
                    self.metrics.focal,
                );
            }

            for entry in &self.draw_list {
                if entry.item_index < self.items.len() {
                    let item = &mut self.items[entry.item_index];
                    draw_card(
                        &gfx.ctx,
                        &gfx.brush,
                        &gfx.d2d_factory,
                        item,
                        entry.chain,
                        entry,
                        t,
                        self.settings.shadows,
                        itab,
                        self.metrics.focal,
                        self.metrics.height,
                    );
                }
            }

            if panel {
                self.draw_panel_front(&gfx, t);
            }
            self.draw_close_button(&gfx, t);
            if itab {
                self.draw_caption(&gfx, t);
            } else if !panel {
                self.draw_title(&gfx, t);
            }

            let hr = gfx.ctx.EndDraw(None, None);
            gfx.ctx.SetTarget(None);

            if hr.is_ok() {
                gfx.swap_chain
                    .as_ref()
                    .unwrap()
                    .Present(1, DXGI_PRESENT(0))
                    .ok()
            } else {
                hr
            }
        };

        self.gfx = Some(gfx);

        if let Err(e) = present_res {
            log::warn!("Present error: {:?}", e);
            self.handle_device_lost();
            return;
        }

        if self.state == State::Closing && self.open_tween.done(now) {
            self.finish_close();
        }
    }

    pub fn handle_message(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let hotkey_msg = G_HOTKEY_MESSAGE.load(Ordering::SeqCst);
        if hotkey_msg != 0 && msg == hotkey_msg {
            self.on_hotkey(wparam.0);
            return LRESULT(0);
        }

        match msg {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),

            WM_LBUTTONDOWN => {
                let x = (lparam.0 & 0xffff) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
                self.on_click(POINT { x, y });
                LRESULT(0)
            }

            WM_MBUTTONDOWN => {
                let x = (lparam.0 & 0xffff) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
                self.on_middle_click(POINT { x, y });
                LRESULT(0)
            }

            WM_APP_START => {
                self.on_start(wparam.0);
                LRESULT(0)
            }

            WM_HOTKEY => {
                if (crate::shortcuts::G_SHORTCUTS.load(Ordering::SeqCst)
                    & (1 << (Shortcut::WinCtrlTab as usize)))
                    != 0
                {
                    let backwards = ((lparam.0 & 0xffff) & MOD_SHIFT.0 as isize) != 0;
                    let flags = if backwards { START_BACKWARDS } else { 0 }
                        | ((Shortcut::WinCtrlTab as usize) << START_SHORTCUT_SHIFT);
                    self.on_hotkey(flags);
                }
                LRESULT(0)
            }

            WM_APP_STEP => {
                self.on_step(wparam.0 as isize as i32);
                LRESULT(0)
            }

            WM_APP_WHEEL => {
                self.wheel_remainder += wparam.0 as isize as i32;
                while self.wheel_remainder.abs() >= WHEEL_DELTA as i32 {
                    let dir = if self.wheel_remainder > 0 { 1 } else { -1 };
                    self.wheel_remainder -= dir * WHEEL_DELTA as i32;
                    self.on_step(-dir);
                }
                LRESULT(0)
            }

            WM_APP_COMMIT => {
                self.on_commit(-1);
                LRESULT(0)
            }

            WM_APP_CANCEL => {
                self.on_cancel();
                LRESULT(0)
            }

            WM_APP_REFRESH_ICONS => {
                if self.state == State::Idle {
                    if let Some(gfx) = &self.gfx {
                        self.icons
                            .refresh_icons(&gfx.ctx, &gfx.wic_factory, self.dpi_scale);
                    }
                }
                LRESULT(0)
            }

            WM_APP_MOVE_DEVICE => {
                self.move_device_to_monitor();
                LRESULT(0)
            }

            WM_APP_SETTINGS => {
                self.settings = Settings::load();
                if self.state != State::Idle {
                    self.settings.style =
                        self.settings.shortcut_style[self.session_shortcut as usize];
                }
                G_TAKE_OVER.store(
                    self.settings.alt_tab_style != AnimationStyle::Native,
                    Ordering::SeqCst,
                );
                apply_shortcut_settings(&self.settings);

                let hook_thread_id = crate::shortcuts::G_HOOK_THREAD_ID.load(Ordering::SeqCst);
                if hook_thread_id != 0 {
                    unsafe {
                        let _ = PostThreadMessageW(
                            hook_thread_id,
                            WM_HOOK_UPDATE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }

                self.background_mgr.background_key.clear();
                self.title_format = None;

                if !G_TAKE_OVER.load(Ordering::SeqCst) {
                    self.deactivate();
                } else if self.gfx.is_none() {
                    self.activate();
                } else {
                    if self.state != State::Idle {
                        self.compute_layout();
                    }
                    self.publish_state();
                }
                LRESULT(0)
            }

            WM_TIMER => {
                if wparam.0 == RESTORE_TIMER_ID {
                    self.check_restored();
                    return LRESULT(0);
                }
                if wparam.0 == DEVICE_RETRY_TIMER_ID {
                    if !G_TAKE_OVER.load(Ordering::SeqCst) || self.gfx.is_some() {
                        unsafe {
                            let _ = KillTimer(Some(self.hwnd), DEVICE_RETRY_TIMER_ID);
                        }
                    } else if self.state == State::Idle {
                        if self.borderless_requested {
                            let mon =
                                unsafe { MonitorFromRect(&self.monitor, MONITOR_DEFAULTTOPRIMARY) };
                            self.recreate_device(mon);
                        } else {
                            self.activate();
                        }
                    }
                    return LRESULT(0);
                }
                // SAFETY: DefWindowProcW for other timers.
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }

            WM_DISPLAYCHANGE => {
                self.background_mgr.background_key.clear();
                unsafe {
                    let _ = PostMessageW(Some(self.hwnd), WM_APP_MOVE_DEVICE, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }

            WM_SETTINGCHANGE => {
                self.background_mgr.background_key.clear();
                LRESULT(0)
            }

            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    pub fn run(&mut self) {
        // Register instance for WndProc
        SWITCHER_INSTANCE.with(|cell| {
            *cell.borrow_mut() = Some(self as *mut Switcher);
        });

        let mut msg = MSG::default();
        loop {
            if self.state == State::Idle || self.state == State::Pending {
                let timeout = if self.state == State::Pending {
                    let remaining = self.show_at - now_seconds();
                    if self.capturing {
                        4
                    } else {
                        ((remaining * 1000.0).ceil() as u32).clamp(0, 15)
                    }
                } else {
                    windows::Win32::System::Threading::INFINITE
                };

                let ev_opt = self.gfx.as_ref().and_then(|g| g.device_removed_event);
                let handles: &[HANDLE] = match &ev_opt {
                    Some(ev) => std::slice::from_ref(ev),
                    None => &[],
                };

                let wait_res = unsafe {
                    MsgWaitForMultipleObjectsEx(
                        Some(handles),
                        timeout,
                        QS_ALLINPUT,
                        MWMO_INPUTAVAILABLE,
                    )
                };

                if wait_res.0 == windows::Win32::Foundation::WAIT_OBJECT_0.0 && !handles.is_empty()
                {
                    if let Some(gfx) = &self.gfx {
                        if gfx.device_removed() {
                            if self.state == State::Idle {
                                log::info!("Graphics device lost, recreating");
                                let mon = unsafe {
                                    MonitorFromRect(&self.monitor, MONITOR_DEFAULTTOPRIMARY)
                                };
                                self.recreate_device(mon);
                            } else {
                                self.handle_device_lost();
                            }
                        }
                    }
                }
            }

            // Drain window message queue
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        return;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            if self.state == State::Pending {
                let now = now_seconds();
                if self.hold_released(now) && end_switching() {
                    self.on_commit(-1);
                } else if now >= self.show_at {
                    if !self.capturing {
                        self.start_captures();
                        self.capture_deadline = now + 0.15;
                    }
                    if self.first_frames_ready() || now >= self.capture_deadline {
                        self.show_overlay();
                    }
                }
            }

            if self.state == State::Open || self.state == State::Closing {
                self.render_frame();
            }
        }
    }

    pub fn shutdown(&mut self) {
        G_READY.store(false, Ordering::SeqCst);
        self.update_hotkeys();
        end_switching();
        if !self.hwnd.0.is_null() {
            self.hide_overlay();
        }
        self.teardown();
        self.release_device_resources();
        self.icons.cache.clear();

        unsafe {
            if !self.hwnd.0.is_null() {
                let _ = DestroyWindow(self.hwnd);
                self.hwnd = HWND(std::ptr::null_mut());
            }
            if !self.background_mgr.backdrop.0.is_null() {
                let _ = DestroyWindow(self.background_mgr.backdrop);
                self.background_mgr.backdrop = HWND(std::ptr::null_mut());
            }

            let class_name_w: Vec<u16> =
                format!("{}\0", OVERLAY_CLASS_NAME).encode_utf16().collect();
            let proxy_class_w: Vec<u16> =
                format!("{}\0", PROXY_CLASS_NAME).encode_utf16().collect();
            let _ = UnregisterClassW(PCWSTR::from_raw(class_name_w.as_ptr()), None);
            let _ = UnregisterClassW(PCWSTR::from_raw(proxy_class_w.as_ptr()), None);

            if let Some(timer) = self.frame_timer.take() {
                let _ = CloseHandle(timer);
            }
        }

        SWITCHER_INSTANCE.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}
