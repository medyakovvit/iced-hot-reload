mod shellapp;

use log::LevelFilter;
use shellapp::ShellApp;
use simplelog::{ConfigBuilder, SimpleLogger};

fn main() -> iced::Result {
    let log_config = ConfigBuilder::new()
        .set_max_level(LevelFilter::Trace)
        .set_time_level(LevelFilter::Trace)
        .add_filter_allow_str("app_core")
        .add_filter_allow_str("app_shell")
        .build();

    let _ = SimpleLogger::init(LevelFilter::Trace, log_config);

    iced::application("Application", ShellApp::update, ShellApp::view)
        .subscription(ShellApp::subscription)
        .run()
}
