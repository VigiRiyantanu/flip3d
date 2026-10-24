// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::cell::Cell;
use std::panic::catch_unwind;
use std::sync::atomic::Ordering;

use windows::Win32::Foundation::{HANDLE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
    VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PeekMessageW, PostMessageW, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, LLKHF_INJECTED,
    MSG, MSLLHOOKSTRUCT, PM_NOREMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_MOUSEWHEEL,
    WM_SYSKEYDOWN,
};

use crate::shortcuts::{
    end_switching, get_overlay_hwnd, hold_key_for, match_shortcut, DUMMY_VK, G_HOLD_KEY, G_READY,
    G_SESSION_SHORTCUT, G_SWITCHING, G_TAKE_OVER, INJECTED_INPUT_TAG, START_BACKWARDS,
    WM_APP_CANCEL, WM_APP_COMMIT, WM_APP_START, WM_APP_STEP, WM_APP_WHEEL, WM_HOOK_DISENGAGE,
    WM_HOOK_ENGAGE, WM_HOOK_UPDATE,
};

const LEFT_CTRL_BIT: i32 = 1;
const RIGHT_CTRL_BIT: i32 = 2;
const LEFT_WIN_BIT: i32 = 4;
const RIGHT_WIN_BIT: i32 = 8;

thread_local! {
    static PHYSICAL_MODIFIERS: Cell<i32> = const { Cell::new(0) };
    static SWALLOWED_PRESS: Cell<[bool; 256]> = const { Cell::new([false; 256]) };
    static KEYBOARD_HOOK: Cell<Option<HHOOK>> = const { Cell::new(None) };
    static MOUSE_HOOK: Cell<Option<HHOOK>> = const { Cell::new(None) };
}

pub fn is_key_down(vk: i32) -> bool {
    // SAFETY: GetAsyncKeyState is a safe Win32 API to query physical/async key state.
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

pub fn send_dummy_key_press() {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(DUMMY_VK),
                    wScan: 0,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: INJECTED_INPUT_TAG,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(DUMMY_VK),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: INJECTED_INPUT_TAG,
                },
            },
        },
    ];

    // SAFETY: SendInput called with 2 valid keyboard inputs tagged with INJECTED_INPUT_TAG.
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn modifier_bit(vk: u32) -> i32 {
    match vk {
        v if v == VK_LCONTROL.0 as u32 => LEFT_CTRL_BIT,
        v if v == VK_RCONTROL.0 as u32 => RIGHT_CTRL_BIT,
        v if v == VK_LWIN.0 as u32 => LEFT_WIN_BIT,
        v if v == VK_RWIN.0 as u32 => RIGHT_WIN_BIT,
        _ => 0,
    }
}

fn reset_physical_modifiers() {
    let mut bits = 0;
    for vk in [
        VK_LCONTROL.0 as i32,
        VK_RCONTROL.0 as i32,
        VK_LWIN.0 as i32,
        VK_RWIN.0 as i32,
    ] {
        if is_key_down(vk) {
            bits |= modifier_bit(vk as u32);
        }
    }
    PHYSICAL_MODIFIERS.with(|cell| cell.set(bits));
}

fn ctrl_held() -> bool {
    let bits = PHYSICAL_MODIFIERS.with(|cell| cell.get());
    (bits & (LEFT_CTRL_BIT | RIGHT_CTRL_BIT)) != 0 && is_key_down(VK_CONTROL.0 as i32)
}

fn win_held() -> bool {
    let bits = PHYSICAL_MODIFIERS.with(|cell| cell.get());
    (bits & (LEFT_WIN_BIT | RIGHT_WIN_BIT)) != 0
        && (is_key_down(VK_LWIN.0 as i32) || is_key_down(VK_RWIN.0 as i32))
}

