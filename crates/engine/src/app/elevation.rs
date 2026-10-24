// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn is_elevated() -> bool {
    // SAFETY: Query process token for elevation information.
    unsafe {
        let mut token = HANDLE(std::ptr::null_mut());
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        );

        let _ = CloseHandle(token);
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

pub fn relaunch_elevated() -> Result<()> {
    let current_exe = std::env::current_exe()
        .map_err(|e| windows::core::Error::new(windows::core::HRESULT(-1), e.to_string()))?;

    let exe_w: Vec<u16> = format!("{}\0", current_exe.display())
        .encode_utf16()
        .collect();
    let runas_w: Vec<u16> = "runas\0".encode_utf16().collect();

    // SAFETY: ShellExecuteW with "runas" verb triggers UAC elevation prompt.
    unsafe {
        let res = ShellExecuteW(
            None,
            PCWSTR::from_raw(runas_w.as_ptr()),
            PCWSTR::from_raw(exe_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        if (res.0 as isize) > 32 {
            // Success, exit unelevated process
            std::process::exit(0);
        } else {
            Err(windows::core::Error::from_thread())
        }
    }
}
