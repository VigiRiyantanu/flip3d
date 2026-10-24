// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::os::windows::process::CommandExt;
use std::sync::Arc;
use tao::window::Window;
use wry::WebView;

use crate::engine_link;
use crate::window_fx::BlurInfo;
use settings_core::{Config, ALL_STYLES};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Deserialize)]
pub struct IpcMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct IpcResponse {
    pub id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn send_response(webview: &WebView, resp: &IpcResponse) {
    if let Ok(json_str) = serde_json::to_string(resp) {
        let script = format!("window.__app.receive({});", json_str);
        let _ = webview.evaluate_script(&script);
    }
}

pub fn send_push_event(webview: &WebView, event: &str, data: serde_json::Value) {
    let payload = json!({
        "event": event,
        "data": data,
    });
    if let Ok(json_str) = serde_json::to_string(&payload) {
        let script = format!("window.__app.push({});", json_str);
        let _ = webview.evaluate_script(&script);
    }
}

pub enum DispatchResult {
    Continue,
    Exit,
}

pub fn handle_ipc_message(
    msg: IpcMessage,
    window: &Arc<Window>,
    webview: &WebView,
    blur_info: &BlurInfo,
) -> DispatchResult {
    match msg.msg_type.as_str() {
        "ui_ready" => {
            log::info!("UI ready signal received from frontend. Revealing settings window.");
            window.set_visible(true);
            window.set_focus();
            let resp = IpcResponse {
                id: msg.id,
                ok: true,
                payload: None,
                error: None,
            };
            send_response(webview, &resp);
        }
        "log" => {
            let level = msg
                .payload
                .get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("info");
            let message = msg
                .payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match level {
                "error" => log::error!("[JS ERROR] {}", message),
                "warn" => log::warn!("[JS WARN] {}", message),
                _ => log::info!("[JS LOG] {}", message),
            }
        }
        "drag_window" => {
            let _ = window.drag_window();
        }
        "minimize_window" => {
            window.set_minimized(true);
        }
        "close_window" => {
            return DispatchResult::Exit;
        }
        "get_initial_data" => {
            let (config, _) = settings_core::load_config();
            let defaults = Config::default();
            let is_engine_running = engine_link::is_engine_running();
            let autostart = engine_link::is_autostart_enabled();
            let paths = settings_core::AppPaths::get();
            let distribution = match (paths.mode, paths.fallback_from_portable) {
                (settings_core::DistributionMode::Portable, false) => "portable",
                (settings_core::DistributionMode::Portable, true) => "portable_fallback",
                (settings_core::DistributionMode::Installed, _) => "installed",
            };
            let data_path = paths.config_dir().display().to_string();

            let resp = IpcResponse {
                id: msg.id,
                ok: true,
                payload: Some(json!({
                    "config": config,
                    "defaults": defaults,
                    "all_styles": ALL_STYLES,
                    "is_engine_running": is_engine_running,
                    "blur_status": blur_info.blur_type,
                    "blur_status_text": blur_info.status_text,
                    "autostart": autostart,
                    "distribution": distribution,
                    "data_path": data_path,
                })),
                error: None,
            };
            send_response(webview, &resp);
        }
        "save_config" => {
            let config_val = msg.payload.get("config").cloned().unwrap_or_default();
            match serde_json::from_value::<Config>(config_val) {
                Ok(mut config) => {
                    config.validate_and_clamp();
                    match settings_core::save_config_atomic(&config) {
                        Ok(_) => {
                            let apply_live = msg
                                .payload
                                .get("apply_live")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            if apply_live {
                                engine_link::notify_settings_changed();
                            }
                            let resp = IpcResponse {
                                id: msg.id,
                                ok: true,
                                payload: Some(json!({ "config": config })),
                                error: None,
                            };
                            send_response(webview, &resp);
                        }
                        Err(e) => {
                            let resp = IpcResponse {
                                id: msg.id,
                                ok: false,
                                payload: None,
                                error: Some(format!("Failed to save config: {}", e)),
                            };
                            send_response(webview, &resp);
                        }
                    }
                }
                Err(e) => {
                    let resp = IpcResponse {
                        id: msg.id,
                        ok: false,
                        payload: None,
                        error: Some(format!("Invalid config schema: {}", e)),
                    };
                    send_response(webview, &resp);
                }
            }
        }
        "restore_defaults" => {
            let resp = IpcResponse {
                id: msg.id,
                ok: true,
                payload: Some(json!({ "config": Config::default() })),
                error: None,
            };
            send_response(webview, &resp);
        }
        "get_autostart" => {
            let enabled = engine_link::is_autostart_enabled();
            let resp = IpcResponse {
                id: msg.id,
                ok: true,
                payload: Some(json!({ "enabled": enabled })),
                error: None,
            };
            send_response(webview, &resp);
        }
        "set_autostart" => {
            let enabled = msg
                .payload
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match engine_link::set_autostart_enabled(enabled) {
                Ok(_) => {
                    let resp = IpcResponse {
                        id: msg.id,
                        ok: true,
                        payload: Some(json!({ "enabled": enabled })),
                        error: None,
                    };
                    send_response(webview, &resp);
                }
                Err(e) => {
                    let resp = IpcResponse {
                        id: msg.id,
                        ok: false,
                        payload: None,
                        error: Some(e),
                    };
                    send_response(webview, &resp);
                }
            }
        }
        "export_settings" => {
            let file_path = rfd::FileDialog::new()
                .add_filter("JSON Config (*.json)", &["json"])
                .set_file_name("flip3d-config.json")
                .save_file();

            if let Some(path) = file_path {
                let (config, _) = settings_core::load_config();
                if let Ok(json_str) = serde_json::to_string_pretty(&config) {
                    if std::fs::write(&path, json_str).is_ok() {
                        let resp = IpcResponse {
                            id: msg.id,
                            ok: true,
                            payload: Some(json!({ "path": path.display().to_string() })),
                            error: None,
                        };
                        send_response(webview, &resp);
                        return DispatchResult::Continue;
                    }
                }
            }
            let resp = IpcResponse {
                id: msg.id,
                ok: false,
                payload: None,
                error: Some("Export cancelled or write error".to_string()),
            };
            send_response(webview, &resp);
        }
        "import_settings" => {
            let file_path = rfd::FileDialog::new()
                .add_filter("JSON Config (*.json)", &["json"])
                .pick_file();

            if let Some(path) = file_path {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut config) = serde_json::from_str::<Config>(&content) {
                        config.validate_and_clamp();
                        if settings_core::save_config_atomic(&config).is_ok() {
                            engine_link::notify_settings_changed();
                            let resp = IpcResponse {
                                id: msg.id,
                                ok: true,
                                payload: Some(json!({ "config": config })),
                                error: None,
                            };
                            send_response(webview, &resp);
                            return DispatchResult::Continue;
                        }
                    }
                }
            }
            let resp = IpcResponse {
                id: msg.id,
                ok: false,
                payload: None,
                error: Some("Import cancelled or invalid file".to_string()),
            };
            send_response(webview, &resp);
        }
        "toggle_engine_pause" => {
            engine_link::notify_settings_changed();
            let is_active = engine_link::is_engine_running();
            let resp = IpcResponse {
                id: msg.id,
                ok: true,
                payload: Some(json!({ "is_active": is_active })),
                error: None,
            };
            send_response(webview, &resp);
        }
        "open_config_folder" => {
            let dir = settings_core::app_data_dir();
            let mut cmd = std::process::Command::new("explorer.exe");
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.arg(dir);
            let _ = cmd.spawn();
        }
        "open_log_file" => {
            let log_path = settings_core::get_log_file_path("flip3d");
            let mut cmd = std::process::Command::new("notepad.exe");
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.arg(log_path);
            let _ = cmd.spawn();
        }
        _ => {
            log::warn!("Unhandled IPC command: {}", msg.msg_type);
        }
    }

    DispatchResult::Continue
}
