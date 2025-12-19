use crate::controller::{Controller, Delays, Distances};
use crate::delay_dialog::{DelayDialog, DelayDialogData};
use crate::distance_dialog::{DistanceDialog, DistanceDialogData};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{cell::RefCell, thread};

pub static ICON: &[u8] = include_bytes!("../icon.ico");

#[derive(Default)]
pub struct SystemTray {
    window: nwg::MessageWindow,
    icon: nwg::Icon,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    enabled_toggle: nwg::MenuItem,
    delay_menu: nwg::Menu,
    delay_30_menu: nwg::MenuItem,
    delay_1_menu: nwg::MenuItem,
    delay_2_menu: nwg::MenuItem,
    delay_5_menu: nwg::MenuItem,
    delay_custom_menu: nwg::MenuItem,
    distance_menu: nwg::Menu,
    distance_small_menu: nwg::MenuItem,
    distance_medium_menu: nwg::MenuItem,
    distance_large_menu: nwg::MenuItem,
    distance_custom_menu: nwg::MenuItem,
    screen_menu: nwg::Menu,
    screens_map: RefCell<HashMap<String, nwg::MenuItem>>,
    exit_menu: nwg::MenuItem,
    separator_delay: nwg::MenuSeparator,
    separator_distance: nwg::MenuSeparator,
    controller: Arc<Mutex<Controller>>,
    delay_dialog_data: RefCell<Option<thread::JoinHandle<DelayDialogData>>>,
    delay_dialog_notice: nwg::Notice,
    distance_dialog_data: RefCell<Option<thread::JoinHandle<DistanceDialogData>>>,
    distance_dialog_notice: nwg::Notice,
}

impl SystemTray {
    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn toggle_enabled(&self) {
        println!("Toggling enabled called");
        self.controller.lock().unwrap().toggle_running();
        self.update_toggle();
        self.update_tooltip();
    }

    fn hello1(&self) {
        nwg::modal_info_message(&self.window, "Hello", "Hello World!");
    }

