use std::{thread, cell::RefCell};
use nwg::{ControlHandle, NativeUi};

#[derive(Default)]
pub struct SpinDialog {
    window: nwg::Window,
    icon: nwg::Icon,
    label: nwg::Label,
    number_select: nwg::NumberSelect,
    data: RefCell<Option<String>>,
    ok_button: nwg::Button,
    cancel_button: nwg::Button,
}

impl SpinDialog {

    /// Create the dialog UI on a new thread. The dialog result will be returned by the thread handle.
    /// To alert the main GUI that the dialog completed, this function takes a notice sender object.
    pub(crate) fn popup(sender: nwg::NoticeSender) -> thread::JoinHandle<String> {
        return thread::spawn(move || {
            // Create the UI just like in the main function
            let app = SpinDialog::build_ui(Default::default()).expect("Failed to build UI");
            nwg::dispatch_thread_events();

            // Notice the main thread that the dialog completed
            sender.notice();

            // Return the dialog data
            app.data.take().unwrap_or("Cancelled!".to_owned())
        })
    }

    fn choose(&self, btn: &ControlHandle) {
        let mut data = self.data.borrow_mut();
        if btn == &self.ok_button {
            let value = self.number_select.data();
            *data = Some(value.formatted_value());
        }
        else if btn == &self.cancel_button {
            *data = Some("Cancelled!".to_owned());
        }

        self.window.close();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

mod number_select_app_ui {
    use native_windows_gui as nwg;
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::ops::Deref;

    pub struct SpinDialogUI {
        inner: Rc<SpinDialog>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl NativeUi<SpinDialogUI> for SpinDialog {
        fn build_ui(mut data: SpinDialog) -> Result<SpinDialogUI, nwg::NwgError> {
            // Resources
            nwg::Icon::builder()
                .source_bin(Some(include_bytes!("cog.ico")))
                .build(&mut data.icon)?;

            // Controls
            nwg::Window::builder()
                .size((320, 70))
                .center(true)
                .title("Delay Select Dialog")
                .icon(Some(&data.icon))
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::POPUP)
                .build(&mut data.window)?;

            let mut grid = nwg::GridLayout::default();
            nwg::GridLayout::builder()
                .parent(&data.window)
                .spacing(1)
                .build(&mut grid)?;

            nwg::Label::builder()
                .text("Value, in milliseconds:")
                .parent(&data.window)
                .build(&mut data.label)?;

            nwg::NumberSelect::builder()
                .size((152, 27))
                .decimals( 0)
                .min_int(200)
                .value_int(30000)
                .parent(&data.window)
                .build(&mut data.number_select)?;

            nwg::Button::builder()
                .text("Ok")
                .parent(&data.window)
                .build(&mut data.ok_button)?;

            nwg::Button::builder()
                .text("Cancel")
                .parent(&data.window)
                .build(&mut data.cancel_button)?;

            grid.add_child(0, 0, &data.label);
            grid.add_child(1, 0, &data.number_select);
            grid.add_child(0, 1, &data.ok_button);
            grid.add_child(1, 1, &data.cancel_button);

            // Wrap-up
            let ui = SpinDialogUI {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            use nwg::Event as E;

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle: ControlHandle| {
                if let Some(ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnButtonClick => {
                            if &handle == &ui.ok_button || &handle == &ui.cancel_button {
                                SpinDialog::choose(&ui, &handle);
                            }
                        }
                        E::OnWindowClose => {
                            if &handle == &ui.window {
                                SpinDialog::exit(&ui);
                            }
                        }
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

    impl Drop for SpinDialogUI {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for SpinDialogUI {
        type Target = SpinDialog;

        fn deref(&self) -> &SpinDialog {
            &self.inner
        }
    }
}
