use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use lazy_static::lazy_static;

use crate::mover;
use crate::settings::SettingsManager;

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
                let (_, timeout_result) = cvar.wait_timeout(running, interval).unwrap();

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
}
