#![forbid(unsafe_code)]

// Native desktop launcher; all pages are Rust egui widgets, not a WebView.
fn main() -> eframe::Result {
    #[cfg(target_os = "ios")]
    let options = eframe::NativeOptions {
        run_and_return: false,
        ..Default::default()
    };
    #[cfg(not(target_os = "ios"))]
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([640.0, 460.0]),
        ..Default::default()
    };
    v2rayq_client::run_with_native_options(options)
}
