#![windows_subsystem = "windows"]

// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use std::sync::atomic::Ordering;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, CreateThread, SetEvent, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_QUIT,
};

use flip3d::app::tray::TrayIcon;
use flip3d::hooks::hook_thread_proc;
use flip3d::logging::FileLogger;
use flip3d::shortcuts::{get_overlay_hwnd, G_HOOK_THREAD_ID, G_UI_THREAD_ID, WM_APP_SETTINGS};
use flip3d::switcher::Switcher;

#[repr(C)]
struct OsVersionInfoW {
    os_version_info_size: u32,
    major_version: u32,
    minor_version: u32,
    build_number: u32,
    platform_id: u32,
    sz_csd_version: [u16; 128],
}

fn get_windows_build() -> (u32, u32, u32) {
    // SAFETY: Dynamically query RtlGetVersion from ntdll.dll to avoid manifest spoofing.
    unsafe {
        let ntdll_name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
        let ntdll = GetModuleHandleW(PCWSTR::from_raw(ntdll_name.as_ptr()));
        if let Ok(h) = ntdll {
            let proc = GetProcAddress(h, windows::core::s!("RtlGetVersion"));
            if let Some(rtl_get_version) = proc {
                let func: extern "system" fn(*mut OsVersionInfoW) -> i32 =
                    std::mem::transmute(rtl_get_version);
                let mut info = OsVersionInfoW {
                    os_version_info_size: std::mem::size_of::<OsVersionInfoW>() as u32,
                    major_version: 0,
                    minor_version: 0,
                    build_number: 0,
                    platform_id: 0,
                    sz_csd_version: [0; 128],
                };
                if func(&mut info) == 0 {
                    return (info.major_version, info.minor_version, info.build_number);
                }
            }
        }
    }
    (10, 0, 19041)
}

unsafe extern "system" fn ui_thread_entry(parameter: *mut std::ffi::c_void) -> u32 {
    let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    let start_event = HANDLE(parameter);
    let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
    if hr.is_err() {
        log::error!("CoInitializeEx failed in UI thread: {:?}", hr);
        let _ = SetEvent(start_event);
        return 1;
    }

    {
        let mut switcher = Switcher::new();
        let created = switcher.create_overlay();
        let _ = SetEvent(start_event);
        if created {
            switcher.activate();
            switcher.run();
        }
        switcher.shutdown();
    }

    CoUninitialize();
    0
}

unsafe extern "system" fn hook_thread_entry(parameter: *mut std::ffi::c_void) -> u32 {
    let start_event = HANDLE(parameter);
    hook_thread_proc(start_event)
}

