// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use std::borrow::Cow;
use wry::http::{header, Request, Response, StatusCode};

pub const CUSTOM_PROTOCOL_NAME: &str = "app";

// Embedded assets
static INDEX_HTML: &[u8] = include_bytes!("../ui/index.html");
static APP_CSS: &[u8] = include_bytes!("../ui/dist/app.css");
static TOKENS_CSS: &[u8] = include_bytes!("../ui/css/tokens.css");
static BASE_CSS: &[u8] = include_bytes!("../ui/css/base.css");
static COMPONENTS_CSS: &[u8] = include_bytes!("../ui/css/components.css");
static MAIN_JS: &[u8] = include_bytes!("../ui/js/main.js");
static IPC_JS: &[u8] = include_bytes!("../ui/js/ipc.js");
static STORE_JS: &[u8] = include_bytes!("../ui/js/store.js");
static BIND_JS: &[u8] = include_bytes!("../ui/js/bind.js");
static VIEW_STYLE_JS: &[u8] = include_bytes!("../ui/js/views/style.js");
static VIEW_SHORTCUTS_JS: &[u8] = include_bytes!("../ui/js/views/shortcuts.js");
static VIEW_ANIMATION_JS: &[u8] = include_bytes!("../ui/js/views/animation.js");
static VIEW_WINDOWS_JS: &[u8] = include_bytes!("../ui/js/views/windows.js");
static VIEW_BACKGROUND_JS: &[u8] = include_bytes!("../ui/js/views/background.js");
static VIEW_DISPLAY_JS: &[u8] = include_bytes!("../ui/js/views/display.js");
static VIEW_PERFORMANCE_JS: &[u8] = include_bytes!("../ui/js/views/performance.js");
static VIEW_SYSTEM_JS: &[u8] = include_bytes!("../ui/js/views/system.js");
static FONT_INTER: &[u8] = include_bytes!("../ui/assets/fonts/inter.woff2");
static FONT_MONO: &[u8] = include_bytes!("../ui/assets/fonts/jetbrainsmono.woff2");
static LOGO_PNG: &[u8] = include_bytes!("../ui/assets/logo.png");

fn get_embedded_asset(path: &str) -> Option<(&'static [u8], &'static str)> {
    let clean = path.trim_start_matches('/');
    let clean = if clean.is_empty() {
        "index.html"
    } else {
        clean
    };

    match clean {
        "index.html" => Some((INDEX_HTML, "text/html; charset=utf-8")),
        "dist/app.css" => Some((APP_CSS, "text/css; charset=utf-8")),
        "css/tokens.css" => Some((TOKENS_CSS, "text/css; charset=utf-8")),
        "css/base.css" => Some((BASE_CSS, "text/css; charset=utf-8")),
        "css/components.css" => Some((COMPONENTS_CSS, "text/css; charset=utf-8")),
        "js/main.js" => Some((MAIN_JS, "application/javascript; charset=utf-8")),
        "js/ipc.js" => Some((IPC_JS, "application/javascript; charset=utf-8")),
        "js/store.js" => Some((STORE_JS, "application/javascript; charset=utf-8")),
        "js/bind.js" => Some((BIND_JS, "application/javascript; charset=utf-8")),
        "js/views/style.js" => Some((VIEW_STYLE_JS, "application/javascript; charset=utf-8")),
        "js/views/shortcuts.js" => {
            Some((VIEW_SHORTCUTS_JS, "application/javascript; charset=utf-8"))
        }
        "js/views/animation.js" => {
            Some((VIEW_ANIMATION_JS, "application/javascript; charset=utf-8"))
        }
        "js/views/windows.js" => Some((VIEW_WINDOWS_JS, "application/javascript; charset=utf-8")),
        "js/views/background.js" => {
            Some((VIEW_BACKGROUND_JS, "application/javascript; charset=utf-8"))
        }
        "js/views/display.js" => Some((VIEW_DISPLAY_JS, "application/javascript; charset=utf-8")),
        "js/views/performance.js" => {
            Some((VIEW_PERFORMANCE_JS, "application/javascript; charset=utf-8"))
        }
        "js/views/system.js" => Some((VIEW_SYSTEM_JS, "application/javascript; charset=utf-8")),
        "assets/fonts/inter.woff2" => Some((FONT_INTER, "font/woff2")),
        "assets/fonts/jetbrainsmono.woff2" => Some((FONT_MONO, "font/woff2")),
        "assets/logo.png" => Some((LOGO_PNG, "image/png")),
        _ => None,
    }
}

