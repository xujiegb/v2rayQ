#![forbid(unsafe_code)]

mod ui;

pub use ui::ClientApp;

/// Uses the same native egui application on every supported renderer.
/// On Android the host supplies an AndroidApp; iOS requires an Xcode host.
pub fn run_with_native_options(options: eframe::NativeOptions) -> eframe::Result {
    eframe::run_native(
        "v2rayQ",
        options,
        Box::new(|context| Ok(Box::new(ClientApp::new(context)))),
    )
}