fn show_already_running_notification() {
    let class_name_w: Vec<u16> = "WindhawkFlip3DTrayWindow\0".encode_utf16().collect();
    unsafe {
        if let Ok(hwnd) = windows::Win32::UI::WindowsAndMessaging::FindWindowW(
            PCWSTR::from_raw(class_name_w.as_ptr()),
            PCWSTR::null(),
        ) {
            if !hwnd.0.is_null() {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(hwnd),
                    flip3d::app::tray::WM_TRAY_BALLOON,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--settings" || a == "-s") {
        let debug_mode = args.iter().any(|a| a == "--debug" || a == "-d");
        settings_ui::run_settings_ui(debug_mode);
        return;
    }

    let _ = FileLogger::init("flip3d");
    log::info!(
        "Alt+Tab Flip 3D (Vista Style) starting up. Args: {:?}",
        args
    );

    let (major, minor, build) = get_windows_build();
    log::info!("Windows OS Version: {}.{}.{}", major, minor, build);

    flip3d::app::autostart::repair_portable_autostart_if_needed();

    // Enforce single instance using the designated engine mutex
    let mutex_name_w: Vec<u16> = settings_core::ENGINE_MUTEX_NAME.encode_utf16().collect();

    // SAFETY: CreateMutexW creates session-scoped mutex.
    let single_instance_mutex =
        unsafe { CreateMutexW(None, true, PCWSTR::from_raw(mutex_name_w.as_ptr())) };

    if unsafe { GetLastError() } == windows::Win32::Foundation::ERROR_ALREADY_EXISTS {
        log::warn!("Another instance of Flip 3D is already running in this session. Exiting.");
        show_already_running_notification();
        return;
    }

    // Set DPI awareness for main thread
    // SAFETY: SetThreadDpiAwarenessContext to PerMonitorV2.
    unsafe {
        let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Spawn UI thread
    // SAFETY: CreateEventW creates event for synchronization.
    let ui_start_event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
        .expect("Failed to create UI start event");

    let mut ui_thread_id = 0u32;
    // SAFETY: CreateThread launches UI thread.
    let ui_thread = unsafe {
        CreateThread(
            None,
            0,
            Some(ui_thread_entry),
            Some(ui_start_event.0),
            windows::Win32::System::Threading::THREAD_CREATION_FLAGS(0),
            Some(&mut ui_thread_id),
        )
    }
    .expect("Failed to create UI thread");

    unsafe {
        WaitForSingleObject(ui_start_event, INFINITE);
        let _ = CloseHandle(ui_start_event);
    }
    G_UI_THREAD_ID.store(ui_thread_id, Ordering::SeqCst);

    // Spawn Hook thread
    let hook_start_event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
        .expect("Failed to create hook start event");

    let mut hook_thread_id = 0u32;
    // SAFETY: CreateThread launches Hook thread.
    let hook_thread = unsafe {
        CreateThread(
            None,
            0,
            Some(hook_thread_entry),
            Some(hook_start_event.0),
            windows::Win32::System::Threading::THREAD_CREATION_FLAGS(0),
            Some(&mut hook_thread_id),
        )
    }
    .expect("Failed to create hook thread");

    unsafe {
        WaitForSingleObject(hook_start_event, INFINITE);
        let _ = CloseHandle(hook_start_event);
    }
    G_HOOK_THREAD_ID.store(hook_thread_id, Ordering::SeqCst);

    // Spawn Settings change watcher (listens on event and polls file modification time)
    let watcher_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let watcher_running_clone = watcher_running.clone();
    let watcher_thread = std::thread::spawn(move || {
        let change_event = settings_core::create_settings_change_event();
        let config_path = settings_core::config_json_path();
        let mut last_modified = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();

        while watcher_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
            let signaled = if let Ok(evt) = change_event {
                unsafe {
                    WaitForSingleObject(evt, 1000) == windows::Win32::Foundation::WAIT_OBJECT_0
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                false
            };

            if !watcher_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let current_modified = std::fs::metadata(&config_path)
                .and_then(|m| m.modified())
                .ok();
            let file_changed = current_modified.is_some() && current_modified != last_modified;

            if signaled || file_changed {
                last_modified = current_modified;
                log::info!(
                    "Settings change detected (event_signaled={}, file_changed={}). Reloading switcher settings...",
                    signaled,
                    file_changed
                );
                let overlay = get_overlay_hwnd();
                if !overlay.0.is_null() {
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            Some(overlay),
                            WM_APP_SETTINGS,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
        }

        if let Ok(evt) = change_event {
            unsafe {
                let _ = CloseHandle(evt);
            }
        }
    });

    // Initialize System Tray Icon
    let tray = match TrayIcon::new() {
        Ok(t) => {
            log::info!("System tray icon registered successfully");
            Some(t)
        }
        Err(e) => {
            log::error!("Failed to create system tray icon: {:?}", e);
            None
        }
    };

    // Run tray message pump in main thread
    let mut msg = MSG::default();
    // SAFETY: GetMessageW runs main message loop.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    log::info!("Shutting down Flip 3D...");

    // Stop watcher thread
    watcher_running.store(false, Ordering::SeqCst);
    settings_core::notify_settings_changed();
    let _ = watcher_thread.join();

    // Remove tray icon
    if let Some(t) = tray {
        t.remove_icon();
    }

    // Signal UI thread and hook thread to terminate
    unsafe {
        let _ = PostThreadMessageW(hook_thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        let overlay = get_overlay_hwnd();
        if !overlay.0.is_null() {
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(overlay),
                WM_QUIT,
                WPARAM(0),
                LPARAM(0),
            );
        }
        let _ = PostThreadMessageW(ui_thread_id, WM_QUIT, WPARAM(0), LPARAM(0));

        let _ = WaitForSingleObject(hook_thread, 3000);
        let _ = WaitForSingleObject(ui_thread, 3000);

        let _ = CloseHandle(hook_thread);
        let _ = CloseHandle(ui_thread);

        if let Ok(m) = single_instance_mutex {
            let _ = CloseHandle(m);
        }
    }

    log::info!("Flip 3D exited cleanly.");
}
