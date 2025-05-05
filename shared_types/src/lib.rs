use iced::Element;

pub type AppInterfacePtr = *mut Box<dyn AppInterface>;
pub type CreateFn = unsafe extern "C" fn(AppState) -> AppInterfacePtr;
pub type DestroyFn = unsafe extern "C" fn(AppInterfacePtr);

/// All UI events/messages passed between shell and core.
#[repr(C)]
#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Increment,
    Decrement,
    Reload,
}

/// The state of the application
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AppState {
    pub counter: i32,
}

/// Represents the contract between app and core.
pub trait AppInterface {
    fn update(&mut self, message: Message);
    fn view(&self) -> Element<'static, Message>;
    fn state(&self) -> &AppState;
}
