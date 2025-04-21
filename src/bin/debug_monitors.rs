//! Simple console that dumps every monitor OLEDShift knows about.
//! Meant for debugging!

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use oledshift::{
    controller::{Controller},
    settings::SettingsManager,
};

/// Pretty‑prints the merged monitor table the same way the tray UI builds it.
fn dump(controller: &Controller) {
    let merged = controller.get_monitors_merged();

    println!(
        "{:<42} | {:<30} | {:^9} | {:^7}",
        "Device ID", "Friendly name", "Connected", "Enabled"
    );
    println!("{}", "-".repeat(95));

    for (id, (friendly, enabled, connected)) in merged {
        println!(
            "{:<42} | {:<30} | {:^9} | {:^7}",
            id,
            friendly,
            connected,
            enabled
        );
    }
    println!();
}

fn main() {
    // Bring up the same controller state as the tray app
    let settings = SettingsManager::new().unwrap_or_else(|(err, mgr)| {
        eprintln!("WARNING: settings.json parse error, using defaults:\n{err}");
        mgr
    });

    let controller = Arc::new(Mutex::new(Controller::new()));
    Controller::set_settings(controller.clone(), settings);
    
    // Monitors found in the settings
    let found_monitors = controller.lock().unwrap().get_all_monitors();
    print!("Found {} monitors in settings.json: {:?}\n", found_monitors.len(), found_monitors);

    
    loop {
        println!("OLEDShift monitor debug console — press R then <Enter> to refresh, Q to quit\n");

        dump(&controller.lock().unwrap());

        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        
        match line.trim().to_ascii_lowercase().as_str() {
            "r" => continue,
            "q" | "quit" | "exit" => break,
            _ => continue,
        }
    }
}
