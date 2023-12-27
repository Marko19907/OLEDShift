#![windows_subsystem = "windows"]

/*!
An application that runs in the system tray.

Requires the following features: `cargo run --example system_tray --features "tray-notification message-window menu cursor"`
 */
extern crate native_windows_gui as nwg;

use nwg::NativeUi;
use view::SystemTray;

mod view;
mod mover;
mod controller;
mod delay_dialog;
mod distance_dialog;
mod settings;


fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
