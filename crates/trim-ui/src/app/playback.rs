//! Playback controls, frame stepping, shuttle speeds, and clock synchronization.

use crate::app::TrimApp;
use egui::{ColorImage, TextureOptions};
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
}
