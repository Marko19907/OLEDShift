use std::sync::{Arc, Mutex};
use crate::controller::Controller;

#[derive(Default)]
pub struct SystemTray {
    window: nwg::MessageWindow,
    icon: nwg::Icon,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    enabled_toggle: nwg::MenuItem,
    tray_item1: nwg::MenuItem,
    tray_item2: nwg::MenuItem,
    tray_item3: nwg::MenuItem,
    separator: nwg::MenuSeparator,
    controller: Arc<Mutex<Controller>>,
}

impl SystemTray {

    fn new(&self) -> Self {
        let controller = Arc::new(Mutex::new(Controller::new()));

        return SystemTray {
            controller,
            ..Default::default()
        }
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn toggle_enabled(&self) {
        println!("Toggling enabled called");
        self.controller.lock().unwrap().toggle_running();
    }

    fn hello1(&self) {
        nwg::modal_info_message(&self.window, "Hello", "Hello World!");
    }

    fn hello2(&self) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray.show("Hello World", Some("Welcome to my application"), Some(flags), Some(&self.icon));
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}


//
// ALL of this stuff is handled by native-windows-derive
//
mod system_tray_ui {
    use native_windows_gui as nwg;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::ops::Deref;
    use crate::controller::Controller;
    use crate::view::SystemTray;

    pub struct SystemTrayUi {
        inner: Rc<SystemTray>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl nwg::NativeUi<SystemTrayUi> for SystemTray {
        fn build_ui(mut data: SystemTray) -> Result<SystemTrayUi, nwg::NwgError> {
            use nwg::Event as E;

            // Resources
            nwg::Icon::builder()
                .source_bin(Some(include_bytes!("cog.ico")))
                .build(&mut data.icon)?;

            // Controls
            nwg::MessageWindow::builder()
                .build(&mut data.window)?;

            nwg::TrayNotification::builder()
                .parent(&data.window)
                .icon(Some(&data.icon))
                .tip(Some("Hello"))
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

            nwg::MenuItem::builder()
                .text("Delay")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item1)?;

            nwg::MenuItem::builder()
                .text("Set max distance")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item2)?;

            nwg::MenuSeparator::builder()
                .parent(&data.tray_menu)
                .build(&mut data.separator)?;

            nwg::MenuItem::builder()
                .text("Exit")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item3)?;

            // Wrap-up
            let ui = SystemTrayUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Start the controller
            Controller::run(ui.inner.controller.clone());

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(evt_ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnContextMenu =>
                            if &handle == &evt_ui.tray {
                                SystemTray::show_menu(&evt_ui);
                            }
                        E::OnMenuItemSelected =>
                            if &handle == &evt_ui.enabled_toggle {
                                SystemTray::toggle_enabled(&evt_ui);
                            }
                            else if &handle == &evt_ui.tray_item1 {
                                SystemTray::hello1(&evt_ui);
                            } else if &handle == &evt_ui.tray_item2 {
                                SystemTray::hello2(&evt_ui);
                            } else if &handle == &evt_ui.tray_item3 {
                                SystemTray::exit(&evt_ui);
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
