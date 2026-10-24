// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

pub mod app;
pub mod engine_link;
pub mod ipc;
pub mod protocol;
pub mod webview_runtime;
pub mod window_fx;

use tao::event::Event;
use tao::event_loop::ControlFlow;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::app::SettingsApp;
use crate::ipc::{handle_ipc_message, DispatchResult, IpcMessage};

pub fn run_settings_ui(debug_mode: bool) {
    let _ = settings_core::FileLogger::init("settings-ui");
    log::info!(
        "Flip 3D Settings UI starting up. debug_mode: {}",
        debug_mode
    );

    // 1. Single Instance Check via named mutex
    let mutex_name_w: Vec<u16> = settings_core::SETTINGS_UI_MUTEX_NAME
        .encode_utf16()
        .collect();
    let single_instance_mutex =
        unsafe { CreateMutexW(None, true, PCWSTR::from_raw(mutex_name_w.as_ptr())) };

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        log::warn!("Settings UI is already running. Focusing existing instance.");
        let title_w: Vec<u16> = "Flip 3D Settings\0".encode_utf16().collect();
        unsafe {
            if let Ok(hwnd) = FindWindowW(PCWSTR::null(), PCWSTR::from_raw(title_w.as_ptr())) {
                if !hwnd.0.is_null() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
        return;
    }

    // 2. Ensure WebView2 Runtime exists
    if !webview_runtime::ensure_webview2_runtime() {
        log::error!("WebView2 Runtime missing. Aborting launch.");
        return;
    }

    // 3. Construct application and window
    let app = match SettingsApp::build(debug_mode) {
        Ok(app) => app,
        Err(e) => {
            log::error!("Failed to initialize Settings UI: {:#?}", e);
            return;
        }
    };

    let window = app.window;
    let webview = app.webview;
    let blur_info = app.blur_info;

    log::info!("Entering Settings UI event loop.");

    app.event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(raw_json) => match serde_json::from_str::<IpcMessage>(&raw_json) {
                Ok(msg) => {
                    let res = handle_ipc_message(msg, &window, &webview, &blur_info);
                    if let DispatchResult::Exit = res {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Err(e) => {
                    log::error!("Failed to deserialize IPC message ({}): {}", e, raw_json);
                }
            },
            Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                log::info!("Settings UI event loop exited. Cleaning up mutex.");
                if let Ok(m) = single_instance_mutex {
                    unsafe {
                        let _ = CloseHandle(m);
                    }
                }
            }
            _ => {}
        }
    });
}
