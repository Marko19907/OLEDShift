use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use crate::mover;

pub(crate) struct Controller {
    interval: i32,
    running: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            interval: 5000,
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
            loop {
                let controller = controller.lock().unwrap();
                if controller.running {
                    println!("Moved windows from a thread!");
                    mover::run();
                }
                let interval = controller.interval;
                drop(controller);
                sleep(Duration::from_millis(interval as u64));
            }
        }).expect("Thread failed to start");
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
}
