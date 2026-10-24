// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistributionMode {
    Installed,
    Portable,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub mode: DistributionMode,
    pub exe_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub webview_dir: PathBuf,
    pub fallback_from_portable: bool,
}

static INSTANCE: OnceLock<AppPaths> = OnceLock::new();

impl AppPaths {
    /// Return the global singleton instance of AppPaths, initialized on first call.
    pub fn get() -> &'static AppPaths {
        INSTANCE.get_or_init(AppPaths::resolve)
    }

    /// Resolve paths for the current running process environment.
    pub fn resolve() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let env_portable = std::env::var("FLIP3D_PORTABLE").ok();
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("flip3d.exe"));
        let exe_dir = current_exe
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let app_data = std::env::var_os("APPDATA").map(PathBuf::from);
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

        Self::resolve_custom(
            &args,
            env_portable.as_deref(),
            &exe_dir,
            app_data.as_deref(),
            local_app_data.as_deref(),
        )
    }

    /// Custom resolution logic allowing full unit test coverage without polluting environment.
    pub fn resolve_custom(
        args: &[String],
        env_portable: Option<&str>,
        exe_dir: &Path,
        app_data: Option<&Path>,
        local_app_data: Option<&Path>,
    ) -> Self {
        // Precedence:
        // 1. env FLIP3D_PORTABLE=1/true or --portable arg -> Portable
        // 2. flip3d.portable marker file next to current_exe() -> Portable
        // 3. Default -> Installed
        let is_env_or_arg_portable = env_portable
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
            || args.iter().any(|a| a == "--portable");

        let is_marker_portable = exe_dir.join("flip3d.portable").exists();

        let requested_portable = is_env_or_arg_portable || is_marker_portable;

        let base_app_data = app_data
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let base_local_app_data = local_app_data
            .map(PathBuf::from)
            .unwrap_or_else(|| base_app_data.clone());

        let installed_config_dir = base_app_data.join("Flip3D");
        let installed_config_file = installed_config_dir.join("config.json");
        let installed_log_dir = base_local_app_data.join("Flip3D").join("logs");
        let installed_webview_dir = base_local_app_data.join("Flip3D").join("webview2");

        if requested_portable {
            let portable_data_dir = exe_dir.join("data");
            // Probe writability
            if Self::probe_writability(&portable_data_dir) {
                let portable_config_dir = portable_data_dir.clone();
                let portable_config_file = portable_config_dir.join("config.json");
                let portable_log_dir = portable_data_dir.join("logs");
                let portable_webview_dir = portable_data_dir.join("webview2");

                return Self {
                    mode: DistributionMode::Portable,
                    exe_dir: exe_dir.to_path_buf(),
                    config_dir: portable_config_dir,
                    config_file: portable_config_file,
                    log_dir: portable_log_dir,
                    webview_dir: portable_webview_dir,
                    fallback_from_portable: false,
                };
            } else {
                log::warn!(
                    "Portable data directory {:?} is not writable. Falling back to Installed paths.",
                    portable_data_dir
                );
                return Self {
                    mode: DistributionMode::Portable,
                    exe_dir: exe_dir.to_path_buf(),
                    config_dir: installed_config_dir,
                    config_file: installed_config_file,
                    log_dir: installed_log_dir,
                    webview_dir: installed_webview_dir,
                    fallback_from_portable: true,
                };
            }
        }

        Self {
            mode: DistributionMode::Installed,
            exe_dir: exe_dir.to_path_buf(),
            config_dir: installed_config_dir,
            config_file: installed_config_file,
            log_dir: installed_log_dir,
            webview_dir: installed_webview_dir,
            fallback_from_portable: false,
        }
    }

    /// Tests if the directory can be created and written to.
    pub fn probe_writability(dir: &Path) -> bool {
        if fs::create_dir_all(dir).is_err() {
            return false;
        }

        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let test_file = dir.join(format!(".writetest_{}_{}.tmp", pid, nanos));

        let write_res = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&test_file)
            .and_then(|mut f| {
                f.write_all(b"probe")?;
                f.sync_all()
            });

        if write_res.is_ok() {
            let _ = fs::remove_file(&test_file);
            true
        } else {
            false
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn webview_dir(&self) -> &Path {
        &self.webview_dir
    }

    pub fn is_portable(&self) -> bool {
        self.mode == DistributionMode::Portable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_detection_env() {
        let temp =
            std::env::temp_dir().join(format!("flip3d_test_mode_env_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        // Test FLIP3D_PORTABLE=1
        let paths = AppPaths::resolve_custom(&["flip3d.exe".into()], Some("1"), &temp, None, None);
        assert_eq!(paths.mode, DistributionMode::Portable);
        assert!(!paths.fallback_from_portable);

        // Test FLIP3D_PORTABLE=true
        let paths =
            AppPaths::resolve_custom(&["flip3d.exe".into()], Some("true"), &temp, None, None);
        assert_eq!(paths.mode, DistributionMode::Portable);

        // Test CLI arg --portable
        let paths = AppPaths::resolve_custom(
            &["flip3d.exe".into(), "--portable".into()],
            None,
            &temp,
            None,
            None,
        );
        assert_eq!(paths.mode, DistributionMode::Portable);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_mode_detection_file() {
        let temp =
            std::env::temp_dir().join(format!("flip3d_test_mode_file_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);
        let marker = temp.join("flip3d.portable");
        fs::write(&marker, b"").expect("create marker file");

        let paths = AppPaths::resolve_custom(&["flip3d.exe".into()], None, &temp, None, None);
        assert_eq!(paths.mode, DistributionMode::Portable);
        assert!(!paths.fallback_from_portable);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_mode_detection_default() {
        let temp =
            std::env::temp_dir().join(format!("flip3d_test_mode_def_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        let paths = AppPaths::resolve_custom(&["flip3d.exe".into()], None, &temp, None, None);
        assert_eq!(paths.mode, DistributionMode::Installed);
        assert!(!paths.fallback_from_portable);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_paths_installed() {
        let appdata = std::env::temp_dir().join(format!("flip3d_appdata_{}", std::process::id()));
        let local_appdata =
            std::env::temp_dir().join(format!("flip3d_local_{}", std::process::id()));
        let exe_dir = std::env::temp_dir().join(format!("flip3d_exe_{}", std::process::id()));

        let paths = AppPaths::resolve_custom(
            &["flip3d.exe".into()],
            None,
            &exe_dir,
            Some(&appdata),
            Some(&local_appdata),
        );

        assert_eq!(paths.mode, DistributionMode::Installed);
        assert_eq!(paths.config_dir(), appdata.join("Flip3D").as_path());
        assert_eq!(
            paths.config_file(),
            appdata.join("Flip3D").join("config.json").as_path()
        );
        assert_eq!(
            paths.log_dir(),
            local_appdata.join("Flip3D").join("logs").as_path()
        );
        assert_eq!(
            paths.webview_dir(),
            local_appdata.join("Flip3D").join("webview2").as_path()
        );
    }

    #[test]
    fn test_paths_portable() {
        let exe_dir = std::env::temp_dir().join(format!("flip3d_port_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&exe_dir);
        let marker = exe_dir.join("flip3d.portable");
        fs::write(&marker, b"").expect("write marker");

        let appdata = std::env::temp_dir().join("some_appdata");
        let local_appdata = std::env::temp_dir().join("some_local");

        let paths = AppPaths::resolve_custom(
            &["flip3d.exe".into()],
            None,
            &exe_dir,
            Some(&appdata),
            Some(&local_appdata),
        );

        assert_eq!(paths.mode, DistributionMode::Portable);
        assert!(!paths.fallback_from_portable);
        assert_eq!(paths.config_dir(), exe_dir.join("data").as_path());
        assert_eq!(
            paths.config_file(),
            exe_dir.join("data").join("config.json").as_path()
        );
        assert_eq!(paths.log_dir(), exe_dir.join("data").join("logs").as_path());
        assert_eq!(
            paths.webview_dir(),
            exe_dir.join("data").join("webview2").as_path()
        );

        let _ = fs::remove_dir_all(&exe_dir);
    }

    #[test]
    fn test_writability_fallback() {
        let temp = std::env::temp_dir().join(format!("flip3d_ro_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        // Create a regular file at temp/data so that fs::create_dir_all(temp/data) fails
        let blocker = temp.join("data");
        fs::write(&blocker, b"not a directory").expect("write blocker file");

        let appdata =
            std::env::temp_dir().join(format!("flip3d_ro_appdata_{}", std::process::id()));
        let local_appdata =
            std::env::temp_dir().join(format!("flip3d_ro_local_{}", std::process::id()));

        let paths = AppPaths::resolve_custom(
            &["flip3d.exe".into(), "--portable".into()],
            None,
            &temp,
            Some(&appdata),
            Some(&local_appdata),
        );

        assert_eq!(paths.mode, DistributionMode::Portable);
        assert!(paths.fallback_from_portable);
        // Paths must have fallen back to installed paths
        assert_eq!(paths.config_dir(), appdata.join("Flip3D").as_path());
        assert_eq!(
            paths.log_dir(),
            local_appdata.join("Flip3D").join("logs").as_path()
        );
        assert_eq!(
            paths.webview_dir(),
            local_appdata.join("Flip3D").join("webview2").as_path()
        );

        let _ = fs::remove_dir_all(&temp);
    }
}
