use std::collections::{HashMap, HashSet};
use std::fs::{File, write};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use winapi::{
    shared::basetsd::UINT32,
    shared::ntdef::PWSTR,
    shared::winerror::{APPMODEL_ERROR_NO_PACKAGE, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS}
};
use crate::controller::Delays;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    running: bool,
    delay_milliseconds: i32,
    max_distance_x: i32,
    max_distance_y: i32,
    #[serde(default)] // If the field is missing, default to an empty HashMap. TODO: Maybe introduce a version field to handle future changes?
    enabled_monitors: HashMap<String, bool>,
}

/// Lowest delay allowed, in milliseconds (1 second)
pub const LOWEST_DELAY: Duration = Duration::from_secs(1);

/// Highest delay allowed, in milliseconds (30 minutes)
pub const MAX_DELAY: Duration = Duration::from_secs(30 * 60);

/// Lowest max distance allowed, in pixels
pub const LOWEST_MAX_DISTANCE: i32 = 1;

impl Settings {
    fn default() -> Self {
        // The default settings
        return Settings {
            running: true,
            delay_milliseconds: Delays::ThirtySeconds as i32,
            max_distance_x: 50,
            max_distance_y: 50,
            enabled_monitors: HashMap::new(),
        };
    }

    pub fn get_running(&self) -> bool {
        return self.running;
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Returns the delay
    pub fn get_delay(&self) -> Duration {
        return Duration::from_millis(self.delay_milliseconds as u64);
    }

    /// Sets the delay, in milliseconds
    pub fn set_delay(&mut self, delay: Duration) {
        self.delay_milliseconds = delay.as_millis() as i32;
    }

    pub fn get_max_distance(&self) -> (i32, i32) {
        return (self.max_distance_x, self.max_distance_y);
    }

    pub fn set_max_distance(&mut self, max_distance_x: i32, max_distance_y: i32) {
        self.max_distance_x = max_distance_x;
        self.max_distance_y = max_distance_y;
    }

    pub fn get_all_monitors(&self) -> HashMap<String, bool> {
        return self.enabled_monitors.clone();
    }

    pub fn set_monitor_state(&mut self, monitor: &str, enabled: bool) {
        self.enabled_monitors.insert(monitor.to_string(), enabled);
    }
}



// win-api doesn't have bindings for GetCurrentPackageFamilyName yet, so we define it ourselves
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentPackageFamilyName(packageFamilyNameLength: *mut UINT32, packageFamilyName: PWSTR) -> i32;
}

/// Returns Some(PackageFamilyName) if packaged, None if unpackaged.
fn package_family_name() -> Option<String> {
    unsafe {
        let mut len: UINT32 = 0;

        // First probe for required length
        let rc = GetCurrentPackageFamilyName(&mut len, std::ptr::null_mut());
        let rc_u = rc as u32;

        if rc_u == APPMODEL_ERROR_NO_PACKAGE {
            return None;
        }
        if rc_u != ERROR_INSUFFICIENT_BUFFER {
            return None;
        }

        // Then allocate and fetch the PFN
        let mut buf: Vec<u16> = vec![0u16; len as usize];
        let rc2 = GetCurrentPackageFamilyName(&mut len, buf.as_mut_ptr());
        if (rc2 as u32) != ERROR_SUCCESS {
            return None;
        }

        // len includes the NUL terminator, truncate it
        if len > 0 {
            buf.truncate((len - 1) as usize);
        }

        return Some(String::from_utf16_lossy(&buf));
    }
}

/// settings.json path:
/// - packaged: %LOCALAPPDATA%\Packages\<PFN>\LocalState\settings.json (Microsoft Store package)
/// - unpackaged: next to the exe, great for development and portable use
fn settings_path() -> PathBuf {
    if let Some(pfn) = package_family_name() {
        let mut base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        base.push("Packages");
        base.push(pfn);
        base.push("LocalState");

        // Ensure LocalState exists, create if necessary
        let _ = std::fs::create_dir_all(&base);

        base.push("settings.json");
        base
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("settings.json")))
            .unwrap_or_else(|| PathBuf::from("settings.json"))
    }
}



pub struct SettingsManager {
    settings: Arc<Mutex<Settings>>,
}

impl Default for SettingsManager {
    fn default() -> Self {
        return SettingsManager {
            settings: Arc::new(Mutex::new(Settings::default())),
        };
    }
}

