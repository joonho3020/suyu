mod app;
mod model;
mod text_format;

fn main() -> eframe::Result<()> {
    let icon = load_icon();
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "SanSuYu",
        native_options,
        Box::new(|cc| Ok(Box::new(app::DiagramApp::new(cc)))),
    )
}

fn load_icon() -> eframe::egui::IconData {
    let icon_path = "icon.png";
    if let Ok(image_data) = std::fs::read(icon_path) {
        if let Ok(image) = image::load_from_memory(&image_data) {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            return eframe::egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            };
        }
    }
    eframe::egui::IconData::default()
}
