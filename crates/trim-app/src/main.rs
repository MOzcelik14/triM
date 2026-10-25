#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! triM. Desktop Video Editor main entry point.

use eframe::egui;
use std::path::Path;
use std::sync::Arc;
use trim_ui::TrimApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("triM. v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "triM. v{} - Profesyonel Masaüstü Video Düzenleyici (Rust NLE)",
                    env!("CARGO_PKG_VERSION")
                );
                println!("\nKullanım: trim [DOSYA]");
                println!("\nSeçenekler:");
                println!("  -v, --version    Sürüm bilgisini göster");
                println!("  -h, --help       Bu yardım mesajını göster");
                return Ok(());
            }
            _ => {}
        }
    }

    log::info!(
        "Starting triM. v{} (Rust Desktop NLE)...",
        env!("CARGO_PKG_VERSION")
    );

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("triM. v{}", env!("CARGO_PKG_VERSION")))
        .with_inner_size([1366.0, 850.0])
        .with_min_inner_size([1024.0, 600.0]);

    // Embed the icon: installed builds cannot rely on the current directory.
    let icon_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/icons/trim_256x256.png"
    ));
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let icon_data = egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        };
        viewport = viewport.with_icon(Arc::new(icon_data));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let initial_file = std::env::args().skip(1).find(|arg| !arg.starts_with("--"));

    eframe::run_native(
        "triM.",
        native_options,
        Box::new(move |cc| {
            let mut app = TrimApp::new(cc);
            if let Some(file_str) = initial_file {
                let p = Path::new(&file_str);
                if p.exists() {
                    app.open_path(p);
                    app.welcome_dialog.close();
                }
            }
            if std::env::args().any(|arg| arg == "--no-welcome") {
                app.welcome_dialog.close();
            }
            if std::env::args().any(|arg| arg == "--open-shortcuts") {
                app.welcome_dialog.close();
                app.shortcuts_dialog.open();
            }
            if std::env::args().any(|arg| arg == "--open-mixer") {
                app.welcome_dialog.close();
                app.audio_mixer_dialog.open();
            }
            if std::env::args().any(|arg| arg == "--open-export") {
                app.welcome_dialog.close();
                app.export_dialog.open(&app.project);
            }
            if std::env::args().any(|arg| arg == "--enable-safe-zones") {
                app.show_safe_zones = true;
            }
            if let Some(srt_idx) = std::env::args().position(|arg| arg == "--import-srt") {
                if let Some(srt_file) = std::env::args().nth(srt_idx + 1) {
                    if let Ok(content) = std::fs::read_to_string(&srt_file) {
                        if let Ok(entries) = trim_core::parse_srt(&content) {
                            let clips = trim_core::subtitle_entries_to_clips(&entries, 48.0);
                            let target_track_id = app.project.timeline.add_new_video_track();
                            if let Some(tr) =
                                app.project.timeline.get_track_by_id_mut(&target_track_id)
                            {
                                tr.name = "V2 (Altyazı)".to_string();
                                for clip in clips {
                                    tr.add_clip(clip);
                                }
                            }
                        }
                    }
                }
            }
            if std::env::args().any(|arg| arg == "--tab-effects") {
                app.active_left_tab = trim_ui::LeftPanelTab::Effects;
            }
            if std::env::args().any(|arg| arg == "--select-v1") {
                app.active_left_tab = trim_ui::LeftPanelTab::Effects;
                if let Some(t) = app.project.timeline.tracks.iter().find(|t| t.name == "V1") {
                    if let Some(c) = t.clips.first() {
                        app.canvas_state.selected_clip_pairs = vec![(t.id.clone(), c.id.clone())];
                        app.current_time = 1.0;
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}
