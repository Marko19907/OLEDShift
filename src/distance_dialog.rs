use crate::mover;
use std::{thread, cell::RefCell};
use nwg::{ControlHandle, NativeUi, NumberSelectData};
use crate::settings::LOWEST_MAX_DISTANCE;

pub enum DistanceDialogData {
    Cancel,
    Value(i32, i32),
}

#[derive(Default)]
pub struct DistanceDialog {
    window: nwg::Window,
    icon: nwg::Icon,
    label_x: nwg::Label,
    label_y: nwg::Label,
    number_select_x: nwg::NumberSelect,
    number_select_y: nwg::NumberSelect,
    data: RefCell<Option<DistanceDialogData>>,
    ok_button: nwg::Button,
    cancel_button: nwg::Button,
}

impl DistanceDialog {

    /// Create the dialog UI on a new thread. The dialog result will be returned by the thread handle.
    /// To alert the main GUI that the dialog completed, this function takes a notice sender object.
    pub(crate) fn popup(sender: nwg::NoticeSender, current_value_x: i32, current_value_y: i32) -> thread::JoinHandle<DistanceDialogData> {
        return thread::spawn(move || {
            // Create the UI just like in the main function
            let app = DistanceDialog::build_ui(Default::default()).expect("Failed to build UI");

            let (smallest_x, smallest_y) = mover::get_smallest_screen_size().unwrap_or((400, 400));

            let number_select_data_x = NumberSelectData::Int {
                value: current_value_x as i64,
                step: 1,
                max: smallest_x as i64 / 4,
                min: LOWEST_MAX_DISTANCE as i64,
            };
            app.number_select_x.set_data(number_select_data_x);

            let number_select_data_y = NumberSelectData::Int {
                value: current_value_y as i64,
                step: 1,
                max: smallest_y as i64 / 4,
                min: LOWEST_MAX_DISTANCE as i64,
            };
            app.number_select_y.set_data(number_select_data_y);

            nwg::dispatch_thread_events();

            // Notice the main thread that the dialog completed
            sender.notice();

            // Return the dialog data
            return app.data.take().unwrap_or(DistanceDialogData::Cancel)
        })
    }

    fn choose(&self, btn: &ControlHandle) {
        let mut data = self.data.borrow_mut();
        if btn == &self.ok_button {
            let value_x = self.number_select_x.data();
            let value_y = self.number_select_y.data();

            let value_x = value_x.formatted_value().parse::<i32>();
            let value_y = value_y.formatted_value().parse::<i32>();

            if value_x.is_ok() && value_y.is_ok() {
                *data = Some(DistanceDialogData::Value(value_x.unwrap(), value_y.unwrap()));
            } else {
                // TODO: Handle the error, if any
                println!("Failed to parse value!");
                *data = Some(DistanceDialogData::Cancel);
            }
        } else if btn == &self.cancel_button {
            *data = Some(DistanceDialogData::Cancel);
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

    pub struct DistanceDialogUI {
        inner: Rc<DistanceDialog>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl NativeUi<DistanceDialogUI> for DistanceDialog {
        fn build_ui(mut data: DistanceDialog) -> Result<DistanceDialogUI, nwg::NwgError> {
            // Resources
            nwg::Icon::builder()
                .source_bin(Option::from(ICON))
                .build(&mut data.icon)?;

            // Controls
            nwg::Window::builder()
                .size((320, 100))
                .center(true)
                .title("Distance Dialog")
                .icon(Some(&data.icon))
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::POPUP)
                .build(&mut data.window)?;

            let mut grid = nwg::GridLayout::default();
            nwg::GridLayout::builder()
                .parent(&data.window)
                .spacing(1)
                .build(&mut grid)?;

            nwg::Label::builder()
                .text("Max distance x (pixels):")
                .parent(&data.window)
                .build(&mut data.label_x)?;

            nwg::Label::builder()
                .text("Max distance y (pixels):")
                .parent(&data.window)
                .build(&mut data.label_y)?;

            nwg::NumberSelect::builder()
                .size((152, 27))
                .decimals( 0)
                .parent(&data.window)
                .build(&mut data.number_select_x)?;

            nwg::NumberSelect::builder()
                .size((152, 27))
                .decimals( 0)
                .parent(&data.window)
                .build(&mut data.number_select_y)?;

            nwg::Button::builder()
                .text("Ok")
                .parent(&data.window)
                .build(&mut data.ok_button)?;

            nwg::Button::builder()
                .text("Cancel")
                .parent(&data.window)
                .build(&mut data.cancel_button)?;

            grid.add_child(0, 0, &data.label_x);
            grid.add_child(1, 0, &data.number_select_x);
            grid.add_child(0, 1, &data.label_y);
            grid.add_child(1, 1, &data.number_select_y);
            grid.add_child(0, 2, &data.ok_button);
            grid.add_child(1, 2, &data.cancel_button);

            // Wrap-up
            let ui = DistanceDialogUI {
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
                                DistanceDialog::choose(&ui, &handle);
                            }
                        }
                        E::OnWindowClose => {
                            if &handle == &ui.window {
                                DistanceDialog::exit(&ui);
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

    impl Drop for DistanceDialogUI {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for DistanceDialogUI {
        type Target = DistanceDialog;

        fn deref(&self) -> &DistanceDialog {
            &self.inner
        }
    }
}