    fn show_start_message(&self) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray.show("OLEDShift", Some("OLEDShift is running in the system tray"), Some(flags), Some(&self.icon));
    }

    /// Shows the failed to parse the config file error message
    fn show_config_parse_failed_message(&self, error_message: &str) {
        let message = format!("Failed to parse the config file!\nThe default settings will be used instead.\n\nError: {}", error_message);
        nwg::modal_error_message(&self.window, "Config parsing failed", &message);
    }

    fn do_delay(&self, delay: Delays) {
        match delay {
            Delays::ThirtySeconds => self.controller.lock().unwrap().set_interval(Delays::ThirtySeconds.as_duration()),
            Delays::OneMinute => self.controller.lock().unwrap().set_interval(Delays::OneMinute.as_duration()),
            Delays::TwoMinutes => self.controller.lock().unwrap().set_interval(Delays::TwoMinutes.as_duration()),
            Delays::FiveMinutes => self.controller.lock().unwrap().set_interval(Delays::FiveMinutes.as_duration()),
            Delays::Custom => {
                self.delay_custom();
                return; // Don't update the menu or tooltip, will be done when the dialog closes in the callback
            },
        }
        self.update_delay_menu();
        self.update_tooltip();
    }

    /// Opens a dialog to set a custom delay
    fn delay_custom(&self) {
        *self.delay_dialog_data.borrow_mut() = Some(DelayDialog::popup(
            self.delay_dialog_notice.sender(),
            self.controller.lock().unwrap().get_interval()
        ));
    }

    fn do_distance(&self, distance: Distances) {
        match distance {
            Distances::Small => self.controller.lock().unwrap().set_max_move(Distances::Small as i32, Distances::Small as i32),
            Distances::Medium => self.controller.lock().unwrap().set_max_move(Distances::Medium as i32, Distances::Medium as i32),
            Distances::Large => self.controller.lock().unwrap().set_max_move(Distances::Large as i32, Distances::Large as i32),
            Distances::Custom => {
                self.distance_custom();
                return; // Don't update the menu or tooltip, will be done when the dialog closes in the callback
            },
        }
        self.update_distance_menu();
        self.update_tooltip();
    }

    /// Opens a dialog to set a custom distance
    fn distance_custom(&self) {
        let (max_x, max_y) = self.controller.lock().unwrap().get_max_move();

        *self.distance_dialog_data.borrow_mut() = Some(DistanceDialog::popup(
            self.distance_dialog_notice.sender(),
            max_x,
            max_y
        ));
    }

    /// Updates the toggle menu item to reflect the current state of the controller
    fn update_toggle(&self) {
        self.enabled_toggle.set_checked(self.controller.lock().unwrap().is_running());
    }

    /// Updates the delay menu item to reflect the current state of the controller
    fn update_delay_menu(&self) {
        [&self.delay_30_menu, &self.delay_1_menu, &self.delay_2_menu, &self.delay_5_menu, &self.delay_custom_menu].iter()
            .for_each(|x| x.set_checked(false));

        let interval = self.controller.lock().unwrap().get_interval();
        let interval = interval.as_millis() as i32;
        match Delays::from_millis(interval) {
            Delays::ThirtySeconds => self.delay_30_menu.set_checked(true),
            Delays::OneMinute => self.delay_1_menu.set_checked(true),
            Delays::TwoMinutes => self.delay_2_menu.set_checked(true),
            Delays::FiveMinutes => self.delay_5_menu.set_checked(true),
            _ => self.delay_custom_menu.set_checked(true),
        }
    }

    /// Updates the distance menu item to reflect the current state of the controller
    fn update_distance_menu(&self) {
        [&self.distance_small_menu, &self.distance_medium_menu, &self.distance_large_menu, &self.distance_custom_menu].iter()
            .for_each(|x| x.set_checked(false));

        let (max_x, max_y) = self.controller.lock().unwrap().get_max_move();
        match Distances::from_distance(max_x, max_y) {
            Distances::Small => self.distance_small_menu.set_checked(true),
            Distances::Medium => self.distance_medium_menu.set_checked(true),
            Distances::Large => self.distance_large_menu.set_checked(true),
            _ => self.distance_custom_menu.set_checked(true),
        }
    }

    /// Updates the tooltip to reflect the current state of the controller
    fn update_tooltip(&self) {
        let controller = self.controller.lock().unwrap();

        let pause = if controller.is_running() { "running" } else { "paused" };
        let interval = controller.get_interval();
        let distance = controller.get_max_move();

        drop(controller);

        let delay = self.format_interval(interval);
        let format_distance = self.format_distance(distance.0, distance.1);
        let tooltip = format!("OLEDShift\nStatus: {}\nDelay: {}\nMax distance: {}", pause, delay, format_distance);

        self.tray.set_tip(&tooltip);
    }

    /// Formats an interval in milliseconds into a human readable string
    fn format_interval(&self, duration: Duration) -> String {
        let interval = duration.as_millis() as i32;

        match Delays::from_millis(interval) {
            Delays::ThirtySeconds => return "30 seconds".to_string(),
            Delays::OneMinute => return "1 minute".to_string(),
            Delays::TwoMinutes => return "2 minutes".to_string(),
            Delays::FiveMinutes => return "5 minutes".to_string(),
            _ => {}
        }

        let seconds = interval / 1000;
        if seconds < 60 {
            return format!("{} second{} (Custom)", seconds, if seconds == 1 { "" } else { "s" });
        }

        let minutes = seconds / 60;
        let seconds = seconds % 60;
        if seconds == 0 {
            return format!("{} minutes (Custom)", minutes);
        }

        return format!("{} minute{} and {} second{} (Custom)", minutes, if minutes == 1 { "" } else { "s" }, seconds, if seconds == 1 { "" } else { "s" });
    }

    /// Formats the distance into a human readable string
    fn format_distance(&self, max_x: i32, max_y: i32) -> String {
        match Distances::from_distance(max_x, max_y) {
            Distances::Small => return "25 px (Small)".to_string(),
            Distances::Medium => return "50 px (Medium)".to_string(),
            Distances::Large => return "100 px (Large)".to_string(),
            _ => {}
        }

        return format!("{} px (x) {} px (y) (Custom)", max_x, max_y);
    }

    /// Callback for the dialog notice
    fn read_delay_dialog_output(&self) {
        let data = self.delay_dialog_data.borrow_mut().take();
        match data {
            Some(handle) => {
                let dialog_result = handle.join().unwrap();

                match dialog_result {
                    DelayDialogData::Value(delay) => {
                        self.controller.lock().unwrap().set_interval(delay);
                        self.update_delay_menu();
                        self.update_tooltip();
                    },
                    DelayDialogData::Cancel => {}
                }
            },
            None => {}
        }
    }

    /// Callback for the distance dialog notice
    fn read_distance_dialog_output(&self) {
        let data = self.distance_dialog_data.borrow_mut().take();
        match data {
            Some(handle) => {
                let dialog_result = handle.join().unwrap();

                match dialog_result {
                    DistanceDialogData::Value(distance_x, distance_y) => {
                        self.controller.lock().unwrap().set_max_move(distance_x, distance_y);
                        self.update_distance_menu();
                        self.update_tooltip();
                    },
                    DistanceDialogData::Cancel => {}
                }
            },
            None => {}
        }
    }

    pub fn handle_monitor_selected(&self, device_id: &str) {
        let mut controller = self.controller.lock().unwrap();

        let binding = controller.get_all_monitors();
        let enabled = binding.get(device_id).unwrap_or_else(|| {
            // If the monitor is not in the settings file, add it
            controller.add_monitor(device_id);
            return &true;
        });

        controller.set_monitor_state(device_id, !enabled);
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}


