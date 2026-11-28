use std::time::Instant;
use trim_core::clip::{ChromaKeyConfig, Clip, ColorWheelsConfig, LutPreset};
use trim_core::color::{CustomCurvesConfig, HslCurvesConfig};
use trim_render::{bake_adjustment_curves_lut, LayerUniforms};

fn main() {
    let mut clip = Clip::new("bench_media", 0.0, 10.0);
    clip.scale = 1.25;
    clip.pos_x = 12.0;
    clip.pos_y = -8.0;
    clip.rotation = 15.0;
    clip.lut_preset = LutPreset::Kodak2383;
    clip.chromakey = Some(ChromaKeyConfig {
        enabled: true,
        key_color: [0, 255, 0],
        similarity: 0.25,
        smoothness: 0.08,
    });
    clip.color_wheels = Some(ColorWheelsConfig {
        lift: [0.05, 0.0, -0.05],
        gamma: [1.05, 1.0, 0.95],
        gain: [1.1, 1.0, 0.9],
        luminance: [0.02, 0.0, -0.02],
    });
    clip.custom_curves = Some(CustomCurvesConfig {
        master: vec![[0.0, 0.05], [0.5, 0.55], [1.0, 0.95]],
        ..Default::default()
    });
    clip.hsl_curves = Some(HslCurvesConfig::default());

    let t0 = Instant::now();
    let frames = 1000usize;
    let layers_per_frame = 4usize;
    let mut acc = 0.0f32;
    for f in 0..frames {
        let dt = (f as f64) * 0.01;
        for _ in 0..layers_per_frame {
            if let Some(u) = LayerUniforms::from_clip(&clip, 3840, 2160, 1920, 1080, dt) {
                acc += u.opacity;
            }
        }
    }
    let uniforms_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let uniforms_per_frame_us = (uniforms_ms * 1000.0) / (frames as f64);

    let t1 = Instant::now();
    let lut_iters = 50usize;
    let mut lut_bytes = 0usize;
    for _ in 0..lut_iters {
        let lut = bake_adjustment_curves_lut(&clip, None, 17);
        lut_bytes += lut.rgba_strip.len();
    }
    let lut_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let lut_per_bake_ms = lut_ms / (lut_iters as f64);

    println!("=== triM. trim-render GPU Pipeline Benchmark ===");
    println!(
        "LayerUniforms (4 katman/kare, 1000 kare): toplam {:.2} ms | {:.2} µs/kare (acc={:.1})",
        uniforms_ms, uniforms_per_frame_us, acc
    );
    println!(
        "Adjustment 3D LUT Bake (17x17x17, 50 tekrar): toplam {:.2} ms | {:.3} ms/bake ({} bytes)",
        lut_ms,
        lut_per_bake_ms,
        lut_bytes / lut_iters
    );
}
