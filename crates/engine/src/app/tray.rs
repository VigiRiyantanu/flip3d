// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::sync::atomic::Ordering;
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyIcon, DestroyMenu, GetCursorPos, LoadCursorW, LoadIconW, PostMessageW, PostQuitMessage,
    PostThreadMessageW, RegisterClassExW, RegisterWindowMessageW, SetForegroundWindow,
    TrackPopupMenuEx, HICON, HMENU, IDC_ARROW, IDI_APPLICATION, LR_DEFAULTCOLOR, MF_CHECKED,
    MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_RETURNCMD, WM_APP, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSEXW, WS_OVERLAPPED,
};

use crate::app::autostart::{get_autostart_mode, set_autostart_mode, AutostartMode};
use crate::app::elevation::{is_elevated, relaunch_elevated};
use crate::app::settings_ui::spawn_settings_window;
use crate::shortcuts::{
    get_overlay_hwnd, G_HOOK_THREAD_ID, G_TAKE_OVER, WM_APP_SETTINGS, WM_HOOK_UPDATE,
};

const TRAY_CLASS_NAME: &str = "WindhawkFlip3DTrayWindow";
const WM_TRAYICON: u32 = WM_APP + 100;
pub const WM_TRAY_BALLOON: u32 = WM_APP + 101;
const TRAY_ICON_ID: u32 = 1001;

const ID_SETTINGS: usize = 2001;
const ID_PAUSE: usize = 2002;
const ID_AUTOSTART_DISABLED: usize = 2003;
const ID_AUTOSTART_NORMAL: usize = 2004;
const ID_AUTOSTART_ELEVATED: usize = 2005;
const ID_RUN_ADMIN: usize = 2006;
const ID_ABOUT: usize = 2007;
const ID_EXIT: usize = 2008;

static TRAY_PNG: &[u8] = include_bytes!("../../tray.png");

