use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use crate::mover;

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
}

lazy_static! {
    pub static ref MAX_MOVE_X: Mutex<i32> = Mutex::new(50);
    pub static ref MAX_MOVE_Y: Mutex<i32> = Mutex::new(50);
}

pub(crate) struct Controller {
    interval: i32,
    running: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            interval: Delays::ThirtySeconds as i32,
            running: true,
        }
    }
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(controller: Arc<Mutex<Self>>) {
        thread::Builder::new().name("mover_thread".to_string()).spawn(move || {
            let interval = controller.lock().unwrap().interval as u64;
            sleep(Duration::from_millis(interval)); // Wait for the first interval, don't move windows immediately

            loop {
                let controller = controller.lock().unwrap();
                let running = controller.running;
                let interval = controller.interval as u64;
                drop(controller);

                if running {
                    mover::run();
                }

                sleep(Duration::from_millis(interval));
            }
        }).expect("Thread failed to start");
    }

    pub fn get_interval(&self) -> i32 {
        self.interval
    }

    pub fn set_interval(&mut self, interval: i32) {
        self.interval = interval;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn toggle_running(&mut self) {
        self.running = !self.running;
    }

    pub fn get_max_move(&self) -> (i32, i32) {
        let max_x = *MAX_MOVE_X.lock().unwrap();
        let max_y = *MAX_MOVE_Y.lock().unwrap();
        return (max_x, max_y);
    }

    pub fn set_max_move(&mut self, max_move_x: i32, max_move_y: i32) {
        *MAX_MOVE_X.lock().unwrap() = max_move_x;
        *MAX_MOVE_Y.lock().unwrap() = max_move_y;
    }
}
