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

/// Mobile host may supply an Android/iOS application sandbox path without
/// changing the app's presentation or service logic.
pub fn run_with_native_options_and_data_path(
    options: eframe::NativeOptions,
    data_path: std::path::PathBuf,
) -> eframe::Result {
    eframe::run_native(
        "v2rayQ",
        options,
        Box::new(move |context| Ok(Box::new(ClientApp::new_with_data_path(context, data_path)))),
    )
}
