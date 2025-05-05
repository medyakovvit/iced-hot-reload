use iced::widget::{button, column, Text};
use iced::{Alignment, Element, Length};
use log::trace;
use log::LevelFilter;
use shared_types::{AppInterface, AppState, Message};
use simplelog::{ConfigBuilder, SimpleLogger};

/// The implementation of the AppInterface
#[repr(C)]
pub struct CoreApp {
    pub state: AppState,
}

impl AppInterface for CoreApp {
    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                trace!("Increment!");
                self.state.counter += 1
            }
            Message::Decrement => {
                trace!("Decrement!");
                self.state.counter -= 1
            }
            Message::Reload => (), // handled in the ShellApp
            Message::Tick => (),
        }
    }

    fn view(&self) -> Element<'static, Message> {
        iced::widget::Container::new(
            column![
                button("+").on_press(Message::Increment),
                Text::new(format!("Counter: {}", self.state.counter)),
                button("-").on_press(Message::Decrement),
            ]
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .into()
    }

    fn state(&self) -> &AppState {
        &self.state
    }
}

/// Creates the CoreApp instance with initial state `state`.
#[unsafe(no_mangle)]
pub extern "C" fn create_app(state: AppState) -> *mut Box<dyn AppInterface> {
    let log_config = ConfigBuilder::new()
        .set_max_level(LevelFilter::Trace)
        .set_time_level(LevelFilter::Trace)
        .add_filter_allow_str("app_core")
        .add_filter_allow_str("app_shell")
        .build();

    let _ = SimpleLogger::init(LevelFilter::Trace, log_config);

    trace!("Create app");
    let app = CoreApp { state };

    let boxed: Box<dyn AppInterface> = Box::new(app);
    Box::into_raw(Box::new(boxed))
}

/// Destoroys the memory allocated for the core instance.
#[unsafe(no_mangle)]
pub extern "C" fn destroy_app(ptr: *mut Box<dyn AppInterface>) {
    trace!("Destroy app");
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}
