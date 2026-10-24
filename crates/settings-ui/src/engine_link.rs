// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_VALUE_NAME: &str = "Flip3D";

pub fn is_engine_running() -> bool {
    settings_core::is_engine_running()
}

pub fn notify_settings_changed() {
    settings_core::notify_settings_changed();
}

pub fn find_engine_exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

pub fn is_autostart_enabled() -> bool {
    let subkey_w: Vec<u16> = format!("{}\0", RUN_KEY).encode_utf16().collect();
    let val_w: Vec<u16> = format!("{}\0", APP_VALUE_NAME).encode_utf16().collect();

    unsafe {
        let mut key = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            Some(0),
            KEY_READ,
            &mut key,
        )
        .is_err()
        {
            return false;
        }

        let mut size = 0u32;
        let res = RegQueryValueExW(
            key,
            PCWSTR::from_raw(val_w.as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        );
        let _ = RegCloseKey(key);
        res.is_ok() && size > 0
    }
}

pub fn set_autostart_enabled(enable: bool) -> Result<(), String> {
    let subkey_w: Vec<u16> = format!("{}\0", RUN_KEY).encode_utf16().collect();
    let val_w: Vec<u16> = format!("{}\0", APP_VALUE_NAME).encode_utf16().collect();

    unsafe {
        let mut key = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            Some(0),
            KEY_WRITE,
            &mut key,
        )
        .is_err()
        {
            return Err("Failed to open HKCU Run registry key for writing".to_string());
        }

        if enable {
            let exe_path = find_engine_exe().or_else(|| std::env::current_exe().ok());
            match exe_path {
                Some(path) => {
                    let formatted = format!("\"{}\"\0", path.display());
                    let data_w: Vec<u16> = formatted.encode_utf16().collect();
                    let byte_slice = std::slice::from_raw_parts(
                        data_w.as_ptr() as *const u8,
                        data_w.len() * std::mem::size_of::<u16>(),
                    );
                    let res = RegSetValueExW(
                        key,
                        PCWSTR::from_raw(val_w.as_ptr()),
                        Some(0),
                        REG_SZ,
                        Some(byte_slice),
                    );
                    let _ = RegCloseKey(key);
                    if res.is_ok() {
                        Ok(())
                    } else {
                        Err(format!("RegSetValueExW failed: {:?}", res))
                    }
                }
                None => {
                    let _ = RegCloseKey(key);
                    Err("Could not determine executable path for autostart".to_string())
                }
            }
        } else {
            let res = RegDeleteValueW(key, PCWSTR::from_raw(val_w.as_ptr()));
            let _ = RegCloseKey(key);
            // It's fine if the value didn't exist
            if res.is_ok() || res.0 == 2 {
                // ERROR_FILE_NOT_FOUND is 2
                Ok(())
            } else {
                Err(format!("RegDeleteValueW failed: {:?}", res))
            }
        }
    }
}
