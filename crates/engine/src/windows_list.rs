// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DWM_CLOAKED_SHELL,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, IntersectRect, MonitorFromRect, OffsetRect, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::IVirtualDesktopManager;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindow,
    GetWindowLongPtrW, GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, GWL_EXSTYLE, GW_OWNER, WPF_RESTORETOMAXIMIZED, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::settings::Settings;
use crate::shortcuts::get_overlay_hwnd;

#[derive(Clone, Default)]
pub struct WindowFilter {
    pub desktops: Option<IVirtualDesktopManager>,
    pub other_desktops: bool,
    pub manager_hidden: bool,
}

pub fn windows_shows_all_desktops() -> bool {
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced\0"
        .encode_utf16()
        .collect();
    let val_name: Vec<u16> = "VirtualDesktopAltTabFilter\0".encode_utf16().collect();

    // SAFETY: RegGetValueW safely reads DWORD registry value.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey.as_ptr()),
            PCWSTR::from_raw(val_name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut _ as *mut _),
            Some(&mut size),
        )
    };
    status == ERROR_SUCCESS && value == 0
}

pub fn show_hidden_window(hwnd: HWND, cloaked: u32, filter: &WindowFilter) -> bool {
    let desktops = match &filter.desktops {
        Some(d) => d,
        None => return false,
    };
    if cloaked != DWM_CLOAKED_SHELL {
        return false;
    }

    let mut class_buf = [0u16; 64];
    // SAFETY: GetClassNameW writes class name to buffer.
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name == "Windows.UI.Core.CoreWindow" {
            return false;
        }
    }

    // SAFETY: IVirtualDesktopManager COM methods query desktop GUID and current status.
    let (desktop, current) = unsafe {
        let desk = match desktops.GetWindowDesktopId(hwnd) {
            Ok(d) if d != GUID::default() => d,
            _ => return false,
        };
        let cur = match desktops.IsWindowOnCurrentVirtualDesktop(hwnd) {
            Ok(c) => c,
            _ => return false,
        };
        (desk, cur)
    };

    let _ = desktop;
    if current.as_bool() {
        filter.manager_hidden
    } else {
        filter.other_desktops
    }
}

pub fn is_switchable_window(hwnd: HWND, filter: &WindowFilter, hidden: &mut bool) -> bool {
    *hidden = false;
    let overlay_wnd = get_overlay_hwnd();
    // SAFETY: Checking window visibility and shell window.
    unsafe {
        if hwnd == overlay_wnd || !IsWindowVisible(hwnd).as_bool() || hwnd == GetShellWindow() {
            return false;
        }

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if (ex_style & WS_EX_APPWINDOW.0 as isize) == 0 {
            if (ex_style & (WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize)) != 0 {
                return false;
            }
            if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
                if !owner.0.is_null() {
                    return false;
                }
            }
        }

        let mut class_buf = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_buf[..len as usize]);
            for excluded in [
                "Progman",
                "WorkerW",
                "Shell_TrayWnd",
                "Shell_SecondaryTrayWnd",
            ] {
                if class_str == excluded {
                    return false;
                }
            }
        }

        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
        {
            if !show_hidden_window(hwnd, cloaked, filter) {
                return false;
            }
            *hidden = true;
        }
    }

    true
}

pub fn get_window_process_path(hwnd: HWND) -> String {
    let mut pid = 0u32;
    // SAFETY: GetWindowThreadProcessId extracts PID.
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return String::new();
        }
        let process = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(p) => p,
            Err(_) => return String::new(),
        };

        let mut path_buf = [0u16; 512];
        let mut size = path_buf.len() as u32;
        let success = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(path_buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(process);

        if success.is_ok() && size > 0 {
            String::from_utf16_lossy(&path_buf[..size as usize])
        } else {
            String::new()
        }
    }
}

