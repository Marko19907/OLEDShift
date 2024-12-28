use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use crate::monitor_info::{get_all_monitors_info, get_display_device_info, monitor_device_name};
use crate::mover;
use crate::settings::SettingsManager;
use lazy_static::lazy_static;

/// The delays that can be selected from the tray menu, in milliseconds
pub enum Delays {
    ThirtySeconds = 30000,
    OneMinute = 60000,
    TwoMinutes = 120000,
    FiveMinutes = 300000,
    Custom = 0,
}

impl Delays {
    /// Converts a delay in milliseconds to a Delays enum
    pub(crate) fn from_millis(millis: i32) -> Self {
        match millis {
            x if x == Delays::ThirtySeconds as i32 => Delays::ThirtySeconds,
            x if x == Delays::OneMinute as i32 => Delays::OneMinute,
            x if x == Delays::TwoMinutes as i32 => Delays::TwoMinutes,
            x if x == Delays::FiveMinutes as i32 => Delays::FiveMinutes,
            _ => Delays::Custom,
        }
    }

    /// Returns the delay in milliseconds
    fn as_millis(&self) -> u64 {
        match self {
            Delays::ThirtySeconds => 30000,
            Delays::OneMinute => 60000,
            Delays::TwoMinutes => 120000,
            Delays::FiveMinutes => 300000,
            Delays::Custom => 0,
        }
    }

    /// Converts a delay to Rust's Duration
    pub fn as_duration(&self) -> Duration {
        Duration::from_millis(self.as_millis())
    }
}

pub enum Distances {
    Small = 25,
    Medium = 50,
    Large = 100,
    Custom = 0,
}

impl Distances {
    /// Converts a distance to a Distances enum
    pub(crate) fn from_distance(max_x: i32, max_y: i32) -> Self {
        if max_x != max_y {
            return Distances::Custom;
        }
        match max_x {
            x if x == Distances::Small as i32 => Distances::Small,
            x if x == Distances::Medium as i32 => Distances::Medium,
            x if x == Distances::Large as i32 => Distances::Large,
            _ => Distances::Custom,
        }
    }
}

lazy_static! {
    // This is a global variable that can be accessed from anywhere in the program
    // It wasn't possible to pass the max move to the mover function since it's used in a C style callback, so this is the next best thing
    pub static ref MAX_MOVE: Mutex<(i32, i32)> = Mutex::new((50, 50));

    // Same as above, but for the enabled monitors
    pub static ref ENABLED_MONITORS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

pub(crate) struct Controller {
    settings_manager: SettingsManager,
    condvar: Arc<(Mutex<bool>, Condvar)>
}

impl Default for Controller {
    fn default() -> Self {
        let controller = Controller {
            settings_manager: SettingsManager::default(),
            condvar: Arc::new((Mutex::new(false), Condvar::new())),
        };
        controller.update_max_move();
        return controller;
    }
}

impl Controller {
    pub fn new() -> Self {
        return Controller::default();
    }

    /// Sets the settings manager for the controller
    pub fn set_settings(controller: Arc<Mutex<Self>>, settings: SettingsManager) {
        let mut controller = controller.lock().unwrap();
        controller.settings_manager = settings;

        // Sync the controller state with the settings file
        *ENABLED_MONITORS.lock().unwrap() = controller.get_all_monitors();
        controller.update_max_move();
    }

    pub fn run(controller: Arc<Mutex<Self>>) {
        let condvar = controller.lock().unwrap().condvar.clone();
        thread::Builder::new().name("mover_thread".to_string()).spawn(move || {
            let (lock, cvar) = &*condvar;
            loop {
                let interval = {
                    let controller = controller.lock().unwrap();
                    controller.get_interval()
                };

                let mut running = lock.lock().unwrap();
                *running = false;
                let (new_running, timeout_result) = cvar.wait_timeout(running, interval).unwrap();
                running = new_running;

                if timeout_result.timed_out() {
                    let controller = controller.lock().unwrap();
                    if controller.is_running() {
                        mover::move_all_windows();
                    }
                }
            }
        }).expect("Thread failed to start");
    }

    pub fn get_interval(&self) -> Duration {
        return self.settings_manager.get_delay();
    }

    pub fn set_interval(&mut self, interval: Duration) {
        self.settings_manager.set_delay(interval);
        let (lock, cvar) = &*self.condvar;
        let mut running = lock.lock().unwrap();
        *running = true;
        cvar.notify_all();
    }

    pub fn is_running(&self) -> bool {
        return self.settings_manager.is_running();
    }

    pub fn toggle_running(&mut self) {
        self.settings_manager.toggle_running();
        let (lock, cvar) = &*self.condvar;
        let mut running = lock.lock().unwrap();
        *running = true;
        cvar.notify_all();
    }

    pub fn get_max_move(&self) -> (i32, i32) {
        return self.settings_manager.get_max_distance();
    }

    pub fn set_max_move(&mut self, max_move_x: i32, max_move_y: i32) {
        *MAX_MOVE.lock().unwrap() = (max_move_x, max_move_y);
        self.settings_manager.set_max_distance(max_move_x, max_move_y);
    }

    /// Updates the max move from the settings file, to be used on startup
    fn update_max_move(&self) {
        *MAX_MOVE.lock().unwrap() = self.settings_manager.get_max_distance();
    }

    /// Returns all the monitors in the format: device_id => (friendly_name, is_enabled, is_connected)
    pub fn get_monitors_merged(&self) -> HashMap<String, (String, bool, bool)> {
        let connected = self.get_connected_monitors();
        let known = self.get_all_monitors();

        let mut merged = HashMap::new();

        for (device_id, friendly_name) in connected {
            // If device_id is absent in known, default is_enabled to true
            let is_enabled = known.get(&device_id).cloned().unwrap_or(true);
            merged.insert(device_id, (friendly_name, is_enabled, true));
        }

        for (device_id, is_enabled) in known {
            if !merged.contains_key(&device_id) {
                merged.insert(device_id.clone(), ("(Offline)".to_string(), is_enabled, false));
            }
        }

        return merged;
    }


    /// Returns the currently connected monitors in the format (device name, friendly name)
    pub fn get_connected_monitors(&self) -> HashMap<String, String> {
        let monitors_info = get_all_monitors_info();

        return monitors_info.iter().filter_map(|monitor_info| {
            let device_name = monitor_device_name(monitor_info);

            return get_display_device_info(&device_name).map(|(friendly_name, device_id)| {
                (device_id, friendly_name)
            });
        })
            .collect::<HashMap<String, String>>();
    }

    pub fn get_all_monitors(&self) -> HashMap<String, bool> {
        return self.settings_manager.get_all_monitors();
    }

    /// Sets the monitor state in the settings file
    pub fn set_monitor_state(&mut self, monitor: &str, enabled: bool) {
        self.settings_manager.set_monitor_state(monitor, enabled);
        *ENABLED_MONITORS.lock().unwrap() = self.get_all_monitors();
    }

    /// Adds a monitor to the app
    pub fn add_monitor(&mut self, monitor: &str) {
        self.settings_manager.set_monitor_state(monitor, true);
    }
}
