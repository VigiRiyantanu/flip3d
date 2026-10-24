# Notes & Observations on "Alt+Tab Flip 3D (Vista Style)" Port

This document details nuances, quirks, and exact behaviors faithfully ported from the original C++ Windhawk mod (`mod.wh.cpp` by caliberda) into the standalone Rust implementation.

---

## 1. Key Behaviors & Quirks Preserved

### 1.1 The Dummy Virtual Key (`VK 0xE8` / `0x46334453`) Trick
- **The Issue**: On Windows, when an overlay window or hook thread tries to set the foreground window via `SetForegroundWindow(hwnd)`, Windows may deny the request if the calling process does not currently have input focus (error code `ERROR_ACCESS_DENIED`).
- **The Solution**: The mod injects a dummy key event with `VK_NONAME = 0xE8` and attaches a signature tag `dwExtraInfo = 0x46334453` (`'FS3D'`). This generates genuine user input in the Windows subsystem, immediately granting the process permission to change the foreground window.
- **Ported in**: `src/hooks.rs:send_dummy_key_press()` and `src/switcher.rs:send_dummy_key_press()`.

### 1.2 Foreground-First Ordering with Circular Rotate
- When enumerating switchable windows using `EnumWindows`, the active foreground window is often located in the middle of the z-order if other tool windows or topmost windows exist.
- The mod finds the current foreground window in the enumerated list and performs a circular rotation (`std::rotate` in C++) so that the foreground window is at index 0, and the previous foreground window is at index 1 (allowing quick Alt+Tab back-and-forth).
- **Ported in**: `src/windows_list.rs:collect_windows()`.

### 1.3 Two Effect Chains per Card (Wrap-Around Continuity)
- Stacked styles (Flip 3D, Cascade, CoverFlow) support infinite scrolling in both directions. When the carousel wraps around or interpolates across the front/back boundary, drawing a single instance of a card would cause sudden popping or clipping artifacts.
- The mod equips every `Item` with two independent Direct2D effect chains (`Chain[0]` and `Chain[1]`). When a card wraps around from the back of the stack to the front, the second effect chain renders its continuous duplicate with an offset pose.
- **Ported in**: `src/switcher.rs:Item.chains` and `src/render.rs:draw_card()`.

### 1.4 Minimized Window Restore Verification Loop
- Some minimized applications (e.g. legacy MDI apps, custom Win32 apps) do not restore immediately when `ShowWindowAsync(hwnd, SW_RESTORE)` is called.
- The mod implements a 4-try retry timer (`RESTORE_TIMER_ID`, check interval 10 ms, up to 4 retries) which verifies via `IsIconic(hwnd)` whether the window was successfully unminimized. On the first retry, it sends `WM_SYSCOMMAND / SC_RESTORE`, and subsequent retries repeat `ShowWindowAsync`.
- **Ported in**: `src/switcher.rs:check_restored()`.

### 1.5 Quick-Tap Instant Switch (`showDelay`)
- When the user presses and quickly releases Alt+Tab in under `showDelay` milliseconds (default 90 ms), the mod transitions into `State::Pending` and does not display the overlay window at all. It immediately restores the previous window, preventing visual flash and latency.
- If Alt is held longer than `showDelay`, the overlay window animates into view smoothly.
- **Ported in**: `src/switcher.rs:activate()` and `render_frame()`.

### 1.6 Middle-Click and Close Button ("X") Handling
- Hovering over a window in the switcher reveals a close "X" circle in the top right corner.
- Middle-clicking or left-clicking the close circle sends `WM_SYSCOMMAND / SC_CLOSE` to the target window.
- The item begins a slide animation (`RemoveClosedItems`), sliding to the side over 0.22 seconds, smoothly adjusting the carousel target and scroll offset without jumping.
- **Ported in**: `src/switcher.rs:remove_closed_items()`.

---

## 2. Windows 10 & 11 Specific Porting Insights

1. **Direct2D Effect Graph in windows-rs 0.62**:
   - `ID2D1Effect` objects are connected into graphs by obtaining their output image via `effect.GetOutput()?` and passing that `ID2D1Image` to `ctx.DrawImage()` or `next_effect.SetInput()`.
2. **WinRT Free-Threaded Frame Pool**:
   - Windows Graphics Capture frame pool callbacks are processed on a dedicated background worker thread and synchronized with Direct3D11 multithread protection (`SetMultithreadProtected(true)`).
3. **High-Resolution Waitable Timers**:
   - High-refresh rate displays (120Hz, 144Hz, 240Hz) are timed accurately using `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`. When running on older Windows builds where high-resolution waitable timers are unsupported, it falls back to `DwmFlush` and `MsgWaitForMultipleObjects`.

---

## 3. Suggestions for Future Work (Non-Breaking)

1. **Per-Application Exclusions**: Allow users to add specific process names (e.g. games running in exclusive fullscreen) to an ignore list in `config.toml`.
2. **Custom Shaders**: Direct3D11 compute/pixel shaders for custom glass / acrylic / mica backdrop effects.
3. **Touch Gestures**: Support multi-finger swipe gestures on precision touchpads for switching cards.
