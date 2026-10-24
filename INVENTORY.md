# Symbol & Function Inventory Mapping Table

This table maps every C++ constant, enum, struct, function, and method from `mod.wh.cpp` (v1.7.1 by caliberda) to its Rust equivalent in `flip3d`.

---

## 1. Constants & Numeric Values

| C++ Symbol | C++ Value | Rust File | Rust Counterpart | Status / Notes |
| :--- | :--- | :--- | :--- | :--- |
| `kMaxExtraShortcuts` | `20` | `settings.rs` | `MAX_EXTRA_SHORTCUTS = 20` | Exact fidelity |
| `kInjectedInputTag` | `0x46334453` (`'FS3D'`) | `shortcuts.rs` | `INJECTED_INPUT_TAG = 0x46334453` | Exact fidelity |
| `kDummyVk` | `0xE8` | `shortcuts.rs` | `DUMMY_VK = 0xE8` | Exact fidelity |
| `kCloseAnimationSeconds` | `0.22` | `layout.rs` | `CLOSE_ANIMATION_SECONDS = 0.22` | Exact fidelity |
| `kSlideAnimationSeconds` | `0.18` | `layout.rs` | `SLIDE_ANIMATION_SECONDS = 0.18` | Exact fidelity |
| `kDeviceRetryMs` | `1000` | `switcher.rs` | `DEVICE_RETRY_MS = 1000` | Exact fidelity |
| `kRestoreCheckMs` | `10` | `switcher.rs` | `RESTORE_CHECK_MS = 10` | Exact fidelity |
| `kMaxRestoreTries` | `4` | `switcher.rs` | `MAX_RESTORE_TRIES = 4` | Exact fidelity |
| `kHoldReleasedFallbackSeconds`| `0.15` | `switcher.rs` | `HOLD_RELEASED_FALLBACK_SECONDS = 0.15` | Exact fidelity |
| `kMinCloseButtonRadius` | `6.0` | `layout.rs` | `MIN_CLOSE_BUTTON_RADIUS = 6.0` | Exact fidelity |
| `kStep` | `1.0 / 480.0` | `switcher.rs` | `STEP = 1.0 / 480.0` | Exact fidelity |
| `omega` | `15.0 * flipSpeed` | `switcher.rs` | `omega = 15.0 * flip_speed` | Exact fidelity |
| `kMaxLatitude` | `1.15` | `layout.rs` | `MAX_LATITUDE = 1.15` | Exact fidelity |
| `WM_APP_REFRESH_SETTINGS` | `WM_APP + 1` | `shortcuts.rs` | `WM_APP_REFRESH_SETTINGS` | Exact fidelity |
| `WM_APP_MOVE_DEVICE` | `WM_APP + 2` | `shortcuts.rs` | `WM_APP_MOVE_DEVICE` | Exact fidelity |
| `WM_APP_REFRESH_ICONS` | `WM_APP + 3` | `shortcuts.rs` | `WM_APP_REFRESH_ICONS` | Exact fidelity |
| `WM_APP_START_SWITCHING` | `WM_APP + 4` | `shortcuts.rs` | `WM_APP_START_SWITCHING` | Exact fidelity |
| `WM_APP_END_SWITCHING` | `WM_APP + 5` | `shortcuts.rs` | `WM_APP_END_SWITCHING` | Exact fidelity |
| `WM_APP_STEP` | `WM_APP + 6` | `shortcuts.rs` | `WM_APP_STEP` | Exact fidelity |
| `WM_APP_ARROW` | `WM_APP + 7` | `shortcuts.rs` | `WM_APP_ARROW` | Exact fidelity |
| `WM_APP_WHEEL` | `WM_APP + 8` | `shortcuts.rs` | `WM_APP_WHEEL` | Exact fidelity |

---

## 2. Enums

| C++ Enum | Rust Enum | Rust File | Notes |
| :--- | :--- | :--- | :--- |
| `AnimationStyle` (24 variants) | `AnimationStyle` | `settings.rs` | All 23 styles + Native |
| `MonitorMode` | `MonitorMode` | `settings.rs` | `Cursor`, `ActiveWindow` |
| `Shortcut` | `Shortcut` | `settings.rs` | `AltTab`, `WinTab`, `WinCtrlTab`, `CtrlAltTab` |
| `BackgroundMode` | `BackgroundMode` | `settings.rs` | `Wallpaper`, `LiveDesktop`, `None` |
| `TitlePosition` | `TitlePosition` | `settings.rs` | `Top`, `Bottom`, `BelowWindow` |
| `LiveContent` | `LiveContent` | `settings.rs` | `All`, `Selected`, `None` |
| `HoldKey` | `HoldKey` | `shortcuts.rs` | `Alt`, `Win`, `Ctrl` |
| `State` | `State` | `switcher.rs` | `Idle`, `Pending`, `Open`, `Closing` |

---

## 3. Structs

| C++ Struct | Rust Struct | Rust File | Notes |
| :--- | :--- | :--- | :--- |
| `Pose` | `Pose` | `geometry.rs` | Position, scale, angles, opacity, brightness |
| `Quad` | `Quad` | `geometry.rs` | 4 corner points for 3D projection |
| `Tween` | `Tween` | `geometry.rs` | Interpolator between start and target |
| `LayoutMetrics` | `LayoutMetrics` | `layout.rs` | Layout box metrics, step distances, focal |
| `CellLayout` | `CellLayout` | `layout.rs` | Panel mode cell bounds |
| `PanelLayout` | `PanelLayout` | `layout.rs` | Grid / IconRow / Classic panel metrics |
| `CloseButton` | `CloseButton` | `layout.rs` | Close X button geometry and hit testing |
| `DrawEntry` | `DrawEntry` | `layout.rs` | Sorted render item descriptor |
| `FrameKey` | `FrameKey` | `switcher.rs` | Deduplication key for static frames |
| `Item` | `Item` | `capture.rs` | Window content, icon, WGC session, fallback |
| `EffectChain` | `EffectChain` | `render.rs` | Transform3D + ColorMatrix effect pair |
| `WindowFilter` | `WindowFilter` | `windows_list.rs` | Cloaked, tool window, desktop filter rules |
| `Settings` | `Settings` | `settings.rs` | Clamped user configuration |

