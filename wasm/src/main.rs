use wasm::App;

// #[tokio::main(flavor = "current_thread")]
fn main() -> eframe::Result<()> {
    let app = App::default();
    /*
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    */

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(app)),
    )
}
