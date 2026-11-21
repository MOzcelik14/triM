#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! triM. Desktop Video Editor main entry point.

use eframe::egui;
use std::path::Path;
use std::sync::Arc;
use trim_ui::TrimApp;

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log_dir = trim_core::paths::config_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let crash_path = log_dir.join("crash.log");
        let bt = std::backtrace::Backtrace::force_capture();
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<bilinmeyen konum>".to_string());
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Bilinmeyen panic yükü".to_string()
        };
        let entry = format!(
            "=== triM. v{} PANIC ===\nKonum: {}\nMesaj: {}\nBacktrace:\n{}\n\n",
            env!("CARGO_PKG_VERSION"),
            loc,
            msg,
            bt
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
        {
            use std::io::Write;
            let _ = f.write_all(entry.as_bytes());
        }
        log::error!("triM. panic at {}: {}", loc, msg);
        default_hook(info);
    }));
}

#[derive(Default)]
struct CliArgs {
    show_version: bool,
    show_help: bool,
    no_splash: bool,
    no_welcome: bool,
    open_shortcuts: bool,
    open_mixer: bool,
    open_export: bool,
    enable_safe_zones: bool,
    tab_effects: bool,
    select_v1: bool,
    import_srt: Option<String>,
    files: Vec<String>,
}

fn parse_cli_args<I: IntoIterator<Item = String>>(args: I) -> CliArgs {
    let mut parsed = CliArgs::default();
    let mut iter = args.into_iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-v" | "--version" => parsed.show_version = true,
            "-h" | "--help" => parsed.show_help = true,
            "--no-splash" => parsed.no_splash = true,
            "--no-welcome" => parsed.no_welcome = true,
            "--open-shortcuts" => parsed.open_shortcuts = true,
            "--open-mixer" => parsed.open_mixer = true,
            "--open-export" => parsed.open_export = true,
            "--enable-safe-zones" => parsed.enable_safe_zones = true,
            "--tab-effects" => parsed.tab_effects = true,
            "--select-v1" => parsed.select_v1 = true,
            "--import-srt" => {
                if let Some(val) = iter.next() {
                    parsed.import_srt = Some(val);
                }
            }
            _ if arg.starts_with("--") => {
                log::warn!("Bilinmeyen CLI seçeneği yoksayıldı: {}", arg);
            }
            _ => parsed.files.push(arg),
        }
    }
    parsed
}

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    install_panic_hook();

    let cli = parse_cli_args(std::env::args());
    if cli.show_version {
        println!("triM. v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if cli.show_help {
        println!(
            "triM. v{} - Hafif Masaüstü Video Düzenleyici (Rust NLE)",
            env!("CARGO_PKG_VERSION")
        );
        println!("\nKullanım: trim [SEÇENEKLER] [DOSYA...]");
        println!("\nSeçenekler:");
        println!("  -v, --version       Sürüm bilgisini göster");
        println!("  -h, --help          Bu yardım mesajını göster");
        println!("  --no-splash         Açılış ekranını göstermeden başlat");
        println!("  --no-welcome        Karşılama ekranını göstermeden başlat");
        println!("  --import-srt <YOL>  Başlangıçta SRT altyazı dosyasını içe aktar");
        return Ok(());
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

    eframe::run_native(
        "triM.",
        native_options,
        Box::new(move |cc| {
            let mut app = TrimApp::new(cc);
            for file_str in &cli.files {
                let p = Path::new(file_str);
                if p.exists() {
                    app.open_path(p);
                    app.welcome_dialog.close();
                }
            }
            if cli.no_splash {
                app.splash_screen.dismiss();
            }
            if cli.no_welcome {
                app.welcome_dialog.close();
            }
            if cli.open_shortcuts {
                app.welcome_dialog.close();
                app.shortcuts_dialog.open();
            }
            if cli.open_mixer {
                app.welcome_dialog.close();
                app.audio_mixer_dialog.open();
            }
            if cli.open_export {
                app.welcome_dialog.close();
                app.export_dialog.open(&app.project);
            }
            if cli.enable_safe_zones {
                app.show_safe_zones = true;
            }
            if let Some(ref srt_file) = cli.import_srt {
                if let Ok(content) = std::fs::read_to_string(srt_file) {
                    if let Ok(entries) = trim_core::parse_srt(&content) {
                        let clips = trim_core::subtitle_entries_to_clips(&entries, 48.0);
                        let target_track_id = app.project.timeline.add_new_video_track();
                        if let Some(tr) = app.project.timeline.get_track_by_id_mut(&target_track_id)
                        {
                            tr.name = "V2 (Altyazı)".to_string();
                            for clip in clips {
                                tr.add_clip(clip);
                            }
                        }
                    }
                }
            }
            if cli.tab_effects {
                app.active_left_tab = trim_ui::LeftPanelTab::Effects;
            }
            if cli.select_v1 {
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