//
// ALL of this stuff is handled by native-windows-derive
//
mod system_tray_ui {
    use crate::controller::{Controller, Delays, Distances};
    use crate::settings::SettingsManager;
    use crate::view::{SystemTray, ICON};
    use native_windows_gui as nwg;
    use std::cell::RefCell;
    use std::ops::Deref;
    use std::rc::Rc;

    pub struct SystemTrayUi {
        inner: Rc<SystemTray>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    /// Refresh the "Screens" submenu based on the merged monitor info.
    pub fn update_screens_submenu(system_tray: &SystemTray) {
        let merged = {
            let controller = system_tray.controller.lock().unwrap();
            controller.get_monitors_merged()
        };

        let mut screens_map = system_tray.screens_map.borrow_mut();

        for (device_id, (friendly_name, is_enabled, is_connected)) in merged.iter() {
            let menu_item = screens_map.entry(device_id.clone())
                .or_insert_with(|| {
                    let text = format!("Monitor - {} ({})", friendly_name, device_id);
                    let mut item = nwg::MenuItem::default();
                    nwg::MenuItem::builder()
                        .text(&text)
                        .check(*is_enabled)
                        .disabled(!*is_connected)
                        .parent(&system_tray.screen_menu)
                        .build(&mut item)
                        .expect("Failed to build screen menu item");
                    item
                });

            menu_item.set_checked(*is_enabled);
            menu_item.set_enabled(*is_connected);
        }


        // For some reason NWG isn't updating the check state of the menu items, this is a workaround
        {
            // First, check all the items
            for (_, menu_item) in screens_map.iter_mut() {
                menu_item.set_checked(true);
            }

            // Then, sync the check state with the controller. This seems to work around the issue for some reason.
            let known_screens = {
                let controller = system_tray.controller.lock().unwrap();
                controller.get_all_monitors()
            };

            for (device_id, enabled) in known_screens.iter() {
                if let Some(menu_item) = screens_map.get_mut(device_id) {
                    menu_item.set_checked(*enabled);
                }
            }
        }
    }

