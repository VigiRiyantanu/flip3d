// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    CreateEventW, OpenEventW, OpenMutexW, SetEvent, EVENT_MODIFY_STATE, SYNCHRONIZATION_SYNCHRONIZE,
};

pub const ENGINE_MUTEX_NAME: &str = "Local\\Flip3D_Engine\0";
pub const SETTINGS_UI_MUTEX_NAME: &str = "Local\\Flip3D_SettingsUI\0";
pub const SETTINGS_CHANGED_EVENT_NAME: &str = "Local\\Flip3D_SettingsChanged\0";

/// Check if the engine background instance is currently running in this user session.
pub fn is_engine_running() -> bool {
    let name_w: Vec<u16> = ENGINE_MUTEX_NAME.encode_utf16().collect();
    // SAFETY: OpenMutexW checks for existing named mutex.
    unsafe {
        match OpenMutexW(
            SYNCHRONIZATION_SYNCHRONIZE,
            false,
            PCWSTR::from_raw(name_w.as_ptr()),
        ) {
            Ok(handle) => {
                let _ = CloseHandle(handle);
                true
            }
            Err(_) => false,
        }
    }
}

/// Signal to the engine that settings have been updated and config.json should be re-read immediately.
pub fn notify_settings_changed() {
    let name_w: Vec<u16> = SETTINGS_CHANGED_EVENT_NAME.encode_utf16().collect();
    // SAFETY: OpenEventW and SetEvent pulse or trigger the named notification event.
    unsafe {
        if let Ok(handle) = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR::from_raw(name_w.as_ptr()))
        {
            let _ = SetEvent(handle);
            let _ = CloseHandle(handle);
        }
    }
}

/// Create or open the settings changed event for the engine loop to wait on.
pub fn create_settings_change_event() -> windows::core::Result<HANDLE> {
    let name_w: Vec<u16> = SETTINGS_CHANGED_EVENT_NAME.encode_utf16().collect();
    // SAFETY: CreateEventW creates auto-reset named event for IPC notifications.
    unsafe {
        CreateEventW(
            None,
            false, // auto-reset
            false, // initially not signaled
            PCWSTR::from_raw(name_w.as_ptr()),
        )
    }
}
