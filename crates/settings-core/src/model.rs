// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

pub const CURRENT_CONFIG_VERSION: u32 = 1;
pub const SHORTCUT_COUNT: usize = 4;
pub const MAX_EXTRA_SHORTCUTS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationStyle {
    Flip3D,
    Cascade,
    CoverFlow,
    ITab,
    ITabTitle,
    ITabBackground,
    ITabBackgroundTitle,
    Carousel,
    Grid,
    Helix,
    Fan,
    Panorama,
    Tunnel,
    Rolodex,
    Cube,
    Sphere,
    Shuffle,
    Domino,
    Windows11,
    Thumbnails,
    IconRow,
    List,
    Classic,
    Native,
}

impl AnimationStyle {
    pub fn from_str_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "flip3d" => Some(Self::Flip3D),
            "cascade" => Some(Self::Cascade),
            "coverflow" => Some(Self::CoverFlow),
            "itab" => Some(Self::ITab),
            "itabtitle" => Some(Self::ITabTitle),
            "itabbackground" => Some(Self::ITabBackground),
            "itabbackgroundtitle" => Some(Self::ITabBackgroundTitle),
            "carousel" => Some(Self::Carousel),
            "grid" => Some(Self::Grid),
            "helix" => Some(Self::Helix),
            "fan" => Some(Self::Fan),
            "panorama" => Some(Self::Panorama),
            "tunnel" => Some(Self::Tunnel),
            "rolodex" => Some(Self::Rolodex),
            "cube" => Some(Self::Cube),
            "sphere" => Some(Self::Sphere),
            "shuffle" => Some(Self::Shuffle),
            "domino" => Some(Self::Domino),
            "windows11" => Some(Self::Windows11),
            "thumbnails" => Some(Self::Thumbnails),
            "iconrow" | "icons" => Some(Self::IconRow),
            "list" => Some(Self::List),
            "classic" => Some(Self::Classic),
            "native" => Some(Self::Native),
            _ => None,
        }
    }

    pub fn to_id_str(self) -> &'static str {
        match self {
            Self::Flip3D => "flip3d",
            Self::Cascade => "cascade",
            Self::CoverFlow => "coverFlow",
            Self::ITab => "itab",
            Self::ITabTitle => "itabTitle",
            Self::ITabBackground => "itabBackground",
            Self::ITabBackgroundTitle => "itabBackgroundTitle",
            Self::Carousel => "carousel",
            Self::Grid => "grid",
            Self::Helix => "helix",
            Self::Fan => "fan",
            Self::Panorama => "panorama",
            Self::Tunnel => "tunnel",
            Self::Rolodex => "rolodex",
            Self::Cube => "cube",
            Self::Sphere => "sphere",
            Self::Shuffle => "shuffle",
            Self::Domino => "domino",
            Self::Windows11 => "windows11",
            Self::Thumbnails => "thumbnails",
            Self::IconRow => "icons",
            Self::List => "list",
            Self::Classic => "classic",
            Self::Native => "native",
        }
    }

    pub fn to_display_name(self) -> &'static str {
        match self {
            Self::Flip3D => "Flip 3D (Vista stack)",
            Self::Cascade => "Cascade",
            Self::CoverFlow => "Cover Flow",
            Self::ITab => "iTab (Black floor, app name)",
            Self::ITabTitle => "iTab (Black floor, title only)",
            Self::ITabBackground => "iTab (Wallpaper floor, app name)",
            Self::ITabBackgroundTitle => "iTab (Wallpaper floor, title only)",
            Self::Carousel => "Carousel",
            Self::Grid => "Grid (Mission Control)",
            Self::Helix => "Helix",
            Self::Fan => "Fan",
            Self::Panorama => "Panorama",
            Self::Tunnel => "Tunnel",
            Self::Rolodex => "Rolodex",
            Self::Cube => "Cube",
            Self::Sphere => "Sphere",
            Self::Shuffle => "Shuffle",
            Self::Domino => "Domino",
            Self::Windows11 => "Windows 11",
            Self::Thumbnails => "Thumbnails",
            Self::IconRow => "Icons and titles",
            Self::List => "Vertical list",
            Self::Classic => "Classic (Windows XP)",
            Self::Native => "Native Windows Alt+Tab",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MonitorMode {
    Cursor,
    ActiveWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundMode {
    BlurredWallpaper,
    Wallpaper,
    LiveDesktop,
    Dim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TitlePosition {
    Bottom,
    BelowWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveContent {
    All,
    Selected,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VirtualDesktops {
    SameAsWindows,
    Current,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    AltTab = 0,
    WinTab = 1,
    CtrlAltTab = 2,
    WinCtrlTab = 3,
}

impl From<u8> for Shortcut {
    fn from(val: u8) -> Self {
        match val {
            1 => Self::WinTab,
            2 => Self::CtrlAltTab,
            3 => Self::WinCtrlTab,
            _ => Self::AltTab,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtraShortcutConfig {
    pub shortcut: String, // "winCtrlTab" or "ctrlAltTab"
    pub style: String,    // "same" or style id
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub style: String,
    pub win_tab_style: String,
    pub monitor: String,
    pub shortcut: String,
    pub sticky_shortcut: bool,
    pub extra_shortcuts: Vec<ExtraShortcutConfig>,
    pub animation_duration: i32,
    pub flip_speed: i32,
    pub show_delay: i32,
    pub tilt_angle: i32,
    pub stack_spacing: i32,
    pub window_size: i32,
    pub depth: i32,
    pub max_windows: i32,
    pub background: String,
    pub blur_amount: i32,
    pub dim_opacity: i32,
    pub show_title: bool,
    pub title_position: String,
    pub shadows: bool,
    pub include_minimized: bool,
    pub minimized_content: bool,
    pub virtual_desktops: String,
    pub manager_hidden_windows: bool,
    pub live_content: String,
    pub frame_rate: String,
    pub middle_click_close: bool,
    pub close_button: bool,
    pub hide_bars: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            style: "flip3d".to_string(),
            win_tab_style: "same".to_string(),
            monitor: "cursor".to_string(),
            shortcut: "altTab".to_string(),
            sticky_shortcut: true,
            extra_shortcuts: Vec::new(),
            animation_duration: 420,
            flip_speed: 100,
            show_delay: 90,
            tilt_angle: 30,
            stack_spacing: 100,
            window_size: 100,
            depth: 100,
            max_windows: 20,
            background: "blur".to_string(),
            blur_amount: 20,
            dim_opacity: 25,
            show_title: true,
            title_position: "bottom".to_string(),
            shadows: true,
            include_minimized: true,
            minimized_content: true,
            virtual_desktops: "windows".to_string(),
            manager_hidden_windows: false,
            live_content: "all".to_string(),
            frame_rate: "screen".to_string(),
            middle_click_close: true,
            close_button: true,
            hide_bars: true,
            excluded_apps: vec!["yasb.exe".to_string(), "rainmeter.exe".to_string()],
        }
    }
}

impl Config {
    pub fn validate_and_clamp(&mut self) {
        self.version = CURRENT_CONFIG_VERSION;

        // Validate style
        if AnimationStyle::from_str_name(&self.style).is_none() {
            self.style = "flip3d".to_string();
        }

        // Validate win_tab_style
        if !self.win_tab_style.eq_ignore_ascii_case("same")
            && AnimationStyle::from_str_name(&self.win_tab_style).is_none()
        {
            self.win_tab_style = "same".to_string();
        }

        // Validate monitor
        if !self.monitor.eq_ignore_ascii_case("cursor")
            && !self.monitor.eq_ignore_ascii_case("activeWindow")
        {
            self.monitor = "cursor".to_string();
        }

        // Validate shortcut
        if !self.shortcut.eq_ignore_ascii_case("altTab")
            && !self.shortcut.eq_ignore_ascii_case("winTab")
            && !self.shortcut.eq_ignore_ascii_case("both")
        {
            self.shortcut = "altTab".to_string();
        }

        // Extra shortcuts: max 20, validate shortcut identifier
        if self.extra_shortcuts.len() > MAX_EXTRA_SHORTCUTS {
            self.extra_shortcuts.truncate(MAX_EXTRA_SHORTCUTS);
        }
        for entry in &mut self.extra_shortcuts {
            if entry.shortcut != "winCtrlTab" && entry.shortcut != "ctrlAltTab" {
                entry.shortcut = "ctrlAltTab".to_string();
            }
            if !entry.style.eq_ignore_ascii_case("same")
                && AnimationStyle::from_str_name(&entry.style).is_none()
            {
                entry.style = "same".to_string();
            }
        }

        // Numeric clamping
        self.animation_duration = self.animation_duration.clamp(50, 3000);
        self.flip_speed = self.flip_speed.clamp(25, 400);
        self.show_delay = self.show_delay.clamp(0, 1000);
        self.tilt_angle = self.tilt_angle.clamp(0, 70);
        self.stack_spacing = self.stack_spacing.clamp(40, 250);
        self.window_size = self.window_size.clamp(50, 200);
        self.depth = self.depth.clamp(0, 300);
        self.max_windows = self.max_windows.clamp(2, 40);

        // Background
        match self.background.to_ascii_lowercase().as_str() {
            "wallpaper" | "desktop" | "dim" | "blur" => {}
            _ => self.background = "blur".to_string(),
        }

        self.blur_amount = self.blur_amount.clamp(1, 100);
        self.dim_opacity = self.dim_opacity.clamp(0, 90);

        // Title position
        if !self.title_position.eq_ignore_ascii_case("bottom")
            && !self.title_position.eq_ignore_ascii_case("belowWindow")
        {
            self.title_position = "bottom".to_string();
        }

        // Virtual desktops
        match self.virtual_desktops.to_ascii_lowercase().as_str() {
            "current" | "all" | "windows" => {}
            _ => self.virtual_desktops = "windows".to_string(),
        }

        // Live content
        match self.live_content.to_ascii_lowercase().as_str() {
            "selected" | "none" | "all" => {}
            _ => self.live_content = "all".to_string(),
        }

        // Excluded apps sanitization
        let mut clean_apps = Vec::new();
        for item in &self.excluded_apps {
            let mut name = item.trim().trim_matches('"').to_string();
            if let Some(pos) = name.rfind(['\\', '/']) {
                name = name[pos + 1..].to_string();
            }
            let name = name.trim().trim_matches('"').to_lowercase();
            if !name.is_empty() && !clean_apps.contains(&name) {
                clean_apps.push(name);
            }
        }
        self.excluded_apps = clean_apps;
    }
}

pub type RawConfigFile = Config;

#[derive(Debug, Clone)]
pub struct Settings {
    pub style: AnimationStyle,
    pub alt_tab_style: AnimationStyle,
    pub shortcut_on: [bool; SHORTCUT_COUNT],
    pub shortcut_style: [AnimationStyle; SHORTCUT_COUNT],
    pub monitor: MonitorMode,
    pub animation_duration_ms: i32,
    pub flip_speed: f32,
    pub show_delay_ms: i32,
    pub tilt_radians: f32,
    pub stack_spacing: f32,
    pub window_size: f32,
    pub depth: f32,
    pub max_windows: i32,
    pub background: BackgroundMode,
    pub blur_amount: f32,
    pub dim_opacity: f32,
    pub show_title: bool,
    pub title_position: TitlePosition,
    pub shadows: bool,
    pub include_minimized: bool,
    pub minimized_content: bool,
    pub virtual_desktops: VirtualDesktops,
    pub manager_hidden_windows: bool,
    pub live_content: LiveContent,
    pub max_fps: i32,
    pub fps_above_screen: i32,
    pub middle_click_close: bool,
    pub close_button: bool,
    pub hide_bars: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_config(&Config::default())
    }
}

impl Settings {
    pub fn from_config(config: &Config) -> Self {
        let style_of = |val: &str, fallback: AnimationStyle| -> AnimationStyle {
            if val.eq_ignore_ascii_case("same") {
                return fallback;
            }
            AnimationStyle::from_str_name(val).unwrap_or(fallback)
        };

        let alt_tab_style = style_of(&config.style, AnimationStyle::Flip3D);
        let mut win_tab_style = style_of(&config.win_tab_style, alt_tab_style);
        if win_tab_style == AnimationStyle::Native {
            win_tab_style = alt_tab_style;
        }

        let monitor = if config.monitor.eq_ignore_ascii_case("activeWindow") {
            MonitorMode::ActiveWindow
        } else {
            MonitorMode::Cursor
        };

        let alt_tab = !config.shortcut.eq_ignore_ascii_case("winTab");
        let win_tab = config.shortcut.eq_ignore_ascii_case("winTab")
            || config.shortcut.eq_ignore_ascii_case("both");
        let ctrl_alt_tab = alt_tab && config.sticky_shortcut;

        let mut shortcut_on = [false; SHORTCUT_COUNT];
        shortcut_on[Shortcut::AltTab as usize] = alt_tab;
        shortcut_on[Shortcut::WinTab as usize] = win_tab;
        shortcut_on[Shortcut::CtrlAltTab as usize] = ctrl_alt_tab;

        let mut shortcut_style = [alt_tab_style; SHORTCUT_COUNT];
        for (i, style) in shortcut_style.iter_mut().enumerate() {
            let is_win = i == Shortcut::WinTab as usize || i == Shortcut::WinCtrlTab as usize;
            *style = if is_win { win_tab_style } else { alt_tab_style };
        }

        for entry in config.extra_shortcuts.iter().take(MAX_EXTRA_SHORTCUTS) {
            let sc_opt = match entry.shortcut.as_str() {
                "winCtrlTab" => Some(Shortcut::WinCtrlTab),
                "ctrlAltTab" => Some(Shortcut::CtrlAltTab),
                _ => None,
            };
            if let Some(sc) = sc_opt {
                let idx = sc as usize;
                shortcut_on[idx] = true;
                let s = style_of(&entry.style, shortcut_style[idx]);
                if s != AnimationStyle::Native {
                    shortcut_style[idx] = s;
                }
            }
        }

        let animation_duration_ms = config.animation_duration.clamp(50, 3000);
        let flip_speed = config.flip_speed.clamp(25, 400) as f32 / 100.0;
        let show_delay_ms = config.show_delay.clamp(0, 1000);
        let tilt_radians = config.tilt_angle.clamp(0, 70) as f32 * PI / 180.0;
        let stack_spacing = config.stack_spacing.clamp(40, 250) as f32 / 100.0;
        let window_size = config.window_size.clamp(50, 200) as f32 / 100.0;
        let depth = config.depth.clamp(0, 300) as f32 / 100.0;
        let max_windows = config.max_windows.clamp(2, 40);

        let background = match config.background.to_ascii_lowercase().as_str() {
            "wallpaper" => BackgroundMode::Wallpaper,
            "desktop" => BackgroundMode::LiveDesktop,
            "dim" => BackgroundMode::Dim,
            _ => BackgroundMode::BlurredWallpaper,
        };

        let blur_amount = config.blur_amount.clamp(1, 100) as f32;
        let dim_opacity = config.dim_opacity.clamp(0, 90) as f32 / 100.0;
        let show_title = config.show_title;
        let title_position = if config.title_position.eq_ignore_ascii_case("belowWindow") {
            TitlePosition::BelowWindow
        } else {
            TitlePosition::Bottom
        };

        let shadows = config.shadows;
        let include_minimized = config.include_minimized;
        let minimized_content = config.minimized_content;

        let virtual_desktops = match config.virtual_desktops.to_ascii_lowercase().as_str() {
            "current" => VirtualDesktops::Current,
            "all" => VirtualDesktops::All,
            _ => VirtualDesktops::SameAsWindows,
        };

        let manager_hidden_windows = config.manager_hidden_windows;
        let live_content = match config.live_content.to_ascii_lowercase().as_str() {
            "selected" => LiveContent::Selected,
            "none" => LiveContent::None,
            _ => LiveContent::All,
        };

        let max_fps = config.frame_rate.parse::<i32>().unwrap_or(0).clamp(0, 240);
        let fps_above_screen = if config.frame_rate.eq_ignore_ascii_case("screenPlus20") {
            20
        } else {
            0
        };

        let middle_click_close = config.middle_click_close;
        let close_button = config.close_button;
        let hide_bars = config.hide_bars;

        Self {
            style: alt_tab_style,
            alt_tab_style,
            shortcut_on,
            shortcut_style,
            monitor,
            animation_duration_ms,
            flip_speed,
            show_delay_ms,
            tilt_radians,
            stack_spacing,
            window_size,
            depth,
            max_windows,
            background,
            blur_amount,
            dim_opacity,
            show_title,
            title_position,
            shadows,
            include_minimized,
            minimized_content,
            virtual_desktops,
            manager_hidden_windows,
            live_content,
            max_fps,
            fps_above_screen,
            middle_click_close,
            close_button,
            hide_bars,
            excluded_apps: config.excluded_apps.clone(),
        }
    }
}