impl SettingsManager {
    pub fn new() -> Result<SettingsManager, (String, SettingsManager)> {
        println!("Loading settings...");

        let path = settings_path();

        if !path.exists() {
            println!("No settings file found, creating default settings...");
            let settings = Settings::default();
            let serialized = serde_json::to_string_pretty(&settings).unwrap();
            if let Err(err) = write(&path, serialized) {
                eprintln!("Failed to create the default settings file: {}", err);
            }
        }

        let result = File::open(&path)
            .and_then(|mut file| {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                Ok(serde_json::from_str::<Settings>(&contents)?)
            });

        return match result {
            Ok(mut settings) => {
                let mut errors: Vec<String> = Vec::new();

                // Validate the settings and update them if necessary since the user could have edited the settings file
                if settings.delay_milliseconds < LOWEST_DELAY.as_millis() as i32 {
                    settings.delay_milliseconds = LOWEST_DELAY.as_millis() as i32;
                    errors.push(
                        format!("The delay was too low, it has been set to the lowest possible value of {} second.", LOWEST_DELAY.as_millis() as i32 / 1000)
                    );
                }
                if settings.delay_milliseconds > MAX_DELAY.as_millis() as i32 {
                    settings.delay_milliseconds = MAX_DELAY.as_millis() as i32;
                    errors.push(
                        format!("The delay was too high, it has been set to the highest possible value of {} seconds.", MAX_DELAY.as_millis() as i32 / 1000)
                    );
                }

                if settings.max_distance_x < LOWEST_MAX_DISTANCE {
                    settings.max_distance_x = LOWEST_MAX_DISTANCE;
                    errors.push(
                        format!("The max distance X was too low, it has been set to the lowest possible value of {} pixel.", LOWEST_MAX_DISTANCE)
                    );
                }
                if settings.max_distance_y < LOWEST_MAX_DISTANCE {
                    settings.max_distance_y = LOWEST_MAX_DISTANCE;
                    errors.push(
                        format!("The max distance Y was too low, it has been set to the lowest possible value of {} pixel.", LOWEST_MAX_DISTANCE)
                    );
                }

                if !errors.is_empty() {
                    println!("Found invalid values in the settings file!");

                    // Update the settings file with the valid settings
                    let serialized = serde_json::to_string_pretty(&settings).expect("Failed to serialize the settings");
                    if let Err(err) = write(&path, serialized) {
                        eprintln!("Failed to update the settings file: {}", err);
                    }

                    return Err((
                        errors.join("\n"),
                        SettingsManager {
                            settings: Arc::new(Mutex::new(settings)),
                        }
                    ));
                }

                println!("Settings loaded successfully!");
                Ok(SettingsManager {
                    settings: Arc::new(Mutex::new(settings)),
                })
            }
            Err(err) => {
                // Send the error message back to the UI with the default settings
                Err((
                    err.to_string(),
                    SettingsManager::default(),
                ))
            }
        }
    }

    /// Serializes and saves the settings to the settings file
    fn save_settings(settings: &Settings) {
        let path = settings_path();
        let serialized = serde_json::to_string_pretty(settings).unwrap();

        if let Err(err) = write(&path, serialized) {
            eprintln!("Failed to write settings file {:?}: {}", path, err);
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Getters and setters
    // ---------------------------------------------------------------------------------------------

    pub fn is_running(&self) -> bool {
        let settings = self.settings.lock().unwrap();
        return settings.get_running();
    }

    /// Sets the running state, and saves the settings to the settings file
    pub fn set_running(&self, running: bool) {
        let mut settings = self.settings.lock().unwrap();
        settings.set_running(running);
        SettingsManager::save_settings(&*settings);
    }

    /// Toggles the running state, and saves the settings to the settings file
    pub fn toggle_running(&self) {
        let mut settings = self.settings.lock().unwrap();
        settings.running = !settings.running;
        SettingsManager::save_settings(&*settings);
    }

    pub fn get_delay(&self) -> Duration {
        let settings = self.settings.lock().unwrap();
        return settings.get_delay();
    }

    /// Sets the delay, in milliseconds, and saves the settings to the settings file
    pub fn set_delay(&self, duration: Duration) {
        let mut settings = self.settings.lock().unwrap();
        settings.set_delay(duration);
        SettingsManager::save_settings(&*settings);
    }

    pub fn get_max_distance(&self) -> (i32, i32) {
        let settings = self.settings.lock().unwrap();
        return settings.get_max_distance();
    }

    /// Sets the max distance, in pixels, and saves the settings to the settings file
    pub fn set_max_distance(&self, max_distance_x: i32, max_distance_y: i32) {
        let mut settings = self.settings.lock().unwrap();
        settings.set_max_distance(max_distance_x, max_distance_y);
        SettingsManager::save_settings(&*settings);
    }

    /// Returns all the monitors in the settings file
    pub fn get_all_monitors(&self) -> HashMap<String, bool> {
        let settings = self.settings.lock().unwrap();
        return settings.get_all_monitors();
    }

    /// Returns only the enabled monitors
    pub fn get_enabled_monitors(&self) -> HashSet<String> {
        let settings = self.settings.lock().unwrap();
        return settings.get_all_monitors().iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(monitor, _)| monitor.to_string())
            .collect();
    }

    pub fn set_monitor_state(&self, monitor: &str, enabled: bool) {
        let mut settings = self.settings.lock().unwrap();
        settings.set_monitor_state(monitor, enabled);
        SettingsManager::save_settings(&*settings);
    }
}