    impl nwg::NativeUi<SystemTrayUi> for SystemTray {
        fn build_ui(mut data: SystemTray) -> Result<SystemTrayUi, nwg::NwgError> {
            use nwg::Event as E;

            // Resources
            nwg::Icon::builder()
                .source_bin(Option::from(ICON))
                .build(&mut data.icon)?;

            // Controls
            nwg::MessageWindow::builder()
                .build(&mut data.window)?;

            nwg::TrayNotification::builder()
                .parent(&data.window)
                .icon(Some(&data.icon))
                .build(&mut data.tray)?;

            nwg::Menu::builder()
                .popup(true)
                .parent(&data.window)
                .build(&mut data.tray_menu)?;

            nwg::MenuItem::builder()
                .text("Enabled")
                .check(true)
                .parent(&data.tray_menu)
                .build(&mut data.enabled_toggle)?;

            nwg::Menu::builder()
                .text("Delay")
                .parent(&data.tray_menu)
                .build(&mut data.delay_menu)?;

            nwg::MenuItem::builder()
                .text("30 seconds")
                .check(true)
                .parent(&data.delay_menu)
                .build(&mut data.delay_30_menu)?;

            nwg::MenuItem::builder()
                .text("1 minute")
                .parent(&data.delay_menu)
                .build(&mut data.delay_1_menu)?;

            nwg::MenuItem::builder()
                .text("2 minutes")
                .parent(&data.delay_menu)
                .build(&mut data.delay_2_menu)?;

            nwg::MenuItem::builder()
                .text("5 minutes")
                .parent(&data.delay_menu)
                .build(&mut data.delay_5_menu)?;

            nwg::MenuSeparator::builder()
                .parent(&data.delay_menu)
                .build(&mut data.separator_delay)?;

            nwg::MenuItem::builder()
                .text("Custom delay")
                .parent(&data.delay_menu)
                .build(&mut data.delay_custom_menu)?;

            nwg::Menu::builder()
                .text("Max distance")
                .parent(&data.tray_menu)
                .build(&mut data.distance_menu)?;

            nwg::MenuItem::builder()
                .text("Small, 25 px")
                .parent(&data.distance_menu)
                .build(&mut data.distance_small_menu)?;

            nwg::MenuItem::builder()
                .text("Medium, 50 px")
                .parent(&data.distance_menu)
                .build(&mut data.distance_medium_menu)?;

            nwg::MenuItem::builder()
                .text("Large, 100 px")
                .parent(&data.distance_menu)
                .build(&mut data.distance_large_menu)?;

            nwg::MenuSeparator::builder()
                .parent(&data.distance_menu)
                .build(&mut data.separator_distance)?;

            nwg::MenuItem::builder()
                .text("Custom distance")
                .parent(&data.distance_menu)
                .build(&mut data.distance_custom_menu)?;

            nwg::Menu::builder()
                .text("Screens")
                .parent(&data.tray_menu)
                .build(&mut data.screen_menu)?;

            nwg::MenuSeparator::builder()
                .parent(&data.tray_menu)
                .build(&mut data.separator_delay)?;

            nwg::MenuItem::builder()
                .text("Exit")
                .parent(&data.tray_menu)
                .build(&mut data.exit_menu)?;

            // Dialog events
            nwg::Notice::builder()
                .parent(&data.window)
                .build(&mut data.delay_dialog_notice)?;

            nwg::Notice::builder()
                .parent(&data.window)
                .build(&mut data.distance_dialog_notice)?;

            // Wrap-up
            let ui = SystemTrayUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Setup the controller
            let settings_manager = SettingsManager::new().unwrap_or_else(|(err, manager)| {
                ui.inner.show_config_parse_failed_message(&err);
                return manager;
            });
            Controller::set_settings(ui.inner.controller.clone(), settings_manager);
            // Start the controller
            Controller::run(ui.inner.controller.clone());

            // Update the UI to reflect the controller state at startup
            ui.inner.update_delay_menu();
            ui.inner.update_distance_menu();
            ui.inner.update_toggle();
            ui.inner.update_tooltip();
            update_screens_submenu(&ui.inner);

            SystemTray::show_start_message(&ui.inner);

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(evt_ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnNotice =>
                            if &handle == &evt_ui.delay_dialog_notice {
                                SystemTray::read_delay_dialog_output(&evt_ui);
                            }
                            else if &handle == &evt_ui.distance_dialog_notice {
                                SystemTray::read_distance_dialog_output(&evt_ui);
                            }
                        E::OnContextMenu =>
                            if &handle == &evt_ui.tray {
                                SystemTray::show_menu(&evt_ui);
                            }
                        E::OnMenuHover => {
                            if &handle == &evt_ui.screen_menu {
                                // TODO: Maybe we can listen for monitor changes instead of updating everything on hover?
                                update_screens_submenu(&*evt_ui);
                            }
                        },
                        E::OnMenuItemSelected => {
                            if &handle == &evt_ui.enabled_toggle {
                                SystemTray::toggle_enabled(&evt_ui);
                            }
                            else if &handle == &evt_ui.delay_30_menu {
                                SystemTray::do_delay(&evt_ui, Delays::ThirtySeconds)
                            }
                            else if &handle == &evt_ui.delay_1_menu {
                                SystemTray::do_delay(&evt_ui, Delays::OneMinute)
                            }
                            else if &handle == &evt_ui.delay_2_menu {
                                SystemTray::do_delay(&evt_ui, Delays::TwoMinutes)
                            }
                            else if &handle == &evt_ui.delay_5_menu {
                                SystemTray::do_delay(&evt_ui, Delays::FiveMinutes)
                            }
                            else if &handle == &evt_ui.delay_custom_menu {
                                SystemTray::do_delay(&evt_ui, Delays::Custom);
                            }
                            else if &handle == &evt_ui.distance_small_menu {
                                SystemTray::do_distance(&evt_ui, Distances::Small)
                            }
                            else if &handle == &evt_ui.distance_medium_menu {
                                SystemTray::do_distance(&evt_ui, Distances::Medium)
                            }
                            else if &handle == &evt_ui.distance_large_menu {
                                SystemTray::do_distance(&evt_ui, Distances::Large)
                            }
                            else if &handle == &evt_ui.distance_custom_menu {
                                SystemTray::do_distance(&evt_ui, Distances::Custom);
                            }
                            else if &handle == &evt_ui.exit_menu {
                                SystemTray::exit(&evt_ui);
                            }
                            else {
                                // Handle dynamically created screen menu items

                                // Iterate through the screens_items to find a matching handle
                                if let Some((device_id, _menu_item)) = evt_ui.screens_map.borrow().iter()
                                    .find_map(|(device_id, menu_item)| {
                                        if &handle == &menu_item.handle { Some((device_id.clone(), menu_item)) } else { None }
                                    }) {
                                    SystemTray::handle_monitor_selected(&evt_ui, &device_id);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            };

            ui.default_handler.borrow_mut().push(
                nwg::full_bind_event_handler(&ui.window.handle, handle_events)
            );

            return Ok(ui);
        }
    }

    impl Drop for SystemTrayUi {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for SystemTrayUi {
        type Target = SystemTray;

        fn deref(&self) -> &SystemTray {
            &self.inner
        }
    }
}
