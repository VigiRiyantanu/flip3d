// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

pub mod logging;
pub mod model;
pub mod paths;
pub mod persistence;
pub mod registry;
pub mod signal;

pub use logging::*;
pub use model::*;
pub use paths::*;
pub use persistence::*;
pub use registry::*;
pub use signal::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_all_24_styles_registered() {
        assert_eq!(ALL_STYLES.len(), 24);
        for s in ALL_STYLES {
            assert!(s.supported);
            assert!(!s.id.is_empty());
            assert!(!s.display_name.is_empty());
            assert!(find_style(s.id).is_some());
        }
    }

    #[test]
    fn test_validation_and_clamping() {
        let mut cfg = Config {
            version: 99,
            style: "invalid_style".to_string(),
            win_tab_style: "unknown".to_string(),
            monitor: "bad_monitor".to_string(),
            shortcut: "bad_shortcut".to_string(),
            sticky_shortcut: true,
            extra_shortcuts: Vec::new(),
            animation_duration: 99999, // clamped to 3000
            flip_speed: 10,            // clamped to 25
            show_delay: -50,           // clamped to 0
            tilt_angle: 120,           // clamped to 70
            stack_spacing: 10,         // clamped to 40
            window_size: 500,          // clamped to 200
            depth: -20,                // clamped to 0
            max_windows: 100,          // clamped to 40
            background: "bad_bg".to_string(),
            blur_amount: 500, // clamped to 100
            dim_opacity: 200, // clamped to 90
            show_title: false,
            title_position: "nowhere".to_string(),
            shadows: true,
            include_minimized: false,
            minimized_content: false,
            virtual_desktops: "invalid".to_string(),
            manager_hidden_windows: false,
            live_content: "invalid".to_string(),
            frame_rate: "screen".to_string(),
            anti_aliasing: "invalid_aa".to_string(),
            middle_click_close: false,
            close_button: false,
            hide_bars: false,
            excluded_apps: vec!["  C:\\Program Files\\app.exe  ".to_string()],
        };

        cfg.validate_and_clamp();

        assert_eq!(cfg.version, CURRENT_CONFIG_VERSION);
        assert_eq!(cfg.style, "flip3d");
        assert_eq!(cfg.win_tab_style, "same");
        assert_eq!(cfg.monitor, "cursor");
        assert_eq!(cfg.shortcut, "altTab");
        assert_eq!(cfg.animation_duration, 3000);
        assert_eq!(cfg.flip_speed, 25);
        assert_eq!(cfg.show_delay, 0);
        assert_eq!(cfg.tilt_angle, 70);
        assert_eq!(cfg.stack_spacing, 40);
        assert_eq!(cfg.window_size, 200);
        assert_eq!(cfg.depth, 0);
        assert_eq!(cfg.max_windows, 40);
        assert_eq!(cfg.background, "blur");
        assert_eq!(cfg.blur_amount, 100);
        assert_eq!(cfg.dim_opacity, 90);
        assert_eq!(cfg.title_position, "bottom");
        assert_eq!(cfg.virtual_desktops, "windows");
        assert_eq!(cfg.live_content, "all");
        assert_eq!(cfg.anti_aliasing, "fxaa");
        assert_eq!(cfg.excluded_apps, vec!["app.exe"]);
    }

    #[test]
    fn test_engine_settings_conversion() {
        let cfg = Config::default();
        let settings = Settings::from_config(&cfg);
        assert_eq!(settings.style, AnimationStyle::Flip3D);
        assert_eq!(settings.alt_tab_style, AnimationStyle::Flip3D);
        assert!(settings.shortcut_on[Shortcut::AltTab as usize]);
        assert_eq!(settings.animation_duration_ms, 420);
        assert!((settings.flip_speed - 1.0).abs() < 1e-4);
        assert!((settings.stack_spacing - 1.0).abs() < 1e-4);
        assert_eq!(settings.max_windows, 20);
        assert_eq!(settings.background, BackgroundMode::BlurredWallpaper);
        assert_eq!(settings.anti_aliasing, AntiAliasingMode::Fxaa);
    }

    #[test]
    fn test_atomic_save_and_load_flow() {
        let test_dir = std::env::temp_dir().join(format!("flip3d_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&test_dir);
        let json_file = test_dir.join("config.json");

        let config = Config {
            style: "coverFlow".to_string(),
            flip_speed: 150,
            ..Default::default()
        };

        // Verify JSON round-trip
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&json_file, &json_str).unwrap();

        let loaded: Config =
            serde_json::from_str(&fs::read_to_string(&json_file).unwrap()).unwrap();
        assert_eq!(loaded.style, "coverFlow");
        assert_eq!(loaded.flip_speed, 150);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_toml_migration_parsing() {
        let legacy_toml = r#"
            style = "coverFlow"
            win_tab_style = "same"
            monitor = "cursor"
            shortcut = "both"
            sticky_shortcut = true
            animation_duration = 350
            flip_speed = 120
            show_delay = 50
            tilt_angle = 35
            stack_spacing = 90
            window_size = 110
            depth = 80
            max_windows = 15
            background = "wallpaper"
            blur_amount = 15
            dim_opacity = 30
            show_title = true
            title_position = "bottom"
            shadows = true
            include_minimized = true
            minimized_content = true
            virtual_desktops = "current"
            manager_hidden_windows = false
            live_content = "all"
            frame_rate = "screen"
            middle_click_close = true
            close_button = true
            hide_bars = true
            excluded_apps = ["test.exe"]
        "#;

        let mut migrated: Config = toml::from_str(legacy_toml).unwrap();
        migrated.validate_and_clamp();

        assert_eq!(migrated.style, "coverFlow");
        assert_eq!(migrated.shortcut, "both");
        assert_eq!(migrated.animation_duration, 350);
        assert_eq!(migrated.flip_speed, 120);
        assert_eq!(migrated.background, "wallpaper");
        assert_eq!(migrated.excluded_apps, vec!["test.exe"]);
    }
}
