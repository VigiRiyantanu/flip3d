// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
use windows::core::{Result, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_VALUE_NAME: &str = "Flip3D";
const TASK_NAME: &str = "Flip3D";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutostartMode {
    Disabled,
    Normal,
    Elevated,
}

pub fn get_autostart_mode() -> AutostartMode {
    // Check Task Scheduler elevated task first
    if is_task_scheduled() {
        return AutostartMode::Elevated;
    }
    // Check HKCU Run key
    if is_registry_autostart() {
        return AutostartMode::Normal;
    }
    AutostartMode::Disabled
}

pub fn is_registry_autostart() -> bool {
    let subkey_w: Vec<u16> = format!("{}\0", RUN_KEY).encode_utf16().collect();
    let val_w: Vec<u16> = format!("{}\0", APP_VALUE_NAME).encode_utf16().collect();

    // SAFETY: Read HKCU Run key.
    unsafe {
        let mut key = windows::Win32::System::Registry::HKEY::default();
        if !RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
        .is_ok()
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

pub fn set_registry_autostart(enable: bool) -> Result<()> {
    let subkey_w: Vec<u16> = format!("{}\0", RUN_KEY).encode_utf16().collect();
    let val_w: Vec<u16> = format!("{}\0", APP_VALUE_NAME).encode_utf16().collect();

    // SAFETY: Open and update HKCU Run registry value.
    unsafe {
        let mut key = windows::Win32::System::Registry::HKEY::default();
        if !RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            None,
            KEY_WRITE,
            &mut key,
        )
        .is_ok()
        {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(-1),
                "Failed to open registry key",
            ));
        }

        if enable {
            if let Ok(exe_path) = std::env::current_exe() {
                let formatted = format!("\"{}\"\0", exe_path.display());
                let data_w: Vec<u16> = formatted.encode_utf16().collect();
                let byte_slice = std::slice::from_raw_parts(
                    data_w.as_ptr() as *const u8,
                    data_w.len() * std::mem::size_of::<u16>(),
                );
                let _ = RegSetValueExW(
                    key,
                    PCWSTR::from_raw(val_w.as_ptr()),
                    None,
                    REG_SZ,
                    Some(byte_slice),
                );
            }
        } else {
            let _ = RegDeleteValueW(key, PCWSTR::from_raw(val_w.as_ptr()));
        }

        let _ = RegCloseKey(key);
    }
    Ok(())
}

pub fn is_task_scheduled() -> bool {
    let mut cmd = Command::new("schtasks.exe");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd.args(["/query", "/tn", TASK_NAME]).output();
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

pub fn set_task_scheduled(enable: bool) -> Result<()> {
    if enable {
        let exe_path = std::env::current_exe()
            .map_err(|e| windows::core::Error::new(windows::core::HRESULT(-1), e.to_string()))?;
        let tr_arg = format!("\"{}\"", exe_path.display());

        let mut cmd = Command::new("schtasks.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd
            .args([
                "/create", "/tn", TASK_NAME, "/tr", &tr_arg, "/sc", "onlogon", "/rl", "highest",
                "/f",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => {
                let err_str = String::from_utf8_lossy(&o.stderr);
                log::warn!("schtasks failed: {}", err_str);
                Err(windows::core::Error::new(
                    windows::core::HRESULT(-1),
                    &err_str,
                ))
            }
            Err(e) => Err(windows::core::Error::new(
                windows::core::HRESULT(-1),
                e.to_string(),
            )),
        }
    } else {
        let mut cmd = Command::new("schtasks.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.args(["/delete", "/tn", TASK_NAME, "/f"]).output();
        Ok(())
    }
}

pub fn set_autostart_mode(mode: AutostartMode) -> Result<()> {
    match mode {
        AutostartMode::Disabled => {
            let _ = set_registry_autostart(false);
            let _ = set_task_scheduled(false);
        }
        AutostartMode::Normal => {
            let _ = set_task_scheduled(false);
            set_registry_autostart(true)?;
        }
        AutostartMode::Elevated => {
            let _ = set_registry_autostart(false);
            set_task_scheduled(true)?;
        }
    }
    Ok(())
}

/// In portable mode, if HKCU Run contains Flip3D and the path differs from current_exe(),
/// update it quietly so moving the portable folder doesn't break autostart.
pub fn repair_portable_autostart_if_needed() {
    let app_paths = settings_core::AppPaths::get();
    if app_paths.mode != settings_core::DistributionMode::Portable {
        return;
    }

    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };

    let subkey_w: Vec<u16> = format!("{}\0", RUN_KEY).encode_utf16().collect();
    let val_w: Vec<u16> = format!("{}\0", APP_VALUE_NAME).encode_utf16().collect();

    unsafe {
        let mut key = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(subkey_w.as_ptr()),
            None,
            KEY_READ | KEY_WRITE,
            &mut key,
        )
        .is_ok()
        {
            let mut buf = [0u16; 512];
            let mut size = (buf.len() * 2) as u32;
            let mut typ = REG_SZ;
            if RegQueryValueExW(
                key,
                PCWSTR::from_raw(val_w.as_ptr()),
                None,
                Some(&mut typ),
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
            .is_ok()
                && size > 2
            {
                let len = (size as usize / 2).saturating_sub(1);
                let current_reg_val = String::from_utf16_lossy(&buf[..len]);
                let trimmed = current_reg_val.trim_matches('"');
                if trimmed != current_exe.to_string_lossy().as_ref() {
                    log::info!(
                        "Portable autostart path mismatch. Updating registry from {:?} to {:?}",
                        trimmed,
                        current_exe
                    );
                    let formatted = format!("\"{}\"\0", current_exe.display());
                    let data_w: Vec<u16> = formatted.encode_utf16().collect();
                    let byte_slice = std::slice::from_raw_parts(
                        data_w.as_ptr() as *const u8,
                        data_w.len() * std::mem::size_of::<u16>(),
                    );
                    let _ = RegSetValueExW(
                        key,
                        PCWSTR::from_raw(val_w.as_ptr()),
                        None,
                        REG_SZ,
                        Some(byte_slice),
                    );
                }
            }
            let _ = RegCloseKey(key);
        }
    }
}
