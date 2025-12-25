use std::path::PathBuf;
use lazy_static::lazy_static;


lazy_static! {
    static ref SETTINGS_PATH: PathBuf = packaged_settings_path().unwrap_or_else(|_| exe_settings_path());
}

/// settings.json path:
/// - packaged: %LOCALAPPDATA%\Packages\<PFN>\LocalState\settings.json (Microsoft Store package)
/// - unpackaged: next to the exe, great for development and portable use
pub fn settings_path() -> &'static PathBuf {
    &SETTINGS_PATH
}


/// Returns the settings path next to the executable
fn exe_settings_path() -> PathBuf {
    return std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("settings.json")))
        .unwrap_or_else(|| PathBuf::from("settings.json"));
}

/// Returns the settings path for packaged applications using the Windows API
fn packaged_settings_path() -> windows::core::Result<PathBuf> {
    let folder = windows::Storage::ApplicationData::Current()?.LocalFolder()?;
    let local_state_path = folder.Path()?;

    return Ok(PathBuf::from(local_state_path.to_string_lossy()).join("settings.json"));
}