#[allow(clippy::manual_dangling_ptr)]
fn load_app_icon() -> HICON {
    unsafe {
        // 1. Try loading icon from embedded PE resource (resource ID 1)
        if let Ok(hinstance) = GetModuleHandleW(None) {
            let hinst = windows::Win32::Foundation::HINSTANCE(hinstance.0);
            if let Ok(icon) = LoadIconW(Some(hinst), PCWSTR(1 as *const u16)) {
                if !icon.0.is_null() {
                    return icon;
                }
            }
        }

        // 2. Direct memory load from embedded PNG
        if let Ok(icon) =
            CreateIconFromResourceEx(TRAY_PNG, true, 0x00030000, 0, 0, LR_DEFAULTCOLOR)
        {
            if !icon.0.is_null() {
                return icon;
            }
        }

        // 3. Fallback to generic system icon
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

pub struct TrayIcon {
    pub hwnd: HWND,
    pub taskbar_created_msg: u32,
    pub icon: HICON,
}

impl TrayIcon {
    pub fn new() -> Result<Self> {
        let class_name_w: Vec<u16> = format!("{}\0", TRAY_CLASS_NAME).encode_utf16().collect();
        let taskbar_msg_name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();

        // SAFETY: Register tray window class and taskbar message.
        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(tray_wnd_proc),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: PCWSTR::from_raw(class_name_w.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
                PCWSTR::from_raw(class_name_w.as_ptr()),
                PCWSTR::null(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )?;

            let icon = load_app_icon();
            let taskbar_created_msg =
                RegisterWindowMessageW(PCWSTR::from_raw(taskbar_msg_name.as_ptr()));

            let tray = Self {
                hwnd,
                taskbar_created_msg,
                icon,
            };

            tray.add_icon();
            Ok(tray)
        }
    }

    pub fn add_icon(&self) {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: self.icon,
            ..Default::default()
        };

        let tip = "Alt+Tab Flip 3D (Vista Style)\0";
        for (i, c) in tip.encode_utf16().enumerate().take(127) {
            nid.szTip[i] = c;
        }

        // SAFETY: Shell_NotifyIconW adds tray icon.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }
    }

    pub fn remove_icon(&self) {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        // SAFETY: Shell_NotifyIconW removes tray icon.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            if !self.icon.0.is_null() {
                let _ = DestroyIcon(self.icon);
            }
        }
    }

    pub fn show_context_menu(&self) {
        // SAFETY: CreatePopupMenu and show context menu.
        unsafe {
            let menu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(_) => return,
            };

            let autostart_menu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(_) => {
                    let _ = DestroyMenu(menu);
                    return;
                }
            };

            let current_autostart = get_autostart_mode();
            let elevated = is_elevated();
            let paused = !G_TAKE_OVER.load(Ordering::SeqCst);

            let append_item = |hmenu: HMENU, id: usize, text: &str, flags: u32| {
                let wide: Vec<u16> = format!("{}\0", text).encode_utf16().collect();
                let _ = AppendMenuW(
                    hmenu,
                    windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS(flags),
                    id,
                    PCWSTR::from_raw(wide.as_ptr()),
                );
            };

            append_item(menu, ID_SETTINGS, "Settings...", MF_STRING.0);
            append_item(
                menu,
                ID_PAUSE,
                "Pause Switcher",
                MF_STRING.0 | if paused { MF_CHECKED.0 } else { MF_UNCHECKED.0 },
            );
            append_item(menu, 0, "", MF_SEPARATOR.0);

            // Submenu: Start with Windows
            append_item(
                autostart_menu,
                ID_AUTOSTART_DISABLED,
                "Disabled",
                MF_STRING.0
                    | if current_autostart == AutostartMode::Disabled {
                        MF_CHECKED.0
                    } else {
                        MF_UNCHECKED.0
                    },
            );
            append_item(
                autostart_menu,
                ID_AUTOSTART_NORMAL,
                "Normal (Run key)",
                MF_STRING.0
                    | if current_autostart == AutostartMode::Normal {
                        MF_CHECKED.0
                    } else {
                        MF_UNCHECKED.0
                    },
            );
            append_item(
                autostart_menu,
                ID_AUTOSTART_ELEVATED,
                "Elevated (Task Scheduler)",
                MF_STRING.0
                    | if current_autostart == AutostartMode::Elevated {
                        MF_CHECKED.0
                    } else {
                        MF_UNCHECKED.0
                    },
            );

            let sub_title: Vec<u16> = "Start with Windows\0".encode_utf16().collect();
            let _ = AppendMenuW(
                menu,
                windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS(MF_POPUP.0),
                autostart_menu.0 as usize,
                PCWSTR::from_raw(sub_title.as_ptr()),
            );

            // Run as administrator
            if elevated {
                append_item(
                    menu,
                    ID_RUN_ADMIN,
                    "Running as administrator ✓",
                    MF_STRING.0 | MF_GRAYED.0,
                );
            } else {
                append_item(menu, ID_RUN_ADMIN, "Run as administrator", MF_STRING.0);
            }

            append_item(menu, 0, "", MF_SEPARATOR.0);
            append_item(menu, ID_ABOUT, "About Flip 3D...", MF_STRING.0);
            append_item(menu, ID_EXIT, "Exit", MF_STRING.0);

            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);

            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenuEx(
                menu,
                (TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RETURNCMD).0,
                cursor.x,
                cursor.y,
                self.hwnd,
                None,
            );

            let _ = DestroyMenu(menu);

            match cmd.0 as usize {
                ID_SETTINGS => spawn_settings_window(),
                ID_PAUSE => {
                    let new_val = G_TAKE_OVER.fetch_xor(true, Ordering::SeqCst) ^ true;
                    log::info!("Switcher pause toggled: active={}", new_val);
                    let hook_thread_id = G_HOOK_THREAD_ID.load(Ordering::SeqCst);
                    if hook_thread_id != 0 {
                        let _ = PostThreadMessageW(
                            hook_thread_id,
                            WM_HOOK_UPDATE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    let overlay = get_overlay_hwnd();
                    if !overlay.0.is_null() {
                        let _ = PostMessageW(Some(overlay), WM_APP_SETTINGS, WPARAM(0), LPARAM(0));
                    }
                }
                ID_AUTOSTART_DISABLED => {
                    let _ = set_autostart_mode(AutostartMode::Disabled);
                }
                ID_AUTOSTART_NORMAL => {
                    let _ = set_autostart_mode(AutostartMode::Normal);
                }
                ID_AUTOSTART_ELEVATED => {
                    let _ = set_autostart_mode(AutostartMode::Elevated);
                }
                ID_RUN_ADMIN => {
                    if !elevated {
                        let _ = relaunch_elevated();
                    }
                }
                ID_ABOUT => {
                    let title_w: Vec<u16> = "About Alt+Tab Flip 3D\0".encode_utf16().collect();
                    let msg_w: Vec<u16> = concat!(
                        "Alt+Tab Flip 3D (Vista Style) - Standalone Rust Port v1.7.1\n\n",
                        "Original Windhawk mod by caliberda (https://github.com/cesarkali)\n",
                        "Ported to standalone portable Rust by Vigi Riyantanu.\n\n",
                        "Licensed under the MIT License.\n",
                        "Supports Windows 10 (1903+) and Windows 11."
                    )
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                    windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        Some(self.hwnd),
                        PCWSTR::from_raw(msg_w.as_ptr()),
                        PCWSTR::from_raw(title_w.as_ptr()),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK
                            | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                    );
                }
                ID_EXIT => {
                    log::info!("Exit requested from tray menu");
                    // Post WM_QUIT to overlay and UI thread
                    let overlay = get_overlay_hwnd();
                    if !overlay.0.is_null() {
                        let _ = PostMessageW(
                            Some(overlay),
                            windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    PostQuitMessage(0);
                }
                _ => {}
            }
        }
    }
}

