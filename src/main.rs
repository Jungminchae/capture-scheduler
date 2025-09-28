mod app;
mod config;
mod scheduler;

use app::CaptureSchedulerApp;
use eframe::NativeOptions;

fn main() -> Result<(), eframe::Error> {
    let native_options = NativeOptions {
        run_and_return: true,
        ..Default::default()
    };

    eframe::run_native(
        "Capture Scheduler",
        native_options,
        Box::new(|cc| Box::new(CaptureSchedulerApp::new(cc))),
    )
}
