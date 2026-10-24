// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use tao::platform::windows::WindowExtWindows;
use tao::window::Window;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, REG_DWORD,
};

#[derive(Debug, Clone)]
pub struct BlurInfo {
    pub blur_type: &'static str, // "mica", "acrylic", "solid"
    pub status_text: String,
    pub is_active: bool,
}

pub fn is_os_transparency_enabled() -> bool {
    let subkey_w: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
        .encode_utf16()
        .collect();
    let val_w: Vec<u16> = "EnableTransparency\0".encode_utf16().collect();
    let mut key = HKEY::default();

    unsafe {
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            Some(0),
            KEY_READ,
            &mut key,
        )
        .is_ok()
        {
            let mut val: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;
            let mut typ = REG_DWORD;
            let res = RegQueryValueExW(
                key,
                PCWSTR::from_raw(val_w.as_ptr()),
                None,
                Some(&mut typ),
                Some(&mut val as *mut u32 as *mut _),
                Some(&mut size),
            );
            let _ = RegCloseKey(key);
            if res.is_ok() {
                return val != 0;
            }
        }
    }
    true // default to true if key is absent
}

pub fn apply_window_effects(window: &Window, build: u32) -> BlurInfo {
    // Enable system shadow on frameless window
    window.set_undecorated_shadow(true);

    // Apply Windows 11 rounded corners via DwmSetWindowAttribute
    if build >= 22000 {
        let hwnd = HWND(window.hwnd() as *mut _);
        let preference = DWMWCP_ROUND.0;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    if !is_os_transparency_enabled() {
        return BlurInfo {
            blur_type: "solid",
            status_text: "Disabled (OS Transparency off in Windows Settings)".to_string(),
            is_active: false,
        };
    }

    if build >= 22000 {
        if window_vibrancy::apply_mica(window, None).is_ok() {
            return BlurInfo {
                blur_type: "mica",
                status_text: "Active (Windows 11 Mica)".to_string(),
                is_active: true,
            };
        } else if window_vibrancy::apply_acrylic(window, Some((250, 249, 245, 120))).is_ok() {
            return BlurInfo {
                blur_type: "acrylic",
                status_text: "Active (Windows 11 Acrylic)".to_string(),
                is_active: true,
            };
        }
    } else if build >= 17063
        && window_vibrancy::apply_acrylic(window, Some((250, 249, 245, 120))).is_ok()
    {
        return BlurInfo {
            blur_type: "acrylic",
            status_text: "Active (Windows 10 Acrylic)".to_string(),
            is_active: true,
        };
    }

    BlurInfo {
        blur_type: "solid",
        status_text: "Solid ivory fallback (Acrylic/Mica unavailable)".to_string(),
        is_active: false,
    }
}