/// # Safety
/// Standard Win32 window procedure callback invoked by Windows OS message pump.
pub unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = RegisterWindowMessageW(PCWSTR::from_raw(
        "TaskbarCreated\0"
            .encode_utf16()
            .collect::<Vec<_>>()
            .as_ptr(),
    ));

    if msg == taskbar_msg {
        // Recreate tray icon if explorer restarts
        let icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let tray = TrayIcon {
            hwnd,
            taskbar_created_msg: taskbar_msg,
            icon,
        };
        tray.add_icon();
        return LRESULT(0);
    }

    if msg == WM_TRAYICON {
        let event = (lparam.0 & 0xffff) as u32;
        if event == WM_RBUTTONUP {
            let icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
            let tray = TrayIcon {
                hwnd,
                taskbar_created_msg: taskbar_msg,
                icon,
            };
            tray.show_context_menu();
        } else if event == WM_LBUTTONDBLCLK {
            spawn_settings_window();
        }
        return LRESULT(0);
    }

    if msg == WM_TRAY_BALLOON {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: windows::Win32::UI::Shell::NIF_INFO,
            dwInfoFlags: windows::Win32::UI::Shell::NIIF_INFO,
            ..Default::default()
        };
        let title: Vec<u16> = "Flip 3D\0".encode_utf16().collect();
        let info: Vec<u16> = "Flip 3D is already running.\0".encode_utf16().collect();
        let title_len = title.len().min(nid.szInfoTitle.len());
        nid.szInfoTitle[..title_len].copy_from_slice(&title[..title_len]);
        let info_len = info.len().min(nid.szInfo.len());
        nid.szInfo[..info_len].copy_from_slice(&info[..info_len]);
        let _ = Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_MODIFY, &nid);
        return LRESULT(0);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
