use iced::time::{every, Duration};
use iced::widget::Container;
use iced::{Element, Subscription, Task};
use libloading::{Library, Symbol};
use log::{error, trace};
use shared_types::{AppInterfacePtr, AppState, CreateFn, DestroyFn, Message};
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::time::SystemTime;
use time::{macros::format_description, OffsetDateTime};

/// Constructs a platform-specific path to a dynamic library file.
///
/// This function builds the full `PathBuf` to a compiled dynamic library
/// (e.g., `.dll`, `.so`, or `.dylib`) in the `target/debug/` directory
/// based on the provided logical library name.
///
/// # Arguments
///
/// * `lib_name` - The base name of the dynamic library without extension.
///
/// # Returns
///
/// A `PathBuf` pointing to the platform-appropriate dynamic library file.
fn make_lib_path(lib_name: &str) -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let target_folder = "target/";

    let extension = if cfg!(windows) {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    PathBuf::from(format!(
        "{}{}/{}.{}",
        target_folder, profile, lib_name, extension
    ))
}

/// Loads the library and extracts symbols from it.
unsafe fn load_library(
    lib_info: &LibInfo,
    app_state: AppState,
) -> Result<(Library, AppInterfacePtr, Option<DestroyFn>, SystemTime)> {
    if !cfg!(windows) {
        error!("The dynamic library loading is implement only for windows.");
        return Err(Error::new(
            ErrorKind::Other,
            "The dynamic library loading is implement only for windows.",
        ));
    }

    let metadata = match std::fs::metadata(&lib_info.path) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to read metadata: {}", e);
            return Err(e);
        }
    };

    let timestamp = match metadata.modified() {
        Ok(ts) => ts,
        Err(e) => {
            error!("Failed to get modified timestamp: {}", e);
            return Err(e);
        }
    };

    let timestamp_dt: OffsetDateTime = timestamp.into();
    let suffix = timestamp_dt
        .format(format_description!(
            "[year]-[month]-[day]_[hour]-[minute]-[second]"
        ))
        .unwrap();

    let load_lib_path = make_lib_path(format!("{}_{}", lib_info.name, suffix).as_str());

    trace!(
        "Copy from {} to {}",
        lib_info.path.display(),
        load_lib_path.to_str().unwrap()
    );

    if let Err(e) = fs::copy(&lib_info.path, &load_lib_path) {
        error!("Failed to copy library: {}", e);
        return Err(e);
    }

    let library = match unsafe { Library::new(&load_lib_path) } {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to load library: {}", e);
            return Err(e).map_err(|e| Error::new(ErrorKind::Other, e));
        }
    };

    let create_fn: Symbol<CreateFn> =
        match unsafe { library.get(lib_info.create_fn_name.as_bytes()) } {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "Failed to load symbol {} from library {}",
                    lib_info.create_fn_name,
                    load_lib_path.display()
                );
                return Err(e).map_err(|e| Error::new(ErrorKind::Other, e));
            }
        };

    let destroy_fn: Symbol<DestroyFn> =
        match unsafe { library.get(lib_info.destroy_fn_name.as_bytes()) } {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "Failed to load symbol {} from library {}",
                    lib_info.destroy_fn_name,
                    load_lib_path.display()
                );
                return Err(e).map_err(|e| Error::new(ErrorKind::Other, e));
            }
        };

    let destroy_fn_raw: DestroyFn = unsafe { *destroy_fn.into_raw() };

    let app_i = unsafe { create_fn(app_state) };

    if app_i.is_null() {
        error!("Failed to initialize the core app");
        return Err(Error::new(
            ErrorKind::Other,
            "Failed to initialize the core app",
        ));
    }

    Ok((library, app_i, Some(destroy_fn_raw), timestamp))
}

/// Contains metadata and symbol names for a dynamically loaded library.
///
/// `LibInfo` holds all the information needed to load and interface with a
/// dynamic application core, including its name, file path, and the names
/// of its FFI-exported creation and destruction functions.
#[derive(Clone)]
struct LibInfo {
    /// The logical name of the library (e.g., "app_core")
    name: String,

    /// The filesystem path to the dynamic library (.dll, .so, .dylib)
    path: PathBuf,

    /// The exported symbol name for the function creating the core instance
    create_fn_name: String,

    /// The exported symbol name for the function destroying the core instance
    destroy_fn_name: String,
}

/// Manages the main application shell responsible for loading, rendering,
/// and reloading the dynamically linked core application logic.
///
/// `ShellApp` handles the lifecycle of the dynamic library, keeps track of
/// the core's exported functions, and orchestrates hot-reload transitions.
pub struct ShellApp {
    /// A raw pointer to the current dynamic core instance.
    app_interface: AppInterfacePtr,

    /// Optional destructor function exported by the dynamic library.
    destroy_fn: Option<DestroyFn>,

    /// The currently loaded dynamic library, kept alive for symbol safety.
    _lib: Library,

    /// The timestamp of the last time the dynamic library file was modified.
    last_modified: SystemTime,

    /// Metadata and symbol names used to identify and load the dynamic core.
    lib_info: LibInfo,

    /// Whether to render a dummy (empty) UI while flushing old library memory.
    use_dummy_view: bool,
}

impl Drop for ShellApp {
    fn drop(&mut self) {
        log::trace!("Destroy the core");
        unsafe {
            ((self.destroy_fn).unwrap()(self.app_interface));
        }
    }
}

impl ShellApp {
    const LIB_NAME: &'static str = "app_core";
    const CREATE_SYMBOL: &'static str = "create_app";
    const DESTROY_SYMBOL: &'static str = "destroy_app";

    fn new() -> Self {
        let lib_path = make_lib_path(Self::LIB_NAME);
        let lib_info = LibInfo {
            name: Self::LIB_NAME.to_string(),
            path: lib_path,
            create_fn_name: Self::CREATE_SYMBOL.to_string(),
            destroy_fn_name: Self::DESTROY_SYMBOL.to_string(),
        };

        log::trace!("Initial library load");
        let (lib, logic_ptr, destroy_fn, modified) = unsafe {
            load_library(&lib_info, AppState { counter: 0 })
                .expect("Failed to load initial library")
        };

        log::trace!("Library loaded");

        Self {
            app_interface: logic_ptr,
            destroy_fn,
            _lib: lib,
            last_modified: modified,
            lib_info,
            use_dummy_view: false,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Reload => {
                let current_state = unsafe { (**self.app_interface).state().clone() };

                log::trace!("Reload library");
                if let Ok((lib, logic_ptr, destroy_fn, modified)) =
                    unsafe { load_library(&self.lib_info, current_state) }
                {
                    log::trace!("Library reloaded");
                    *self = Self {
                        app_interface: logic_ptr,
                        destroy_fn,
                        _lib: lib,
                        last_modified: modified,
                        lib_info: self.lib_info.clone(),
                        use_dummy_view: false,
                    }
                }
            }
            Message::Tick => {
                if let Ok(modified) =
                    std::fs::metadata(&self.lib_info.path).and_then(|m| m.modified())
                {
                    if modified > self.last_modified {
                        self.use_dummy_view = true;
                        return Task::done(Message::Reload);
                    }
                }
            }
            _ => unsafe {
                (**self.app_interface).update(message);
            },
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        if self.use_dummy_view {
            // To reload the core we need to force iced to release memory allocated in the core
            // before the actual reload. To do that we return empty view here.
            return Container::new("").into();
        }

        unsafe { (**self.app_interface).view() }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        every(Duration::from_secs(1)).map(|_| Message::Tick)
    }
}

impl Default for ShellApp {
    fn default() -> Self {
        Self::new()
    }
}
