# Alt+Tab Flip 3D (Vista Style) for Windows 10 & 11

A standalone, portable Windows 10 & 11 window switcher recreating the classic Windows Vista / Windows 7 **Flip 3D** experience, plus 22 additional switch styles, written in pure Rust with DirectX 11, Direct2D, DirectWrite, and Windows Graphics Capture.

> **Credits:** Original Windhawk mod (`alt-tab-flip-3d-fork`, v1.7.1) by **caliberda** (MIT License).  
> Ported to standalone portable Rust by **Antigravity**.

---

## Highlights

- **Zero Dependencies & Portable**: Single standalone executable `flip3d.exe`. No Windhawk, no code injection, no DLL injection into `explorer.exe`, and no installers required. The single binary contains both the background switcher and the glass WebView2 settings panel (`flip3d.exe --settings`).
- **24 Switcher Styles + Native**:
  1. `Flip3D` (Vista classic)
  2. `Cascade` (Windows 7 aero peek cascade)
  3. `CoverFlow` (classic Apple CoverFlow with reflections)
  4. `ITab` (isometric carousel with floor reflection & light)
  5. `ITabTitle`
  6. `ITabBackground`
  7. `ITabBackgroundTitle`
  8. `Carousel` (circular 3D orbit)
  9. `Grid` (clean responsive grid)
  10. `Helix` (3D DNA spiral)
  11. `Fan` (semi-circular radial fan)
  12. `Panorama` (cylindrical panorama view)
  13. `Tunnel` (depth tunnel view)
  14. `Rolodex` (classic desk card index)
  15. `Cube` (rotating 3D cube)
  16. `Sphere` (spherical orbital layout)
  17. `Shuffle` (card slide & shuffle)
  18. `Domino` (domino toppling effect)
  19. `Windows11` (modern fluid card layout)
  20. `Thumbnails` (flat live window preview panel)
  21. `IconRow` (dock-style icon strip)
  22. `List` (detailed window list with titles)
  23. `Classic` (classic Windows 2000 / XP 2D grid)
  24. `Native` (pass-through to Windows built-in switcher)
- **Extreme Fidelity**: Preserves all original spring physics, layout formulas, camera projection, timing parameters (`kCloseAnimationSeconds = 0.22`, `omega = 15.0 * flipSpeed`, `kStep = 1.0/480`, `kMaxLatitude = 1.15`), and animations.
- **Hardware-Accelerated Rendering**: Direct3D 11 & Direct2D composite pipeline with DirectWrite typography, 3D transform effect chains, gaussian blur, shadow mapping, and high-resolution waitable timer frame pacing (`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`).
- **Live Windows Graphics Capture (WGC)** with graceful GDI `PrintWindow(PW_RENDERFULLCONTENT)` fallback.
- **Interactive Tray & Glass Settings UI**: System tray icon with context menu (Settings, Pause, Run as administrator, Start with Windows, About, Exit) and glass-style WebView2 settings panel with real OS Acrylic / Mica blur, 8 tabs, and 100% offline embedded assets.
- **Clean Codebase**: 100% warning-free build, passing strict `cargo clippy --all-targets -- -D warnings`.

---

## Compatibility Table: Windows 10 vs. Windows 11

| Feature | Windows 10 (1903 / Build 18362+) | Windows 11 (Build 22000+) | Notes |
| :--- | :--- | :--- | :--- |
| **DirectX 11 / Direct2D** | Full Support | Full Support | Hardware-accelerated 3D effects & swapchain. |
| **DirectWrite Typography** | Falls back to **Segoe UI** | Uses **Segoe UI Variable Display / Text** | Automatic family detection & fallback. |
| **Graphics Capture (WGC)** | Supported (1903+) | Supported (Borderless Mode) | On Win10 without borderless API, overlay window covers real windows. |
| **PrintWindow Fallback** | Full Support | Full Support | Used when WGC access is denied or window is non-capturable. |
| **Rounded Corners (`DWMWCP`)**| Silently ignored | `DWMWCP_DONOTROUND` | Prevents DWM corner rounding artifacts on overlay. |
| **High-Res Waitable Timer** | Supported | Supported | `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` with `DwmFlush` fallback. |
| **Virtual Desktops** | Supported | Supported | Uses `IVirtualDesktopManager` COM interface. |
| **Multi-GPU / Multi-Monitor**| Supported | Supported | Dynamic adapter migration when moving across monitors. |

---

## The Elevated Window Limitation & Solutions

On Windows, standard security isolation (User Interface Privilege Isolation, UIPI) prevents an unelevated low-level keyboard hook (`WH_KEYBOARD_LL`) from monitoring keystrokes when the active foreground window is elevated (running as Administrator).

Since this standalone version runs as a normal portable executable without injecting into `explorer.exe`:

1. **"Run as administrator"**: Right-click the tray icon and select "Run as administrator". This relaunches the switcher with elevated privileges (`ShellExecuteW` with `runas`), allowing Alt+Tab to work seamlessly over Task Manager, Registry Editor, and elevated terminals.
2. **Elevated Autostart with Windows**: In Settings / Tray Menu, selecting "Start with Windows (Elevated)" registers a scheduled task via `schtasks.exe` with `HighestAvailable` run level. It automatically runs elevated at logon without displaying a UAC prompt.
3. **RegisterHotKey Path**: For shortcuts using the Windows key (e.g. Win+Ctrl+Tab), Flip 3D registers a Win32 Hotkey via `RegisterHotKey`, which continues to work even across integrity boundaries for supported combinations.

---

## Building from Source

### Prerequisites

- **Rust**: Stable toolchain (Rust 2021 edition or newer), target `x86_64-pc-windows-msvc`. (Also supports `x86_64-pc-windows-gnu`).
- **Visual Studio C++ Build Tools** (for MSVC linker and Windows SDK).

### Build Commands

```powershell
# 1. Run unit tests verifying geometry & all 23 styles
cargo test --all-targets

# 2. Verify strict compiler and clippy cleanliness
cargo clippy --all-targets -- -D warnings

# 3. Compile optimized release executable
cargo build --release
```

The resulting executable is located at:
`target\release\flip3d.exe`

---

## Configuration & Settings

Settings are stored in JSON format at:
`%APPDATA%\Flip3D\config.json`

Log output is written with rotation at:
`%LOCALAPPDATA%\Flip3D\logs\flip3d.log`
`%LOCALAPPDATA%\Flip3D\logs\settings-ui.log`

### Default Configuration (`config.toml`)

```toml
style = "flip3d"
monitor = "cursor"
shortcut = "altTab"
win_tab_style = "same"
sticky_shortcut = true
animation_duration = 420
flip_speed = 100
show_delay = 90
tilt_angle = 30
stack_spacing = 100
window_size = 100
depth = 100
max_windows = 40
background = "wallpaper"
blur_radius = 50
dim_opacity = 0.5
close_button = true
show_title = true
title_position = "bottom"
live_content = "selected"
shadows = true
```

---

## License & Attribution

- **License:** MIT License (see [LICENSE](file:///LICENSE)).
- **Original Mod:** [Alt+Tab Flip 3D (Vista Style)](https://windhawk.net/mods/alt-tab-flip-3d) for Windhawk by **caliberda**.
- Ported with 1:1 mathematical, numeric, and behavioral fidelity to Rust by **Antigravity**.