pub fn protocol_handler(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();

    // In dev mode (FLIP3D_UI_DEV=1), try reading from disk first
    if std::env::var("FLIP3D_UI_DEV").as_deref() == Ok("1") {
        let clean = path.trim_start_matches('/');
        let clean = if clean.is_empty() {
            "index.html"
        } else {
            clean
        };
        let disk_path = std::path::Path::new("crates/settings-ui/ui").join(clean);
        if let Ok(bytes) = std::fs::read(&disk_path) {
            let mime = mime_guess_from_path(clean);
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Cow::Owned(bytes))
                .unwrap_or_else(|_| not_found_response());
        }
    }

    if let Some((bytes, mime)) = get_embedded_asset(path) {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(Cow::Borrowed(bytes))
            .unwrap_or_else(|_| not_found_response())
    } else {
        not_found_response()
    }
}

fn mime_guess_from_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

fn not_found_response() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Cow::Borrowed(b"404 Not Found" as &'static [u8]))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_asset_resolution() {
        let req_root = Request::builder()
            .uri("app://localhost/")
            .body(Vec::new())
            .unwrap();
        let resp_root = protocol_handler(req_root);
        assert_eq!(resp_root.status(), StatusCode::OK);
        assert_eq!(
            resp_root.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(!resp_root.body().is_empty());

        let req_css = Request::builder()
            .uri("app://localhost/dist/app.css")
            .body(Vec::new())
            .unwrap();
        let resp_css = protocol_handler(req_css);
        assert_eq!(resp_css.status(), StatusCode::OK);
        assert_eq!(
            resp_css.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );

        let req_font = Request::builder()
            .uri("app://localhost/assets/fonts/inter.woff2")
            .body(Vec::new())
            .unwrap();
        let resp_font = protocol_handler(req_font);
        assert_eq!(resp_font.status(), StatusCode::OK);
        assert_eq!(
            resp_font.headers().get(header::CONTENT_TYPE).unwrap(),
            "font/woff2"
        );

        let req_404 = Request::builder()
            .uri("app://localhost/nonexistent.file")
            .body(Vec::new())
            .unwrap();
        let resp_404 = protocol_handler(req_404);
        assert_eq!(resp_404.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_zero_external_network_dependencies() {
        let assets = [
            ("index.html", INDEX_HTML),
            ("dist/app.css", APP_CSS),
            ("css/tokens.css", TOKENS_CSS),
            ("css/base.css", BASE_CSS),
            ("css/components.css", COMPONENTS_CSS),
            ("js/main.js", MAIN_JS),
            ("js/ipc.js", IPC_JS),
            ("js/store.js", STORE_JS),
            ("js/bind.js", BIND_JS),
            ("js/views/style.js", VIEW_STYLE_JS),
            ("js/views/shortcuts.js", VIEW_SHORTCUTS_JS),
            ("js/views/animation.js", VIEW_ANIMATION_JS),
            ("js/views/windows.js", VIEW_WINDOWS_JS),
            ("js/views/background.js", VIEW_BACKGROUND_JS),
            ("js/views/display.js", VIEW_DISPLAY_JS),
            ("js/views/performance.js", VIEW_PERFORMANCE_JS),
            ("js/views/system.js", VIEW_SYSTEM_JS),
        ];

        for (name, content) in assets {
            let text = String::from_utf8_lossy(content);
            for line in text.lines() {
                // Filter out XML namespaces and license comments
                if line.contains("http://www.w3.org/2000/svg") || line.contains("tailwindcss.com") {
                    continue;
                }
                assert!(
                    !line.contains("http://") && !line.contains("https://"),
                    "File '{}' contains external URL reference: {}",
                    name,
                    line.trim()
                );
            }
        }
    }
}
