use std::{thread, cell::RefCell};
use nwg::{ControlHandle, NativeUi, NumberSelectData};

pub enum DelayDialogData {
    Cancel,
    Value(i32),
}

#[derive(Default)]
pub struct DelayDialog {
    window: nwg::Window,
    icon: nwg::Icon,
    label: nwg::Label,
    number_select: nwg::NumberSelect,
    data: RefCell<Option<DelayDialogData>>,
    ok_button: nwg::Button,
    cancel_button: nwg::Button,
}

impl DelayDialog {

    /// Create the dialog UI on a new thread. The dialog result will be returned by the thread handle.
    /// To alert the main GUI that the dialog completed, this function takes a notice sender object.
    pub(crate) fn popup(sender: nwg::NoticeSender, current_value: i32) -> thread::JoinHandle<DelayDialogData> {
        return thread::spawn(move || {
            // Create the UI just like in the main function
            let app = DelayDialog::build_ui(Default::default()).expect("Failed to build UI");

            let number_select_data = NumberSelectData::Int {
                value: (current_value / 1000) as i64,
                step: 1,    // 1 second
                max: 1800,  // 30 minutes
                min: 5,     // 5 seconds
            };
            app.number_select.set_data(number_select_data);

            nwg::dispatch_thread_events();

            // Notice the main thread that the dialog completed
            sender.notice();

            // Return the dialog data
            return app.data.take().unwrap_or(DelayDialogData::Cancel)
        })
    }

    fn choose(&self, btn: &ControlHandle) {
        let mut data = self.data.borrow_mut();
        if btn == &self.ok_button {
            let value = self.number_select.data();
            if let Ok(parsed_value) = value.formatted_value().parse::<i32>() {
                *data = Some(DelayDialogData::Value(parsed_value.abs() * 1000));
            } else {
                // TODO: Handle the error, if any
                println!("Failed to parse value!");
                *data = Some(DelayDialogData::Cancel);
            }
        } else if btn == &self.cancel_button {
            *data = Some(DelayDialogData::Cancel);
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
    use crate::view::ICON;

    pub struct SpinDialogUI {
        inner: Rc<DelayDialog>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl NativeUi<SpinDialogUI> for DelayDialog {
        fn build_ui(mut data: DelayDialog) -> Result<SpinDialogUI, nwg::NwgError> {
            // Resources
            nwg::Icon::builder()
                .source_bin(Option::from(ICON))
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
                .text("Value, in seconds:")
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
                                DelayDialog::choose(&ui, &handle);
                            }
                        }
                        E::OnWindowClose => {
                            if &handle == &ui.window {
                                DelayDialog::exit(&ui);
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
        type Target = DelayDialog;

        fn deref(&self) -> &DelayDialog {
            &self.inner
        }
    }
}
