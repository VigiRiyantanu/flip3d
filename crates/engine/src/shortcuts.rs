// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering};
use windows::Win32::Foundation::{HWND, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP};

use crate::settings::{Settings, Shortcut, SHORTCUT_COUNT};

pub const WM_APP_START: u32 = WM_APP + 1;
pub const WM_APP_STEP: u32 = WM_APP + 2;
pub const WM_APP_WHEEL: u32 = WM_APP + 3;
pub const WM_APP_COMMIT: u32 = WM_APP + 4;
pub const WM_APP_CANCEL: u32 = WM_APP + 5;
pub const WM_APP_SETTINGS: u32 = WM_APP + 6;
pub const WM_APP_REFRESH_ICONS: u32 = WM_APP + 7;
pub const WM_APP_MOVE_DEVICE: u32 = WM_APP + 8;

pub const WM_HOOK_ENGAGE: u32 = WM_APP + 20;
pub const WM_HOOK_DISENGAGE: u32 = WM_APP + 21;
pub const WM_HOOK_UPDATE: u32 = WM_APP + 22;

pub const START_BACKWARDS: usize = 1;
pub const START_FROM_HOTKEY: usize = 2;
pub const START_SHORTCUT_SHIFT: usize = 8;

pub const INJECTED_INPUT_TAG: usize = 0x46334453;
pub const DUMMY_VK: u16 = 0xE8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HoldKey {
    Alt = 0,
    Win = 1,
    None = 2,
}

impl From<u8> for HoldKey {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Alt,
            1 => Self::Win,
            _ => Self::None,
        }
    }
}

pub static G_READY: AtomicBool = AtomicBool::new(false);
pub static G_TAKE_OVER: AtomicBool = AtomicBool::new(false);
pub static G_SWITCHING: AtomicBool = AtomicBool::new(false);
pub static G_HOLD_KEY: AtomicU8 = AtomicU8::new(HoldKey::Alt as u8);
pub static G_SHORTCUTS: AtomicU32 = AtomicU32::new(1 << (Shortcut::AltTab as usize));
pub static G_SESSION_SHORTCUT: AtomicU8 = AtomicU8::new(Shortcut::AltTab as u8);

pub static G_OVERLAY_WND: AtomicIsize = AtomicIsize::new(0);
pub static G_UI_THREAD_ID: AtomicU32 = AtomicU32::new(0);
pub static G_HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
pub static G_HOTKEY_MESSAGE: AtomicU32 = AtomicU32::new(0);

pub fn get_overlay_hwnd() -> HWND {
    HWND(G_OVERLAY_WND.load(Ordering::SeqCst) as *mut _)
}

pub fn set_overlay_hwnd(hwnd: HWND) {
    G_OVERLAY_WND.store(hwnd.0 as isize, Ordering::SeqCst);
}

pub fn apply_shortcut_settings(settings: &Settings) {
    let mut shortcuts = 0u32;
    for i in 0..SHORTCUT_COUNT {
        if settings.shortcut_on[i] {
            shortcuts |= 1 << i;
        }
    }
    G_SHORTCUTS.store(shortcuts, Ordering::SeqCst);
}

pub fn match_shortcut(alt: bool, ctrl: bool, win: bool, shortcut: &mut Shortcut) -> bool {
    if alt == win {
        return false;
    }
    if alt {
        *shortcut = if ctrl {
            Shortcut::CtrlAltTab
        } else {
            Shortcut::AltTab
        };
    } else {
        *shortcut = if ctrl {
            Shortcut::WinCtrlTab
        } else {
            Shortcut::WinTab
        };
    }
    let mask = 1u32 << (*shortcut as usize);
    (G_SHORTCUTS.load(Ordering::SeqCst) & mask) != 0
}

pub fn hold_key_for(shortcut: Shortcut) -> HoldKey {
    match shortcut {
        Shortcut::AltTab => HoldKey::Alt,
        Shortcut::WinTab => HoldKey::Win,
        _ => HoldKey::None,
    }
}

pub fn end_switching() -> bool {
    if !G_SWITCHING.swap(false, Ordering::SeqCst) {
        return false;
    }
    let hook_thread_id = G_HOOK_THREAD_ID.load(Ordering::SeqCst);
    if hook_thread_id != 0 {
        // SAFETY: PostThreadMessageW delivers thread message to hook thread queue.
        unsafe {
            let _ = PostThreadMessageW(
                hook_thread_id,
                WM_HOOK_DISENGAGE,
                WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
    }
    true
}