---

## 4. Mathematics & Geometry Functions

| C++ Function | Rust Function | Rust File | Fidelity / Notes |
| :--- | :--- | :--- | :--- |
| `PositiveMod(float)` | `positive_mod_f32` | `geometry.rs` | Exact formula `(val % m + m) % m` |
| `PositiveMod(int)` | `positive_mod_i64` | `geometry.rs` | Exact formula |
| `EaseOutQuart` | `ease_out_quart` | `geometry.rs` | `1.0 - (1.0 - t)^4` |
| `EaseInOutCubic` | `ease_in_out_cubic` | `geometry.rs` | `4 * t^3` / `1 - (-2*t + 2)^3 / 2` |
| `Lerp` | `lerp` | `geometry.rs` | `a + (b - a) * t` |
| `Lerp(Pose)` | `lerp_pose` | `geometry.rs` | Interpolates all pose parameters |
| `QuadContains` | `quad_contains` | `geometry.rs` | 2D point-in-convex-polygon cross products |
| `RectToQuadMatrix` | `rect_to_quad_matrix` | `geometry.rs` | Perspective 4x4 matrix from quad corners |
| `ComputeLayoutMetrics` | `compute_layout_metrics`| `layout.rs` | Computes box sizes for all 23 styles |
| `ProjectPose` | `project_pose` | `layout.rs` | 3D camera projection to screen Quad |
| `LayoutPose` | `layout_pose` | `layout.rs` | Exact mathematical formulas for all 23 styles |
| `BuildDrawList` | `build_draw_list` | `layout.rs` | Depth sorting, wrap chains, closing items |

---

## 5. Switcher Methods (`Switcher` in `src/switcher.rs`)

| C++ Method | Rust Method | Status / Implementation |
| :--- | :--- | :--- |
| `Switcher::CreateOverlay` | `Switcher::create_overlay` | Native Win32 window creation |
| `Switcher::Activate` | `Switcher::activate` | Enters pending state, starts capture |
| `Switcher::RecreateDevice` | `Switcher::recreate_device` | D3D11 / D2D device loss recovery |
| `Switcher::MoveDeviceToMonitor`| `Switcher::move_device_to_monitor` | Multi-GPU adapter migration |
| `Switcher::HandleDeviceLost` | `Switcher::handle_device_lost` | Full teardown and re-creation |
| `Switcher::UpdateFrameRate` | `Switcher::update_frame_rate` | Reads monitor refresh rate via GDI/DWM |
| `Switcher::Step` | `Switcher::step` | Relative movement along carousel |
| `Switcher::OnArrow` | `Switcher::on_arrow` | Grid/spatial navigation (Up/Down/Left/Right) |
| `Switcher::OnClick` | `Switcher::on_click` | Mouse click window activation |
| `Switcher::OnMiddleClick` | `Switcher::on_middle_click`| Mouse middle-click close |
| `Switcher::RequestClose` | `Switcher::request_close` | Sends `WM_SYSCOMMAND / SC_CLOSE` |
| `Switcher::RemoveClosedItems` | `Switcher::remove_closed_items` | Animates closed window slide-out |
| `Switcher::ActivateWindow` | `Switcher::activate_window` | Dummy-key trick + `SetForegroundWindow` |
| `Switcher::CheckRestored` | `Switcher::check_restored` | Retry timer for minimized windows |
| `Switcher::BeginClose` | `Switcher::begin_close` | Initiates 0.22s close tween |
| `Switcher::FinishClose` | `Switcher::finish_close` | Restores window, resets to Idle |
| `Switcher::RenderFrame` | `Switcher::render_frame` | Full Direct2D composite pass |
| `Switcher::DrawCard` | `draw_card` | Renders individual card with 3D effects |
| `Switcher::DrawReflections` | `draw_reflections` | iTab floor reflections and fade brush |
| `Switcher::DrawFloor` | `draw_floor` | iTab black floor and radial light |
| `Switcher::DrawCaption` | `Switcher::draw_caption` | iTab scroll bar and titles |
| `Switcher::DrawTitle` | `Switcher::draw_title` | Active title and app name display |
| `Switcher::DrawPanelFront` | `Switcher::draw_panel_front` | 2D Panel / Classic / IconRow rendering |
| `Switcher::DrawPanelBack` | `Switcher::draw_panel_back` | Panel backdrop and highlight |
| `Switcher::DrawCloseButton` | `Switcher::draw_close_button`| Hover close "X" circle |

---

## 6. Removed Windhawk-Specific Components (Section 5)

| Removed C++ Component | Replacement in Standalone Rust |
| :--- | :--- |
| `Wh_ModInit`, `Wh_ModAfterInit` | Standard `main()` initialization with single-instance mutex |
| `Wh_ModSettingsChanged` | Settings watcher & live-reload via `config.toml` |
| `Wh_ModUninit` | Clean `WM_QUIT` signal handling and drop implementations |
| `WhTool_*` / Process Launcher | Direct executable lifecycle |
| Explorer Thread Injection / Hooks | Pure low-level hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`) |
| `kOverlayTitleReady/Idle` Protocol | Atomic `G_READY` flag |
| `windhawk_utils.h` settings | `serde` + `toml` deserialization with exact clamps |
