//! triM. Desktop Video Editor main entry point.

use eframe::egui;
use std::path::Path;
use std::sync::Arc;
use trim_ui::TrimApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting triM. v0.3.0 (Rust Desktop NLE)...");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("triM.")
        .with_inner_size([1366.0, 850.0])
        .with_min_inner_size([1024.0, 600.0]);

    // Load window icon if present
    let icon_path = Path::new("trim/resources/icons/trim_256x256.png");
    if icon_path.is_file() {
        if let Ok(img) = image::open(icon_path) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let icon_data = egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            };
            viewport = viewport.with_icon(Arc::new(icon_data));
        }
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let initial_file = std::env::args().nth(1);

    eframe::run_native(
        "triM.",
        native_options,
        Box::new(move |cc| {
            let mut app = TrimApp::new(cc);
            if let Some(file_str) = initial_file {
                let p = Path::new(&file_str);
                if p.exists() {
                    app.open_path(p);
                }
            }
            Ok(Box::new(app))
        }),
    )
}
