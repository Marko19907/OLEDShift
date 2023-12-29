use std::fs::{File, write};
use std::io::Read;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    running: bool,
    delay: i32,
    max_distance_x: i32,
    max_distance_y: i32,
}

impl Settings {
    fn default() -> Self {
        // The default settings
        return Settings {
            running: true,
            delay: 30000, // 30 seconds // TODO: Use Delays::ThirtySeconds perhaps or move this to seconds instead of milliseconds
            max_distance_x: 50,
            max_distance_y: 50,
        };
    }

    pub fn get_running(&self) -> bool {
        return self.running;
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Returns the delay in milliseconds
    pub fn get_delay(&self) -> i32 {
        return self.delay;
    }

    /// Sets the delay, in milliseconds
    pub fn set_delay(&mut self, delay: i32) {
        self.delay = delay;
    }

    /// Returns the delay in seconds
    pub fn get_delay_seconds(&self) -> i32 {
        return self.delay / 1000;
    }

    /// Sets the delay, in seconds
    pub fn set_delay_seconds(&mut self, delay: i32) {
        self.delay = delay * 1000;
    }

    pub fn get_max_distance(&self) -> (i32, i32) {
        return (self.max_distance_x, self.max_distance_y);
    }

    pub fn set_max_distance(&mut self, max_distance_x: i32, max_distance_y: i32) {
        self.max_distance_x = max_distance_x;
        self.max_distance_y = max_distance_y;
    }
}



const PATH: &str = "settings.json";

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
    pub fn new() -> Self {
        println!("Loading settings...");

        if !std::path::Path::new(PATH).exists() {
            println!("No settings file found, creating default settings...");
            let settings = Settings::default();
            let serialized = serde_json::to_string_pretty(&settings).unwrap();
            write(PATH, serialized).unwrap();
        }

        let mut file = File::open(PATH).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let settings: Settings = serde_json::from_str(&contents).unwrap();

        println!("Done!");

        return SettingsManager {
            settings: Arc::new(Mutex::new(settings)),
        }
    }

    /// Serializes and saves the settings to the settings file
    fn save_settings(settings: &Settings) {
        let serialized = serde_json::to_string_pretty(settings).unwrap();
        write(PATH, serialized).unwrap();
    }

    // ---------------------------------------------------------------------------------------------
    // Getters and setters
    // ---------------------------------------------------------------------------------------------

    pub fn is_running(&self) -> bool {
        let settings = self.settings.lock().unwrap();
        return settings.running;
    }

    /// Sets the running state, and saves the settings to the settings file
    pub fn set_running(&self, running: bool) {
        let mut settings = self.settings.lock().unwrap();
        settings.running = running;
        SettingsManager::save_settings(&*settings);
    }

    /// Toggles the running state, and saves the settings to the settings file
    pub fn toggle_running(&self) {
        let mut settings = self.settings.lock().unwrap();
        settings.running = !settings.running;
        SettingsManager::save_settings(&*settings);
    }

    pub fn get_delay(&self) -> i32 {
        let settings = self.settings.lock().unwrap();
        return settings.delay;
    }

    /// Sets the delay, in milliseconds, and saves the settings to the settings file
    pub fn set_delay(&self, delay: i32) {
        let mut settings = self.settings.lock().unwrap();
        settings.delay = delay;
        SettingsManager::save_settings(&*settings);
    }

    pub fn get_delay_seconds(&self) -> i32 {
        let settings = self.settings.lock().unwrap();
        return settings.get_delay_seconds();
    }

    /// Sets the delay, in seconds, and saves the settings to the settings file
    pub fn set_delay_seconds(&self, delay: i32) {
        let mut settings = self.settings.lock().unwrap();
        settings.set_delay_seconds(delay);
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
}