pub fn is_excluded_app(hwnd: HWND, excluded: &[String]) -> bool {
    if excluded.is_empty() {
        return false;
    }

    let mut class_buf = [0u16; 256];
    // SAFETY: GetClassNameW to check class exclusions.
    let mut actual_hwnd = hwnd;
    unsafe {
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]).to_lowercase();
            if excluded.iter().any(|ex| ex == &class_name) {
                return true;
            }
            if class_name == "applicationframewindow" {
                let core_class: Vec<u16> = "Windows.UI.Core.CoreWindow\0".encode_utf16().collect();
                if let Ok(core) = FindWindowExW(
                    Some(hwnd),
                    None,
                    PCWSTR::from_raw(core_class.as_ptr()),
                    PCWSTR::null(),
                ) {
                    if !core.0.is_null() {
                        actual_hwnd = core;
                    }
                }
            }
        }
    }

    let path = get_window_process_path(actual_hwnd);
    if path.is_empty() {
        return false;
    }
    let file_name = match path.rfind('\\') {
        Some(idx) => path[idx + 1..].to_lowercase(),
        None => path.to_lowercase(),
    };

    excluded.iter().any(|ex| ex == &file_name)
}

pub fn is_bar_window(hwnd: HWND) -> bool {
    // SAFETY: Query window rect, iconic state, and monitor info.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }

        let monitor = MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let monitor_width = info.rcMonitor.right - info.rcMonitor.left;
        let monitor_height = info.rcMonitor.bottom - info.rcMonitor.top;

        let horizontal = width * 10 >= monitor_width * 3 && height * 100 <= monitor_height * 15;
        let vertical = height * 10 >= monitor_height * 3 && width * 100 <= monitor_width * 15;
        if !horizontal && !vertical {
            return false;
        }

        if (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOPMOST.0 as isize) != 0 {
            return true;
        }

        let mut in_work_area = RECT::default();
        if !IntersectRect(&mut in_work_area, &rect, &info.rcWork).as_bool() {
            return true;
        }

        let visible =
            (in_work_area.right - in_work_area.left) * (in_work_area.bottom - in_work_area.top);
        visible * 2 < width * height
    }
}

pub fn get_window_bounds(hwnd: HWND, minimized: bool, rect: &mut RECT) -> bool {
    // SAFETY: Query window placement or extended frame bounds.
    unsafe {
        if minimized {
            let mut placement = windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT {
                length: std::mem::size_of::<windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT>(
                ) as u32,
                ..Default::default()
            };
            if GetWindowPlacement(hwnd, &mut placement).is_err() {
                return false;
            }
            *rect = placement.rcNormalPosition;
            let monitor = MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                if (placement.flags.0 & WPF_RESTORETOMAXIMIZED.0) != 0 {
                    *rect = info.rcWork;
                } else if (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize) == 0
                {
                    let _ = OffsetRect(
                        rect,
                        info.rcWork.left - info.rcMonitor.left,
                        info.rcWork.top - info.rcMonitor.top,
                    );
                }
            }
        } else if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_err()
            && GetWindowRect(hwnd, rect).is_err()
        {
            return false;
        }

        (rect.right - rect.left > 0) && (rect.bottom - rect.top > 0)
    }
}

pub struct SwitchableWindowInfo {
    pub hwnd: HWND,
    pub hidden: bool,
}

struct EnumContext<'a> {
    windows: Vec<SwitchableWindowInfo>,
    settings: &'a Settings,
    filter: &'a WindowFilter,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> windows_core::BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumContext);
    let mut hidden = false;
    if is_switchable_window(hwnd, ctx.filter, &mut hidden)
        && (ctx.settings.include_minimized || !IsIconic(hwnd).as_bool())
        && !(ctx.settings.hide_bars && is_bar_window(hwnd))
        && !is_excluded_app(hwnd, &ctx.settings.excluded_apps)
    {
        ctx.windows.push(SwitchableWindowInfo { hwnd, hidden });
    }
    windows_core::BOOL((ctx.windows.len() < ctx.settings.max_windows as usize) as i32)
}

pub fn collect_windows(settings: &Settings, filter: &WindowFilter) -> Vec<SwitchableWindowInfo> {
    let mut ctx = EnumContext {
        windows: Vec::new(),
        settings,
        filter,
    };

    // SAFETY: EnumWindows enumerates top-level windows in Z order.
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );

        let foreground = GetForegroundWindow();
        if let Some(pos) = ctx.windows.iter().position(|w| w.hwnd == foreground) {
            ctx.windows[0..=pos].rotate_right(1);
        }
    }

    ctx.windows
}
