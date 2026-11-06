//! Playback controls, frame stepping, shuttle speeds, and clock synchronization.

use crate::app::TrimApp;
use egui::{Color32, ColorImage, Pos2, Rect, RichText, TextureOptions, Vec2};
use std::time::Instant;
use trim_core::media::MediaType;

impl TrimApp {
    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.playback_speed = 1.0;
            self.play();
        }
    }

    pub fn play(&mut self) {
        let dur = self.project.timeline.duration();
        if dur > 0.0 {
            if self.playback_speed >= 0.0 && self.current_time >= dur {
                self.current_time = 0.0;
            } else if self.playback_speed < 0.0 && self.current_time <= 0.0 {
                self.current_time = dur;
            }
        }
        self.is_playing = true;
        if (self.playback_speed - 1.0).abs() < 0.01 {
            self.audio_player.prepare();
        } else {
            self.audio_player.stop();
        }
        self.clock_start = Some(Instant::now());
        self.clock_start_time = self.current_time;
    }

    pub fn pause(&mut self) {
        if self.active_voiceover.is_some() {
            self.finish_voiceover();
        }
        self.is_playing = false;
        self.playback_speed = 1.0;
        self.clock_start = None;
        self.video_engine.streamer.stop();
        self.audio_player.stop();
        self.audio_mixer.reset_limiter();
        self.vu_state.reset();
    }

    pub fn shuttle_stop(&mut self) {
        self.pause();
        self.status_message = "Duraklatıldı (1x)".to_string();
    }

    pub fn toggle_snap(&mut self) {
        self.snap_enabled = !self.snap_enabled;
        self.project.timeline.snap_enabled = self.snap_enabled;
        self.status_message = if self.snap_enabled {
            "Mıknatıs / Yapışma (N): Açık".to_string()
        } else {
            "Mıknatıs / Yapışma (N): Kapalı".to_string()
        };
    }

    pub fn stop(&mut self) {
        self.pause();
        self.seek(0.0);
    }

    pub fn seek(&mut self, time: f64) {
        self.video_engine.streamer.stop();
        self.audio_player.stop();
        self.audio_mixer.reset_limiter();
        let dur = self.project.timeline.duration();
        self.current_time = time.clamp(0.0, dur.max(0.0));
        if self.is_playing {
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
    }

    pub fn step_frame(&mut self, delta: i32) {
        self.pause();
        let fps = self.project.timeline.fps.max(1.0);
        let new_t = self.current_time + (delta as f64 / fps);
        self.seek(new_t);
    }

    pub fn shuttle_forward(&mut self) {
        let speeds = [1.0, 2.0, 4.0, 8.0];
        if !self.is_playing || self.playback_speed <= 0.0 {
            self.playback_speed = 1.0;
            self.play();
        } else {
            let next = speeds
                .iter()
                .find(|&&s| s > self.playback_speed + 0.1)
                .copied()
                .unwrap_or(8.0);
            self.playback_speed = next;
            if (self.playback_speed - 1.0).abs() >= 0.01 {
                self.audio_player.stop();
            }
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
        self.status_message = format!("İleri Sarma: {:.0}x", self.playback_speed);
    }

    pub fn shuttle_reverse(&mut self) {
        let speeds = [-1.0, -2.0, -4.0, -8.0];
        if !self.is_playing || self.playback_speed >= 0.0 {
            self.playback_speed = -1.0;
            self.play();
        } else {
            let next = speeds
                .iter()
                .find(|&&s| s < self.playback_speed - 0.1)
                .copied()
                .unwrap_or(-8.0);
            self.playback_speed = next;
            self.audio_player.stop();
            self.clock_start = Some(Instant::now());
            self.clock_start_time = self.current_time;
        }
        self.status_message = format!("Geri Sarma: {:.0}x", self.playback_speed);
    }

    pub fn update_playback_tick(&mut self) {
        if self.is_playing {
            if let Some(start) = self.clock_start {
                let elapsed = start.elapsed().as_secs_f64() * self.playback_speed;
                let cur = self.clock_start_time + elapsed;
                let dur = self.project.timeline.duration();

                if dur > 0.0 && (cur >= dur || cur < 0.0) {
                    self.current_time = cur.clamp(0.0, dur);
                    self.pause();
                } else {
                    let prev_t = self.current_time;
                    self.current_time = cur.max(0.0);

                    // Render mixed audio chunk & update VU levels & output to speakers
                    if (self.playback_speed - 1.0).abs() < 0.01 {
                        let (mut samples, mut db_l, mut db_r) = self
                            .audio_mixer
                            .render_audio_interval(&self.project, prev_t, self.current_time);
                        let master_vol = self.audio_mixer_dialog.master_volume;
                        if (master_vol - 1.0).abs() > 1e-4 {
                            for s in &mut samples {
                                *s = ((*s as f32) * master_vol)
                                    .round()
                                    .clamp(i16::MIN as f32, i16::MAX as f32)
                                    as i16;
                            }
                            if master_vol <= 0.0001 {
                                db_l = -60.0;
                                db_r = -60.0;
                            } else {
                                let gain_db = 20.0 * master_vol.log10();
                                db_l = (db_l + gain_db).clamp(-60.0, 0.0);
                                db_r = (db_r + gain_db).clamp(-60.0, 0.0);
                            }
                        }
                        self.vu_state.set_levels(db_l, db_r);
                        self.audio_player.write_samples(&samples);
                    }
                }
            }
        }
        self.vu_state.tick_decay();
    }

    pub fn update_monitor_frame(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(compositor) = &mut self.gpu_compositor {
            let layers = self.video_engine.collect_timeline_layers(
                &self.project,
                self.current_time,
                self.is_playing,
            );
            if layers.is_empty() {
                self.monitor_gpu_texture_id = None;
                self.monitor_texture = None;
                self.last_rendered_monitor_frame = None;
                return;
            }
            if let Some(tex_id) =
                compositor.render_layers(&self.project, &layers, self.current_time, frame)
            {
                self.monitor_gpu_texture_id = Some(tex_id);
                self.monitor_texture = None;
                return;
            }
            log::warn!("GPU Compositor çalışma zamanı hatası; CPU fallback moduna geçiliyor.");
            self.gpu_compositor = None;
        }

        self.monitor_gpu_texture_id = None;
        let frame_opt = self.video_engine.render_timeline_frame(
            &self.project,
            self.current_time,
            self.is_playing,
        );

        if let Some(frame) = frame_opt {
            if let Some(last) = &self.last_rendered_monitor_frame {
                if std::sync::Arc::ptr_eq(last, &frame) {
                    return;
                }
            }
            self.last_rendered_monitor_frame = Some(frame.clone());
            let color_image = ColorImage::from_rgba_unmultiplied(
                [frame.width() as usize, frame.height() as usize],
                frame.as_raw(),
            );
            if let Some(tex) = &mut self.monitor_texture {
                tex.set(color_image, TextureOptions::LINEAR);
            } else {
                self.monitor_texture =
                    Some(ctx.load_texture("monitor_preview", color_image, TextureOptions::LINEAR));
            }
        } else {
            self.last_rendered_monitor_frame = None;
            self.monitor_texture = None;
        }
    }

    pub fn update_source_monitor_frame(&mut self, ctx: &egui::Context) {
        let Some(ref m_id) = self.source_state.media_id.clone() else {
            self.source_state.texture = None;
            return;
        };
        let Some(media) = self.project.get_media(m_id) else {
            self.source_state.texture = None;
            return;
        };

        if self.source_state.is_playing {
            let dt = 1.0 / 30.0;
            self.source_state.current_time += dt;
            if self.source_state.current_time >= media.duration {
                self.source_state.current_time = self.source_state.mark_in.unwrap_or(0.0);
                self.source_state.is_playing = false;
            }
            ctx.request_repaint();
        }

        // Poll decoded result from async worker (zero UI blocking)
        if let Some(res) = self.source_state.decoder.poll_result() {
            if res.id == self.source_state.decoder.latest_req_id {
                if let Some(frame) = res.frame {
                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [frame.width() as usize, frame.height() as usize],
                        frame.as_raw(),
                    );
                    if let Some(tex) = &mut self.source_state.texture {
                        tex.set(color_image, TextureOptions::LINEAR);
                    } else {
                        self.source_state.texture = Some(ctx.load_texture(
                            "source_preview",
                            color_image,
                            TextureOptions::LINEAR,
                        ));
                    }
                    self.source_state.last_decoded_id = Some(m_id.clone());
                    self.source_state.last_decoded_time = Some(res.time);
                    ctx.request_repaint();
                }
            }
        }

        let needs_decode = match (
            &self.source_state.last_decoded_id,
            self.source_state.last_decoded_time,
        ) {
            (Some(id), Some(t)) => id != m_id || (t - self.source_state.current_time).abs() > 0.03,
            _ => true,
        };

        if needs_decode {
            if media.media_type == MediaType::Image {
                if let Ok(dyn_img) = image::open(&media.file_path) {
                    let frame = dyn_img.to_rgba8();
                    let color_image = ColorImage::from_rgba_unmultiplied(
                        [frame.width() as usize, frame.height() as usize],
                        frame.as_raw(),
                    );
                    if let Some(tex) = &mut self.source_state.texture {
                        tex.set(color_image, TextureOptions::LINEAR);
                    } else {
                        self.source_state.texture = Some(ctx.load_texture(
                            "source_preview",
                            color_image,
                            TextureOptions::LINEAR,
                        ));
                    }
                    self.source_state.last_decoded_id = Some(m_id.clone());
                    self.source_state.last_decoded_time = Some(self.source_state.current_time);
                }
            } else if media.media_type == MediaType::Video {
                self.source_state
                    .decoder
                    .request_seek(&media.file_path, self.source_state.current_time);
            }
        }
    }

    pub fn render_fullscreen_monitor(&mut self, ctx: &egui::Context) {
        // Exit shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::F11)) {
            self.is_monitor_fullscreen = false;
            return;
        }

        // Space to toggle play/pause
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_play();
        }
        // Left / Right arrow for frame step
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let fps = self.project.timeline.fps.max(1.0);
            self.seek((self.current_time - 1.0 / fps).max(0.0));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let fps = self.project.timeline.fps.max(1.0);
            let dur = self.project.timeline.duration();
            self.seek((self.current_time + 1.0 / fps).min(dur));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let avail_rect = ui.available_rect_before_wrap();
                let ar = (self.project.timeline.width as f32
                    / self.project.timeline.height.max(1) as f32)
                    .clamp(0.2, 5.0);

                let monitor_bounds = Rect::from_min_max(
                    avail_rect.min,
                    Pos2::new(avail_rect.max.x, avail_rect.max.y - 56.0),
                );

                let video_rect = {
                    let w = monitor_bounds.width();
                    let h = monitor_bounds.height();
                    let (vw, vh) = if w / h > ar { (h * ar, h) } else { (w, w / ar) };
                    Rect::from_center_size(monitor_bounds.center(), Vec2::new(vw, vh))
                };

                let painter = ui.painter();

                if let Some(tex) = &self.monitor_texture {
                    painter.image(
                        tex.id(),
                        video_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    painter.rect_filled(video_rect, 0.0, Color32::from_rgb(12, 12, 16));
                    painter.text(
                        video_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if self.is_playing {
                            "Oynatılıyor..."
                        } else {
                            "Önizleme Yükleniyor..."
                        },
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(0x80, 0x80, 0x90),
                    );
                }

                // Double click on screen toggles fullscreen
                let screen_resp = ui.interact(
                    avail_rect,
                    ui.id().with("fs_bg_interact"),
                    egui::Sense::click(),
                );
                if screen_resp.double_clicked() {
                    self.is_monitor_fullscreen = false;
                }

                // Top Exit Badge
                let exit_btn_rect = Rect::from_min_size(
                    Pos2::new(avail_rect.max.x - 180.0, avail_rect.min.y + 12.0),
                    Vec2::new(168.0, 28.0),
                );
                let is_hover_top = ui.rect_contains_pointer(exit_btn_rect.expand(20.0));
                let badge_alpha = if is_hover_top { 240 } else { 140 };

                painter.rect_filled(
                    exit_btn_rect,
                    6.0,
                    Color32::from_rgba_unmultiplied(20, 20, 28, badge_alpha),
                );
                painter.text(
                    exit_btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "ESC / F11: Tam Ekrandan Çık",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(220, 225, 235),
                );
                let exit_resp = ui.interact(
                    exit_btn_rect,
                    ui.id().with("fs_exit_btn"),
                    egui::Sense::click(),
                );
                if exit_resp.clicked() {
                    self.is_monitor_fullscreen = false;
                }

                // Bottom floating transport bar
                let bar_rect = Rect::from_min_max(
                    Pos2::new(avail_rect.min.x, avail_rect.max.y - 52.0),
                    avail_rect.max,
                );
                painter.rect_filled(
                    bar_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(14, 16, 22, 230),
                );

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(16.0);
                        let play_sym = if self.is_playing { "⏸" } else { "▶" };
                        let btn_play = egui::Button::new(
                            RichText::new(play_sym)
                                .size(15.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(0xe0, 0x7a, 0x38))
                        .min_size(Vec2::new(34.0, 26.0));
                        if ui.add(btn_play).clicked() {
                            self.toggle_play();
                        }

                        let btn_prev = egui::Button::new(RichText::new("⏴").size(12.0))
                            .fill(Color32::from_rgb(0x24, 0x24, 0x30))
                            .min_size(Vec2::new(26.0, 24.0));
                        if ui.add(btn_prev).on_hover_text("1 Kare Geri").clicked() {
                            let fps = self.project.timeline.fps.max(1.0);
                            self.seek((self.current_time - 1.0 / fps).max(0.0));
                        }

                        let btn_next = egui::Button::new(RichText::new("⏵").size(12.0))
                            .fill(Color32::from_rgb(0x24, 0x24, 0x30))
                            .min_size(Vec2::new(26.0, 24.0));
                        if ui.add(btn_next).on_hover_text("1 Kare İleri").clicked() {
                            let fps = self.project.timeline.fps.max(1.0);
                            let dur = self.project.timeline.duration();
                            self.seek((self.current_time + 1.0 / fps).min(dur));
                        }

                        ui.add_space(8.0);
                        let total_dur = self.project.timeline.duration().max(0.1);
                        let cur_tc = trim_core::time::seconds_to_timecode(
                            self.current_time,
                            self.project.timeline.fps,
                        );
                        let tot_tc = trim_core::time::seconds_to_timecode(
                            total_dur,
                            self.project.timeline.fps,
                        );
                        ui.label(
                            RichText::new(format!("{} / {}", cur_tc, tot_tc))
                                .monospace()
                                .size(12.0)
                                .color(Color32::from_rgb(0xe0, 0x85, 0x38)),
                        );

                        ui.add_space(12.0);
                        let mut seek_t = self.current_time;
                        let slider = egui::Slider::new(&mut seek_t, 0.0..=total_dur)
                            .show_value(false)
                            .trailing_fill(true);
                        let slider_resp = ui.add_sized([ui.available_width() - 80.0, 18.0], slider);
                        if slider_resp.changed() {
                            self.seek(seek_t);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            let btn_exit = egui::Button::new(RichText::new("⛶").size(14.0))
                                .fill(Color32::from_rgb(0x28, 0x28, 0x34))
                                .min_size(Vec2::new(30.0, 24.0));
                            if ui
                                .add(btn_exit)
                                .on_hover_text("Tam Ekrandan Çık (F11)")
                                .clicked()
                            {
                                self.is_monitor_fullscreen = false;
                            }
                        });
                    });
                });
            });
    }
}
