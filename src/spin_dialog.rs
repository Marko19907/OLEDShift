use std::{thread, cell::RefCell, i32};
use nwg::{ControlHandle, NativeUi, NumberSelectData, stretch::{geometry::Size, style::{Dimension as D, FlexDirection, AlignSelf}}};

pub enum SpinDialogData {
    Cancel,
    Value(i32),
}

#[derive(Default)]
pub struct SpinDialog {
    window: nwg::Window,
    window_box: nwg::FlexboxLayout,
    content_box: nwg::FlexboxLayout,
    button_box: nwg::FlexboxLayout,
    icon: nwg::Icon,
    label: nwg::Label,
    number_select: nwg::NumberSelect,
    data: RefCell<Option<SpinDialogData>>,
    ok_button: nwg::Button,
    cancel_button: nwg::Button,
}

impl SpinDialog {

    /// Create the dialog UI on a new thread. The dialog result will be returned by the thread handle.
    /// To alert the main GUI that the dialog completed, this function takes a notice sender object.
    pub(crate) fn popup(sender: nwg::NoticeSender, current_value: i32) -> thread::JoinHandle<SpinDialogData> {
        return thread::spawn(move || {
            // Create the UI just like in the main function
            let app = SpinDialog::build_ui(Default::default()).expect("Failed to build UI");

            let number_select_data = NumberSelectData::Int {
                value: current_value as i64,
                step: 1,
                max: i64::MAX,
                min: i64::MIN,
            };
            app.number_select.set_data(number_select_data);

            nwg::dispatch_thread_events();

            // Notice the main thread that the dialog completed
            sender.notice();

            // Return the dialog data
            return app.data.take().unwrap_or(SpinDialogData::Cancel)
        })
    }

    fn choose(&self, btn: &ControlHandle) {
        let mut data = self.data.borrow_mut();
        if btn == &self.ok_button {
            let value = self.number_select.data();
            if let Ok(parsed_value) = value.formatted_value().parse::<i32>() {
                *data = Some(SpinDialogData::Value(parsed_value.abs()));
            } else {
                // TODO: Handle the error, if any
                println!("Failed to parse value!");
                *data = Some(SpinDialogData::Cancel);
            }
        } else if btn == &self.cancel_button {
            *data = Some(SpinDialogData::Cancel);
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
    use nwg::stretch::geometry::Rect;
    use nwg::stretch::style::{AlignItems, JustifyContent};
    use nwg::stretch::style::Dimension::Auto;

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
                .topmost(true)
                .center(true)
                .title("Delay Select Dialog")
                .icon(Some(&data.icon))
                // .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE | nwg::WindowFlags::POPUP)
                .build(&mut data.window)?;

            nwg::Label::builder()
                .text("Value, in milliseconds:")
                // .background_color(Option::from([56, 56, 56]))
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

            // Layouts
            const PC_50: D = D::Percent(0.5);
            const PT_50: D = D::Points(50.0);

            const PT_30: D = D::Points(30.0);

            const PT_10: D = D::Points(10.0);
            const PT_5: D = D::Points(5.0);
            const PADDING: Rect<D> = Rect{ start: PT_10, end: PT_10, top: PT_10, bottom: PT_10 };
            const MARGIN: Rect<D> = Rect{ start: PT_5, end: PT_5, top: PT_5, bottom: PT_5 };

            nwg::FlexboxLayout::builder()
                .parent(&data.window)
                .flex_direction(FlexDirection::Row)
                .child(&data.label)
                    .child_size(Size { width: PC_50, height: PT_50 })
                .child(&data.number_select)
                    .child_flex_grow(1.0)
                    .child_min_size(Size { width: PT_50, height: PT_30 })
                // .child_align_self(AlignSelf::FlexEnd)
                .justify_content(JustifyContent::Center)
                .build_partial(&mut data.content_box)?;

            nwg::FlexboxLayout::builder()
                .parent(&data.window)
                .flex_direction(FlexDirection::Row)
                .child(&data.ok_button)
                    .child_size(Size { width: Auto, height: PT_30 })
                    .child_flex_grow(1.0)
                .child(&data.cancel_button)
                    .child_size(Size { width: Auto, height: PT_30 })
                    .child_flex_grow(1.0)
                .build_partial(&mut data.button_box)?;

            nwg::FlexboxLayout::builder()
                .parent(&data.window)
                .flex_direction(FlexDirection::Column)
                .padding(PADDING)
                .child_layout(&data.content_box)
                    // .child_flex_grow(1.0)
                .child_layout(&data.button_box)
                // .child_flex_grow(0.0)
                .build(&mut data.window_box)?;

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
