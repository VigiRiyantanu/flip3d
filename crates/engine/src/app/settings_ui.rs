// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const DETACHED_PROCESS: u32 = 0x00000008;

pub fn spawn_settings_window() {
    std::thread::spawn(|| {
        let current_exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => std::path::PathBuf::from("flip3d.exe"),
        };

        // Prefer dedicated settings-ui.exe if present in the same directory
        let settings_exe = current_exe.parent().map(|p| p.join("settings-ui.exe"));
        let (cmd_path, is_separate_exe) = match settings_exe {
            Some(ref p) if p.exists() => (p.clone(), true),
            _ => (current_exe, false),
        };

        log::info!("Launching settings UI subprocess: {:?}", cmd_path);

        let mut cmd = Command::new(&cmd_path);
        if !is_separate_exe {
            cmd.arg("--settings");
        }
        if settings_core::AppPaths::get().mode == settings_core::DistributionMode::Portable {
            cmd.arg("--portable");
        }

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);

        match cmd.spawn() {
            Ok(_) => {
                log::info!("Settings UI subprocess spawned successfully");
            }
            Err(e) => {
                log::error!("Failed to spawn settings UI subprocess: {}", e);
            }
        }
    });
}