/// # Safety
/// Low-level keyboard hook procedure invoked by Windows OS message pump.
pub unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let result = catch_unwind(|| {
        if code != 0 {
            // HC_ACTION = 0
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if kb.dwExtraInfo == INJECTED_INPUT_TAG {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let msg = wparam.0 as u32;
        let key_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let injected = (kb.flags.0 & LLKHF_INJECTED.0) != 0;
        let vk = kb.vkCode;

        let bit = modifier_bit(vk);
        if bit != 0 && !injected {
            PHYSICAL_MODIFIERS.with(|cell| {
                let current = cell.get();
                if key_down {
                    cell.set(current | bit);
                } else {
                    cell.set(current & !bit);
                }
            });
        }

        let switching = G_SWITCHING.load(Ordering::SeqCst);
        let overlay_wnd = get_overlay_hwnd();

        if !switching {
            let mut shortcut = crate::settings::Shortcut::AltTab;
            if key_down
                && vk == VK_TAB.0 as u32
                && G_READY.load(Ordering::SeqCst)
                && G_TAKE_OVER.load(Ordering::SeqCst)
                && match_shortcut(
                    (kb.flags.0 & LLKHF_ALTDOWN.0) != 0,
                    ctrl_held(),
                    win_held(),
                    &mut shortcut,
                )
            {
                G_SESSION_SHORTCUT.store(shortcut as u8, Ordering::SeqCst);
                G_HOLD_KEY.store(hold_key_for(shortcut) as u8, Ordering::SeqCst);
                G_SWITCHING.store(true, Ordering::SeqCst);

                let start_flags = if is_key_down(VK_SHIFT.0 as i32) {
                    START_BACKWARDS
                } else {
                    0
                };
                let _ = PostMessageW(
                    Some(overlay_wnd),
                    WM_APP_START,
                    WPARAM(start_flags),
                    LPARAM(0),
                );

                let hook_thread_id = crate::shortcuts::G_HOOK_THREAD_ID.load(Ordering::SeqCst);
                if hook_thread_id != 0 {
                    let _ =
                        PostThreadMessageW(hook_thread_id, WM_HOOK_ENGAGE, WPARAM(0), LPARAM(0));
                }

                SWALLOWED_PRESS.with(|cell| {
                    let mut arr = [false; 256];
                    if (vk as usize) < 256 {
                        arr[vk as usize] = true;
                    }
                    cell.set(arr);
                });
                return LRESULT(1);
            }

            if !key_down && (vk as usize) < 256 {
                SWALLOWED_PRESS.with(|cell| {
                    let mut arr = cell.get();
                    arr[vk as usize] = false;
                    cell.set(arr);
                });
            }

            return CallNextHookEx(None, code, wparam, lparam);
        }

        // Switching session active
        match vk {
            v if v == VK_MENU.0 as u32
                || v == VK_LMENU.0 as u32
                || v == VK_RMENU.0 as u32
                || v == VK_LWIN.0 as u32
                || v == VK_RWIN.0 as u32 =>
            {
                let key = if v == VK_LWIN.0 as u32 || v == VK_RWIN.0 as u32 {
                    crate::shortcuts::HoldKey::Win
                } else {
                    crate::shortcuts::HoldKey::Alt
                };

                let current_hold_key = G_HOLD_KEY.load(Ordering::SeqCst);
                if !key_down && !injected && current_hold_key == (key as u8) && end_switching() {
                    let _ = PostMessageW(Some(overlay_wnd), WM_APP_COMMIT, WPARAM(0), LPARAM(0));
                }
                return CallNextHookEx(None, code, wparam, lparam);
            }

            v if v == VK_SHIFT.0 as u32
                || v == VK_LSHIFT.0 as u32
                || v == VK_RSHIFT.0 as u32
                || v == VK_CONTROL.0 as u32
                || v == VK_LCONTROL.0 as u32
                || v == VK_RCONTROL.0 as u32 =>
            {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            _ => {}
        }

        if !key_down {
            let swallowed = SWALLOWED_PRESS.with(|cell| {
                let mut arr = cell.get();
                let was = if (vk as usize) < 256 {
                    arr[vk as usize]
                } else {
                    false
                };
                if (vk as usize) < 256 {
                    arr[vk as usize] = false;
                    cell.set(arr);
                }
                was
            });
            if !swallowed {
                return CallNextHookEx(None, code, wparam, lparam);
            }
            return LRESULT(1);
        }

        SWALLOWED_PRESS.with(|cell| {
            if (vk as usize) < 256 {
                let mut arr = cell.get();
                arr[vk as usize] = true;
                cell.set(arr);
            }
        });

        let mut step = 0isize;
        match vk {
            v if v == VK_TAB.0 as u32 => {
                step = if is_key_down(VK_SHIFT.0 as i32) {
                    -1
                } else {
                    1
                };
            }
            v if v == VK_RIGHT.0 as u32 || v == VK_DOWN.0 as u32 => {
                step = 1;
            }
            v if v == VK_LEFT.0 as u32 || v == VK_UP.0 as u32 => {
                step = -1;
            }
            v if v == VK_ESCAPE.0 as u32 => {
                if end_switching() {
                    let _ = PostMessageW(Some(overlay_wnd), WM_APP_CANCEL, WPARAM(0), LPARAM(0));
                }
            }
            v if (v == VK_RETURN.0 as u32 || v == VK_SPACE.0 as u32) && end_switching() => {
                let _ = PostMessageW(Some(overlay_wnd), WM_APP_COMMIT, WPARAM(0), LPARAM(0));
            }
            _ => {}
        }

        if step != 0 {
            let _ = PostMessageW(
                Some(overlay_wnd),
                WM_APP_STEP,
                WPARAM(step as usize),
                LPARAM(0),
            );
        }

        LRESULT(1)
    });

    match result {
        Ok(lresult) => lresult,
        Err(_) => {
            // SAFETY: CallNextHookEx on panic fallback.
            CallNextHookEx(None, code, wparam, lparam)
        }
    }
}

/// # Safety
/// Low-level mouse hook procedure invoked by Windows OS message pump.
pub unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let result = catch_unwind(|| {
        if code == 0 && wparam.0 as u32 == WM_MOUSEWHEEL && G_SWITCHING.load(Ordering::SeqCst) {
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if ms.dwExtraInfo != INJECTED_INPUT_TAG {
                let delta = ((ms.mouseData >> 16) & 0xffff) as i16;
                let overlay_wnd = get_overlay_hwnd();
                let _ = PostMessageW(
                    Some(overlay_wnd),
                    WM_APP_WHEEL,
                    WPARAM(delta as isize as usize),
                    LPARAM(0),
                );
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    });

    match result {
        Ok(lresult) => lresult,
        Err(_) => CallNextHookEx(None, code, wparam, lparam),
    }
}

pub fn update_keyboard_hook() {
    let take_over = G_TAKE_OVER.load(Ordering::SeqCst);
    KEYBOARD_HOOK.with(|cell| {
        let current = cell.get();
        if take_over && current.is_none() {
            reset_physical_modifiers();
            // SAFETY: Installing low-level keyboard hook with keyboard_proc.
            let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) };
            match hook {
                Ok(h) => {
                    cell.set(Some(h));
                    log::info!("Installed WH_KEYBOARD_LL");
                }
                Err(e) => {
                    log::error!("SetWindowsHookEx(WH_KEYBOARD_LL) failed: {:?}", e);
                }
            }
        } else if !take_over && current.is_some() {
            if let Some(h) = current {
                // SAFETY: UnhookWindowsHookEx on existing hook.
                unsafe {
                    let _ = UnhookWindowsHookEx(h);
                }
                cell.set(None);
                log::info!("Unhooked WH_KEYBOARD_LL");
            }
        }
    });
}

pub fn hook_thread_proc(start_event: HANDLE) -> u32 {
    crate::shortcuts::G_HOOK_THREAD_ID.store(
        // SAFETY: GetCurrentThreadId retrieves the OS thread id of this thread.
        unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
        Ordering::SeqCst,
    );

    let mut msg = MSG::default();
    // SAFETY: PeekMessageW forces message queue creation before signaling caller.
    unsafe {
        let _ = PeekMessageW(
            &mut msg,
            None,
            windows::Win32::UI::WindowsAndMessaging::WM_USER,
            windows::Win32::UI::WindowsAndMessaging::WM_USER,
            PM_NOREMOVE,
        );
    }

    update_keyboard_hook();

    // SAFETY: SetEvent signals creator that hook thread message queue is ready.
    unsafe {
        let _ = windows::Win32::System::Threading::SetEvent(start_event);
    }

    // Message loop
    loop {
        // SAFETY: GetMessageW blocks until a message is posted to this thread queue.
        let status = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if status.0 <= 0 {
            break;
        }

        if !msg.hwnd.0.is_null() {
            // SAFETY: DispatchMessageW dispatches window messages.
            unsafe {
                DispatchMessageW(&msg);
            }
            continue;
        }

        match msg.message {
            WM_HOOK_UPDATE => {
                update_keyboard_hook();
            }

            WM_HOOK_ENGAGE => {
                if msg.wParam.0 != 0 {
                    SWALLOWED_PRESS.with(|cell| {
                        cell.set([false; 256]);
                    });
                }
                send_dummy_key_press();

                MOUSE_HOOK.with(|cell| {
                    if cell.get().is_none() {
                        // SAFETY: SetWindowsHookExW for WH_MOUSE_LL.
                        let hook =
                            unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) };
                        match hook {
                            Ok(h) => cell.set(Some(h)),
                            Err(e) => log::error!("SetWindowsHookEx(WH_MOUSE_LL) failed: {:?}", e),
                        }
                    }
                });
            }

            WM_HOOK_DISENGAGE => {
                MOUSE_HOOK.with(|cell| {
                    if let Some(h) = cell.get() {
                        if !G_SWITCHING.load(Ordering::SeqCst) {
                            // SAFETY: UnhookWindowsHookEx for mouse hook.
                            unsafe {
                                let _ = UnhookWindowsHookEx(h);
                            }
                            cell.set(None);
                        }
                    }
                });
            }

            _ => {}
        }
    }

    MOUSE_HOOK.with(|cell| {
        if let Some(h) = cell.get() {
            // SAFETY: Clean unhook on shutdown.
            unsafe {
                let _ = UnhookWindowsHookEx(h);
            }
            cell.set(None);
        }
    });

    KEYBOARD_HOOK.with(|cell| {
        if let Some(h) = cell.get() {
            // SAFETY: Clean unhook on shutdown.
            unsafe {
                let _ = UnhookWindowsHookEx(h);
            }
            cell.set(None);
        }
    });

    0
}
