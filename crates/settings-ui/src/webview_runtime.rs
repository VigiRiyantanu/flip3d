// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OKCANCEL};

#[repr(C)]
struct OsVersionInfoW {
    os_version_info_size: u32,
    major_version: u32,
    minor_version: u32,
    build_number: u32,
    platform_id: u32,
    sz_csd_version: [u16; 128],
}

/// Retrieve exact Windows version and build number via RtlGetVersion (bypassing manifest lies).
pub fn get_windows_version() -> (u32, u32, u32) {
    unsafe {
        let ntdll_name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
        let ntdll = GetModuleHandleW(PCWSTR::from_raw(ntdll_name.as_ptr()));
        if let Ok(h) = ntdll {
            let proc = GetProcAddress(h, windows::core::s!("RtlGetVersion"));
            if let Some(rtl_get_version) = proc {
                let func: extern "system" fn(*mut OsVersionInfoW) -> i32 =
                    std::mem::transmute(rtl_get_version);
                let mut info = OsVersionInfoW {
                    os_version_info_size: std::mem::size_of::<OsVersionInfoW>() as u32,
                    major_version: 0,
                    minor_version: 0,
                    build_number: 0,
                    platform_id: 0,
                    sz_csd_version: [0; 128],
                };
                if func(&mut info) == 0 {
                    return (info.major_version, info.minor_version, info.build_number);
                }
            }
        }
    }
    (10, 0, 19041)
}

/// Query registry for installed Edge WebView2 Runtime version.
pub fn get_webview2_version() -> Option<String> {
    let subkeys = [
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\0",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\0",
        ),
        (
            HKEY_CURRENT_USER,
            "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\0",
        ),
    ];

    for (root, subkey) in subkeys {
        let subkey_w: Vec<u16> = subkey.encode_utf16().collect();
        let mut key = HKEY::default();
        unsafe {
            if RegOpenKeyExW(
                root,
                PCWSTR::from_raw(subkey_w.as_ptr()),
                Some(0),
                KEY_READ,
                &mut key,
            )
            .is_ok()
            {
                let val_name: Vec<u16> = "pv\0".encode_utf16().collect();
                let mut buf = [0u16; 128];
                let mut size = (buf.len() * 2) as u32;
                let mut typ = REG_SZ;
                if RegQueryValueExW(
                    key,
                    PCWSTR::from_raw(val_name.as_ptr()),
                    None,
                    Some(&mut typ),
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut size),
                )
                .is_ok()
                {
                    let len = (size as usize / 2).saturating_sub(1);
                    let version = String::from_utf16_lossy(&buf[..len]);
                    let _ = RegCloseKey(key);
                    if !version.is_empty() && version != "0.0.0.0" {
                        return Some(version);
                    }
                }
                let _ = RegCloseKey(key);
            }
        }
    }
    None
}

/// Returns the dedicated user data folder for WebView2.
pub fn get_user_data_dir() -> PathBuf {
    let dir = settings_core::AppPaths::get().webview_dir().to_path_buf();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Ensure WebView2 Runtime is available, showing a native dialog if missing.
pub fn ensure_webview2_runtime() -> bool {
    if let Some(ver) = get_webview2_version() {
        log::info!("WebView2 Runtime detected: {}", ver);
        return true;
    }

    log::error!("WebView2 Runtime not found on the system.");

    let title_w: Vec<u16> = "Flip 3D - WebView2 Runtime Required\0"
        .encode_utf16()
        .collect();
    let msg_w: Vec<u16> =
        ("Microsoft Edge WebView2 Runtime is required to display the Flip 3D settings panel.\n\n\
        Click OK to open the official download page, or Cancel to exit.")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

    let result = unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(msg_w.as_ptr()),
            PCWSTR::from_raw(title_w.as_ptr()),
            MB_ICONERROR | MB_OKCANCEL,
        )
    };

    // IDOK is 1
    if result.0 == 1 {
        let _ = std::process::Command::new("explorer.exe")
            .arg("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
            .spawn();
    }

    false
}
