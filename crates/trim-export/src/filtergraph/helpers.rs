use trim_core::clip::Clip;
use trim_core::keyframe::{Interpolation, Keyframe};

pub(super) fn safe_ffmpeg_path_arg(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('-') && std::path::Path::new(trimmed).is_relative() {
        format!("./{trimmed}")
    } else {
        trimmed.to_string()
    }
}

pub(super) fn escape_filtergraph_file_option(raw: &str) -> String {
    let forward = raw.replace('\\', "/");
    let mut out = String::with_capacity(forward.len() + 8);
    for ch in forward.chars() {
        match ch {
            '\0' | '\r' | '\n' => {}
            ':' => out.push_str("\\:"),
            '\'' => out.push_str("'\\''"),
            _ => out.push(ch),
        }
    }
    out
}

pub(super) fn sanitize_drawtext_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '\0' | '\r' => {}
            '\n' => out.push(' '),
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("'\\''"),
            ':' => out.push_str("\\:"),
            '%' => out.push_str("\\%"),
            _ => out.push(ch),
        }
    }
    out
}

/// Accumulates FFmpeg input arguments and hands out input indices.
pub(super) struct InputList {
    pub(super) args: Vec<String>,
    count: usize,
}

impl InputList {
    pub(super) fn new() -> Self {
        Self {
            args: Vec::new(),
            count: 0,
        }
    }

    pub(super) fn push(&mut self, args: impl IntoIterator<Item = String>) -> usize {
        self.args.extend(args);
        let idx = self.count;
        self.count += 1;
        idx
    }
}

pub(super) fn even(v: u32) -> u32 {
    (v.max(2)) & !1
}

/// Builds an FFmpeg expression that evaluates keyframes at local time `var`.
/// Mirrors `trim_core::keyframe::interpolate_keyframes`.
pub(super) fn keyframe_expr(keyframes: &[Keyframe], var: &str, map: impl Fn(f64) -> f64) -> String {
    let mut kfs: Vec<Keyframe> = keyframes
        .iter()
        .filter(|k| k.time.is_finite() && k.value.is_finite())
        .cloned()
        .collect();
    kfs.sort_by(|a, b| a.time.total_cmp(&b.time));
    match kfs.len() {
        0 => return format!("{:.6}", map(0.0)),
        1 => return format!("{:.6}", map(kfs[0].value)),
        _ => {}
    }

    // Build nested if() from the last segment backwards.
    let Some(last) = kfs.last() else {
        return format!("{:.6}", map(0.0));
    };
    let mut expr = format!("{:.6}", map(last.value));
    for i in (0..kfs.len() - 1).rev() {
        let k0 = &kfs[i];
        let k1 = &kfs[i + 1];
        let v0 = map(k0.value);
        let v1 = map(k1.value);
        let span = k1.time - k0.time;
        let seg = if span.abs() < 1e-6 {
            format!("{:.6}", v0)
        } else {
            let u = format!("(({var}-{:.6})/{:.6})", k0.time, span);
            match k0.interpolation {
                Interpolation::Hold => format!("{:.6}", v0),
                Interpolation::Linear => format!("({:.6}+({:.6})*{u})", v0, v1 - v0),
                Interpolation::EaseIn => format!("({:.6}+({:.6})*({u}*{u}))", v0, v1 - v0),
                Interpolation::EaseOut => {
                    format!("({:.6}+({:.6})*(1-(1-{u})*(1-{u})))", v0, v1 - v0)
                }
                Interpolation::EaseInOut => {
                    format!("({:.6}+({:.6})*(3*{u}*{u}-2*{u}*{u}*{u}))", v0, v1 - v0)
                }
                Interpolation::CubicBezier => {
                    let y1 = k0.bezier_handles[1] as f64;
                    let y2 = k0.bezier_handles[3] as f64;
                    format!(
                        "({:.6}+({:.6})*(3*(1-{u})*(1-{u})*{u}*{:.4}+3*(1-{u})*{u}*{u}*{:.4}+{u}*{u}*{u}))",
                        v0,
                        v1 - v0,
                        y1,
                        y2
                    )
                }
            }
        };
        expr = format!("if(lt({var},{:.6}),{seg},{expr})", k1.time);
    }
    let first = &kfs[0];
    format!(
        "if(lt({var},{:.6}),{:.6},{expr})",
        first.time,
        map(first.value)
    )
}

pub(super) fn animated_kfs<'a>(clip: &'a Clip, prop: &str) -> Option<&'a [Keyframe]> {
    clip.keyframes
        .get(prop)
        .filter(|k| !k.is_empty())
        .map(|v| v.as_slice())
}

/// Source window `[start, start + src_dur)` that a clip reads from.
pub(super) fn source_window(clip: &Clip) -> (f64, f64) {
    let dur = clip.duration();
    let src_dur = if let Some(kfs) = clip.keyframes.get("speed").filter(|k| !k.is_empty()) {
        trim_core::keyframe::integrate_keyframes(kfs, dur, clip.speed).max(0.04)
    } else {
        dur * clip.speed.max(0.0001)
    };
    let start = if clip.reverse {
        (clip.effective_source_out() - src_dur).max(0.0)
    } else {
        clip.source_in.max(0.0)
    };
    (start, src_dur)
}

/// Builds a piecewise-linear `setpts` expression mapping source time `T` to timeline local time
/// when a clip uses Speed Ramping (`speed` keyframes).
pub(super) fn build_speed_ramp_setpts(clip: &Clip) -> Option<String> {
    let kfs = clip.keyframes.get("speed").filter(|k| !k.is_empty())?;
    let dur = clip.duration();
    if dur <= 0.04 {
        return None;
    }
    let segments = 8usize;
    let mut s_pts = Vec::with_capacity(segments + 1);
    let mut t_pts = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = (i as f64 / segments as f64) * dur;
        let s = trim_core::keyframe::integrate_keyframes(kfs, t, clip.speed);
        t_pts.push(t);
        s_pts.push(s);
    }
    let mut expr = format!("{:.6}", dur);
    for i in (0..segments).rev() {
        let s0 = s_pts[i];
        let s1 = s_pts[i + 1];
        let t0 = t_pts[i];
        let t1 = t_pts[i + 1];
        let ds = (s1 - s0).max(1e-5);
        let seg = format!("({:.6}+(T-{:.6})*{:.6})", t0, s0, (t1 - t0) / ds);
        expr = format!("if(lt(T,{:.6}),{seg},{expr})", s1);
    }
    Some(format!("setpts='({expr})/TB'"))
}
