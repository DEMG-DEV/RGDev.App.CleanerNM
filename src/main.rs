mod app;
mod scanner;

use app::CleanerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cleaner")
            .with_inner_size([1060.0, 720.0])
            .with_min_inner_size([800.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Cleaner",
        options,
        Box::new(|cc| Ok(Box::new(CleanerApp::new(cc)))),
    )
}
