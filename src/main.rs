mod app;
mod model;
mod text_format;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Diagramming",
        native_options,
        Box::new(|cc| Ok(Box::new(app::DiagramApp::new(cc)))),
    )
}
