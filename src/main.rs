mod app;
mod import;
mod parser;
mod viewer2d;
mod viewer3d;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Geo Ring Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "Geo Ring Viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::GeoApp::new(cc)))),
    )
}
