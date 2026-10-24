// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::window::{Window, WindowBuilder};
use wry::{WebContext, WebView, WebViewBuilder};

use crate::protocol;
use crate::webview_runtime;
use crate::window_fx::{self, BlurInfo};

pub struct SettingsApp {
    pub event_loop: EventLoop<String>,
    pub window: Arc<Window>,
    pub webview: WebView,
    pub blur_info: BlurInfo,
}

impl SettingsApp {
    pub fn build(debug_mode: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let (major, minor, build) = webview_runtime::get_windows_version();
        log::info!("Detected Windows OS: {}.{}.{}", major, minor, build);

        let event_loop = EventLoopBuilder::<String>::with_user_event().build();
        let proxy: EventLoopProxy<String> = event_loop.create_proxy();

        log::info!("Creating Tao frameless window (initially hidden)...");
        let window = WindowBuilder::new()
            .with_title("Flip 3D Settings")
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false) // Wait for ui_ready before showing window
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(900.0, 620.0))
            .with_min_inner_size(LogicalSize::new(760.0, 520.0))
            .build(&event_loop)?;

        // Apply real OS Acrylic / Mica blur effects and rounded corners
        let blur_info = window_fx::apply_window_effects(&window, build);
        log::info!("Window blur effect configured: {}", blur_info.status_text);

        let window = Arc::new(window);

        // WebView2 initialization with dedicated isolated user data directory
        let user_data_dir = webview_runtime::get_user_data_dir();
        log::info!("WebView2 user data folder: {:?}", user_data_dir);
        let mut web_context = WebContext::new(Some(user_data_dir));

        let proxy_clone = proxy.clone();

        log::info!("Building Wry WebView with custom protocol 'app://'...");
        let mut builder = WebViewBuilder::with_web_context(&mut web_context);
        builder = builder
            .with_transparent(true)
            .with_background_color((0, 0, 0, 0))
            .with_custom_protocol(protocol::CUSTOM_PROTOCOL_NAME.into(), |_id, req| {
                protocol::protocol_handler(req)
            })
            .with_url("app://localhost/index.html")
            .with_devtools(debug_mode)
            .with_on_page_load_handler(|event, url| {
                let event_name = match event {
                    wry::PageLoadEvent::Started => "Started",
                    wry::PageLoadEvent::Finished => "Finished",
                };
                log::info!("WebView2 page load event: {}, url: {}", event_name, url);
            })
            .with_ipc_handler(move |req| {
                let _ = proxy_clone.send_event(req.into_body());
            });

        let webview = builder.build(&*window)?;
        log::info!("WebView2 built successfully.");

        // Fallback safety timer: if frontend doesn't send ui_ready within 3s, show window
        let window_safety = Arc::clone(&window);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(3000));
            if !window_safety.is_visible() {
                log::warn!("Safety timer triggered: revealing window after 3s ui_ready timeout");
                window_safety.set_visible(true);
            }
        });

        Ok(Self {
            event_loop,
            window,
            webview,
            blur_info,
        })
    }
}
