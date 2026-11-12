//! Vector/parametric mask models, keyframe evaluation, SDF/Bezier geometry,
//! and rasterization shared across CPU preview, GPU mask textures, and export.

use crate::keyframe::{interpolate_keyframes, Interpolation, Keyframe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskShape {
    #[default]
    Rectangle,
    Ellipse,
    Bezier,
}

impl MaskShape {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rectangle => "Dikdörtgen",
            Self::Ellipse => "Elips",
            Self::Bezier => "Serbest (Bezier)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskTarget {
    #[default]
    ClipAlpha,
    ColorAndEffects,
}

impl MaskTarget {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClipAlpha => "Klip Saydamlığı (Kesme)",
            Self::ColorAndEffects => "Renk & Efekt Alanı",
        }
    }
}

/// A Bezier control point in normalized `[0.0, 1.0]` frame coordinates.
/// `in_tangent` and `out_tangent` are relative offsets from `pos`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskVertex {
    pub pos: [f32; 2],
    #[serde(default)]
    pub in_tangent: [f32; 2],
    #[serde(default)]
    pub out_tangent: [f32; 2],
}

impl MaskVertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            pos: [x, y],
            in_tangent: [0.0, 0.0],
            out_tangent: [0.0, 0.0],
        }
    }

    pub fn with_tangents(pos: [f32; 2], in_t: [f32; 2], out_t: [f32; 2]) -> Self {
        Self {
            pos,
            in_tangent: in_t,
            out_tangent: out_t,
        }
    }
}

/// Evaluates a mask's animated parameters at a clip-relative time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvaluatedMaskParams {
    pub pos_x: f32,
    pub pos_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation_deg: f32,
    pub feather: f32,
    pub opacity: f32,
    pub expansion: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    pub id: String,
    #[serde(default = "default_mask_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub shape: MaskShape,
    #[serde(default)]
    pub target: MaskTarget,
    #[serde(default = "default_half")]
    pub pos_x: f64,
    #[serde(default = "default_half")]
    pub pos_y: f64,
    #[serde(default = "default_mask_scale")]
    pub scale_x: f64,
    #[serde(default = "default_mask_scale")]
    pub scale_y: f64,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "default_feather")]
    pub feather: f64,
    #[serde(default = "default_one")]
    pub opacity: f64,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub expansion: f64,
    #[serde(default)]
    pub vertices: Vec<MaskVertex>,
    #[serde(default)]
    pub keyframes: HashMap<String, Vec<Keyframe>>,
}

fn default_mask_name() -> String {
    "Maske".to_string()
}
fn default_true() -> bool {
    true
}
fn default_half() -> f64 {
    0.5
}
fn default_mask_scale() -> f64 {
    0.45
}
fn default_feather() -> f64 {
    0.02
}
fn default_one() -> f64 {
    1.0
}

impl Default for Mask {
    fn default() -> Self {
        Self::new(MaskShape::Rectangle, "Maske 1")
    }
}

impl Mask {
    pub fn new(shape: MaskShape, name: impl Into<String>) -> Self {
        let (scale_x, scale_y, vertices) = match shape {
            MaskShape::Rectangle | MaskShape::Ellipse => (0.45, 0.45, Vec::new()),
            MaskShape::Bezier => (
                1.0,
                1.0,
                vec![
                    MaskVertex::with_tangents([0.30, 0.30], [-0.05, 0.03], [0.05, -0.03]),
                    MaskVertex::with_tangents([0.70, 0.30], [-0.05, -0.03], [0.05, 0.03]),
                    MaskVertex::with_tangents([0.70, 0.70], [0.05, -0.03], [-0.05, 0.03]),
                    MaskVertex::with_tangents([0.30, 0.70], [0.05, 0.03], [-0.05, -0.03]),
                ],
            ),
        };
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            enabled: true,
            shape,
            target: MaskTarget::ClipAlpha,
            pos_x: 0.5,
            pos_y: 0.5,
            scale_x,
            scale_y,
            rotation: 0.0,
            feather: 0.02,
            opacity: 1.0,
            invert: false,
            expansion: 0.0,
            vertices,
            keyframes: HashMap::new(),
        }
    }

    pub fn get_param_at(&self, prop: &str, clip_time: f64) -> f64 {
        let base = match prop {
            "pos_x" => self.pos_x,
            "pos_y" => self.pos_y,
            "scale_x" => self.scale_x,
            "scale_y" => self.scale_y,
            "rotation" => self.rotation,
            "feather" => self.feather,
            "opacity" => self.opacity,
            "expansion" => self.expansion,
            _ => return 0.0,
        };
        if let Some(kfs) = self.keyframes.get(prop) {
            if !kfs.is_empty() {
                return interpolate_keyframes(kfs, clip_time, base);
            }
        }
        base
    }

    pub fn eval_at(&self, clip_time: f64) -> EvaluatedMaskParams {
        EvaluatedMaskParams {
            pos_x: self.get_param_at("pos_x", clip_time) as f32,
            pos_y: self.get_param_at("pos_y", clip_time) as f32,
            scale_x: (self.get_param_at("scale_x", clip_time) as f32).max(0.01),
            scale_y: (self.get_param_at("scale_y", clip_time) as f32).max(0.01),
            rotation_deg: self.get_param_at("rotation", clip_time) as f32,
            feather: (self.get_param_at("feather", clip_time) as f32).max(0.0),
            opacity: (self.get_param_at("opacity", clip_time) as f32).clamp(0.0, 1.0),
            expansion: (self.get_param_at("expansion", clip_time) as f32).clamp(-0.5, 0.5),
        }
    }

    pub fn set_keyframe(&mut self, prop: &str, time: f64, value: f64, interp: Interpolation) {
        let entry = self.keyframes.entry(prop.to_string()).or_default();
        if let Some(existing) = entry.iter_mut().find(|k| (k.time - time).abs() < 0.01) {
            existing.value = value;
            existing.interpolation = interp;
        } else {
            entry.push(Keyframe::new(time, value, interp));
            entry.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    pub fn remove_keyframe(&mut self, prop: &str, time: f64) {
        if let Some(entry) = self.keyframes.get_mut(prop) {
            entry.retain(|k| (k.time - time).abs() >= 0.01);
            if entry.is_empty() {
                self.keyframes.remove(prop);
            }
        }
    }

    pub fn has_keyframe_at(&self, prop: &str, time: f64) -> bool {
        self.keyframes
            .get(prop)
            .is_some_and(|kfs| kfs.iter().any(|k| (k.time - time).abs() < 0.01))
    }
}

/// Samples a closed cubic Bezier mask path into a polyline of `[x, y]` points.
pub fn flatten_bezier_polygon(vertices: &[MaskVertex], steps_per_segment: usize) -> Vec<[f32; 2]> {
    let n = vertices.len();
    if n < 3 {
        return vertices.iter().map(|v| v.pos).collect();
    }
    let steps = steps_per_segment.max(2);
    let mut pts = Vec::with_capacity(n * steps);
    for i in 0..n {
        let v0 = &vertices[i];
        let v1 = &vertices[(i + 1) % n];
        let p0 = v0.pos;
        let p1 = [p0[0] + v0.out_tangent[0], p0[1] + v0.out_tangent[1]];
        let p3 = v1.pos;
        let p2 = [p3[0] + v1.in_tangent[0], p3[1] + v1.in_tangent[1]];

        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let u = 1.0 - t;
            let uu = u * u;
            let tt = t * t;
            let x = uu * u * p0[0] + 3.0 * uu * t * p1[0] + 3.0 * u * tt * p2[0] + tt * t * p3[0];
            let y = uu * u * p0[1] + 3.0 * uu * t * p1[1] + 3.0 * u * tt * p2[1] + tt * t * p3[1];
            pts.push([x, y]);
        }
    }
    pts
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < 1e-6 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Signed distance to a closed 2D polygon (`< 0` inside, `> 0` outside).
fn polygon_signed_distance(pt: [f32; 2], poly: &[[f32; 2]]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 1.0;
    }
    let mut min_dist_sq = f32::MAX;
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let a = poly[j];
        let b = poly[i];
        // Ray casting parity
        if ((a[1] > pt[1]) != (b[1] > pt[1]))
            && (pt[0] < (b[0] - a[0]) * (pt[1] - a[1]) / (b[1] - a[1] + 1e-12) + a[0])
        {
            inside = !inside;
        }
        // Closest point on segment AB
        let abx = b[0] - a[0];
        let aby = b[1] - a[1];
        let apx = pt[0] - a[0];
        let apy = pt[1] - a[1];
        let ab_len_sq = abx * abx + aby * aby;
        let t = if ab_len_sq > 1e-12 {
            ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let dx = apx - abx * t;
        let dy = apy - aby * t;
        let d_sq = dx * dx + dy * dy;
        if d_sq < min_dist_sq {
            min_dist_sq = d_sq;
        }
        j = i;
    }
    let d = min_dist_sq.sqrt();
    if inside {
        -d
    } else {
        d
    }
}

/// Evaluates a single mask at normalized UV `(u, v) in [0, 1]`, returning coverage in `[0.0, 1.0]`.
pub fn evaluate_single_mask_at_uv(
    mask: &Mask,
    uv: [f32; 2],
    params: &EvaluatedMaskParams,
    aspect: f32,
    bezier_poly: Option<&[[f32; 2]]>,
) -> f32 {
    let rad = -params.rotation_deg.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();

    // Aspect-corrected centered coordinates around (pos_x, pos_y)
    let dx = (uv[0] - params.pos_x) * aspect;
    let dy = uv[1] - params.pos_y;
    let rx = dx * cos_r - dy * sin_r;
    let ry = dx * sin_r + dy * cos_r;

    let sd = match mask.shape {
        MaskShape::Rectangle => {
            let hx = (params.scale_x * 0.5) * aspect;
            let hy = params.scale_y * 0.5;
            let qx = rx.abs() - hx;
            let qy = ry.abs() - hy;
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let inside = qx.max(qy).min(0.0);
            outside + inside
        }
        MaskShape::Ellipse => {
            let hx = ((params.scale_x * 0.5) * aspect).max(1e-4);
            let hy = (params.scale_y * 0.5).max(1e-4);
            let nx = rx / hx;
            let ny = ry / hy;
            let len = (nx * nx + ny * ny).sqrt();
            // Approximate Euclidean signed distance scaled by min radius
            (len - 1.0) * hx.min(hy)
        }
        MaskShape::Bezier => {
            // Transform point back to normalized [0,1] polygon space with scale/pos/rot
            let local_x = (rx / aspect) / params.scale_x.max(1e-4) + 0.5;
            let local_y = ry / params.scale_y.max(1e-4) + 0.5;
            if let Some(poly) = bezier_poly {
                polygon_signed_distance([local_x, local_y], poly)
            } else {
                1.0
            }
        }
    };

    let expanded_sd = sd - params.expansion;
    let feather = params.feather.max(1e-4);
    let mut cov = if params.feather <= 1e-4 {
        if expanded_sd <= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - smoothstep(-feather * 0.5, feather * 0.5, expanded_sd)
    };

    if mask.invert {
        cov = 1.0 - cov;
    }

    (cov * params.opacity).clamp(0.0, 1.0)
}

/// Evaluates all masks at UV `(u, v)` returning `(clip_alpha_mask, color_effect_mask)`.
pub fn evaluate_masks_at_uv(
    masks: &[Mask],
    uv: [f32; 2],
    clip_time: f64,
    aspect: f32,
) -> (f32, f32) {
    if masks.is_empty() {
        return (1.0, 1.0);
    }

    let mut has_alpha = false;
    let mut alpha_accum = 0.0_f32;
    let mut has_effect = false;
    let mut effect_accum = 0.0_f32;

    for mask in masks {
        if !mask.enabled {
            continue;
        }
        let params = mask.eval_at(clip_time);
        let poly = if mask.shape == MaskShape::Bezier {
            Some(flatten_bezier_polygon(&mask.vertices, 10))
        } else {
            None
        };
        let cov = evaluate_single_mask_at_uv(mask, uv, &params, aspect, poly.as_deref());
        match mask.target {
            MaskTarget::ClipAlpha => {
                has_alpha = true;
                alpha_accum = alpha_accum.max(cov);
            }
            MaskTarget::ColorAndEffects => {
                has_effect = true;
                effect_accum = effect_accum.max(cov);
            }
        }
    }

    (
        if has_alpha { alpha_accum } else { 1.0 },
        if has_effect { effect_accum } else { 1.0 },
    )
}

/// Rasterizes all enabled masks into an RGBA8 buffer (`R = ClipAlpha`, `G = ColorAndEffects`, `B = 0`, `A = 255`).
/// Precomputes Bezier polygons once per frame for fast evaluation.
pub fn rasterize_masks_rgba(masks: &[Mask], width: u32, height: u32, clip_time: f64) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut buf = vec![255u8; w * h * 4];
    if w == 0 || h == 0 || masks.is_empty() {
        return buf;
    }

    struct PreparedMask {
        target: MaskTarget,
        invert: bool,
        shape: MaskShape,
        params: EvaluatedMaskParams,
        poly: Option<Vec<[f32; 2]>>,
    }

    let prepared: Vec<PreparedMask> = masks
        .iter()
        .filter(|m| m.enabled)
        .map(|m| PreparedMask {
            target: m.target,
            invert: m.invert,
            shape: m.shape,
            params: m.eval_at(clip_time),
            poly: if m.shape == MaskShape::Bezier {
                Some(flatten_bezier_polygon(&m.vertices, 12))
            } else {
                None
            },
        })
        .collect();

    if prepared.is_empty() {
        return buf;
    }

    let has_alpha = prepared.iter().any(|p| p.target == MaskTarget::ClipAlpha);
    let has_effect = prepared
        .iter()
        .any(|p| p.target == MaskTarget::ColorAndEffects);
    let aspect = (width as f32) / (height.max(1) as f32);

    for y in 0..h {
        let v = (y as f32 + 0.5) / (h as f32);
        for x in 0..w {
            let u = (x as f32 + 0.5) / (w as f32);
            let mut alpha_accum = 0.0_f32;
            let mut effect_accum = 0.0_f32;

            for pm in &prepared {
                // Construct lightweight view to call shared SDF evaluator
                let rad = -pm.params.rotation_deg.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let dx = (u - pm.params.pos_x) * aspect;
                let dy = v - pm.params.pos_y;
                let rx = dx * cos_r - dy * sin_r;
                let ry = dx * sin_r + dy * cos_r;

                let sd = match pm.shape {
                    MaskShape::Rectangle => {
                        let hx = (pm.params.scale_x * 0.5) * aspect;
                        let hy = pm.params.scale_y * 0.5;
                        let qx = rx.abs() - hx;
                        let qy = ry.abs() - hy;
                        (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0)
                    }
                    MaskShape::Ellipse => {
                        let hx = ((pm.params.scale_x * 0.5) * aspect).max(1e-4);
                        let hy = (pm.params.scale_y * 0.5).max(1e-4);
                        let nx = rx / hx;
                        let ny = ry / hy;
                        ((nx * nx + ny * ny).sqrt() - 1.0) * hx.min(hy)
                    }
                    MaskShape::Bezier => {
                        let lx = (rx / aspect) / pm.params.scale_x.max(1e-4) + 0.5;
                        let ly = ry / pm.params.scale_y.max(1e-4) + 0.5;
                        if let Some(poly) = &pm.poly {
                            polygon_signed_distance([lx, ly], poly)
                        } else {
                            1.0
                        }
                    }
                };

                let exp_sd = sd - pm.params.expansion;
                let feather = pm.params.feather.max(1e-4);
                let mut cov = if pm.params.feather <= 1e-4 {
                    if exp_sd <= 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    1.0 - smoothstep(-feather * 0.5, feather * 0.5, exp_sd)
                };
                if pm.invert {
                    cov = 1.0 - cov;
                }
                cov = (cov * pm.params.opacity).clamp(0.0, 1.0);

                match pm.target {
                    MaskTarget::ClipAlpha => alpha_accum = alpha_accum.max(cov),
                    MaskTarget::ColorAndEffects => effect_accum = effect_accum.max(cov),
                }
            }

            let out_a = if has_alpha { alpha_accum } else { 1.0 };
            let out_e = if has_effect { effect_accum } else { 1.0 };
            let idx = (y * w + x) * 4;
            buf[idx] = (out_a * 255.0).round() as u8;
            buf[idx + 1] = (out_e * 255.0).round() as u8;
            buf[idx + 2] = 0;
            buf[idx + 3] = 255;
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_mask_geometry_invert_and_feather() {
        let mut mask = Mask::new(MaskShape::Rectangle, "Rect");
        mask.pos_x = 0.5;
        mask.pos_y = 0.5;
        mask.scale_x = 0.4;
        mask.scale_y = 0.4;
        mask.feather = 0.0;

        let p = mask.eval_at(0.0);
        // Center must be 1.0, far corner must be 0.0
        assert!((evaluate_single_mask_at_uv(&mask, [0.5, 0.5], &p, 1.0, None) - 1.0).abs() < 1e-5);
        assert!(
            (evaluate_single_mask_at_uv(&mask, [0.05, 0.05], &p, 1.0, None) - 0.0).abs() < 1e-5
        );

        // Invert flips inside/outside
        mask.invert = true;
        assert!((evaluate_single_mask_at_uv(&mask, [0.5, 0.5], &p, 1.0, None) - 0.0).abs() < 1e-5);
        assert!(
            (evaluate_single_mask_at_uv(&mask, [0.05, 0.05], &p, 1.0, None) - 1.0).abs() < 1e-5
        );

        // Feather smoothly blends boundary
        mask.invert = false;
        mask.feather = 0.1;
        let pf = mask.eval_at(0.0);
        let edge_val = evaluate_single_mask_at_uv(&mask, [0.7, 0.5], &pf, 1.0, None);
        assert!(edge_val > 0.2 && edge_val < 0.8, "edge_val={edge_val}");
    }

    #[test]
    fn test_ellipse_and_bezier_and_keyframe_interpolation() {
        let mut ell = Mask::new(MaskShape::Ellipse, "Ell");
        ell.scale_x = 0.4;
        ell.scale_y = 0.4;
        ell.feather = 0.0;
        ell.set_keyframe("pos_x", 0.0, 0.2, Interpolation::Linear);
        ell.set_keyframe("pos_x", 2.0, 0.8, Interpolation::Linear);

        let p_mid = ell.eval_at(1.0);
        assert!((p_mid.pos_x - 0.5).abs() < 1e-4);
        assert!(
            (evaluate_single_mask_at_uv(&ell, [0.5, 0.5], &p_mid, 1.0, None) - 1.0).abs() < 1e-4
        );

        let bez = Mask::new(MaskShape::Bezier, "Bez");
        let poly = flatten_bezier_polygon(&bez.vertices, 8);
        assert!(poly.len() >= 24);
        let pb = bez.eval_at(0.0);
        assert!(
            (evaluate_single_mask_at_uv(&bez, [0.5, 0.5], &pb, 1.0, Some(&poly)) - 1.0).abs()
                < 1e-4
        );
        assert!(
            (evaluate_single_mask_at_uv(&bez, [0.02, 0.02], &pb, 1.0, Some(&poly)) - 0.0).abs()
                < 1e-4
        );
    }

    #[test]
    fn test_multi_mask_targets_rasterization() {
        let m_alpha = Mask::new(MaskShape::Rectangle, "Alpha");
        let mut m_fx = Mask::new(MaskShape::Ellipse, "FX");
        m_fx.target = MaskTarget::ColorAndEffects;
        let buf = rasterize_masks_rgba(&[m_alpha, m_fx], 32, 32, 0.0);
        // Center pixel (16, 16) should have both R and G = 255
        let center_idx = (16 * 32 + 16) * 4;
        assert_eq!(buf[center_idx], 255);
        assert_eq!(buf[center_idx + 1], 255);
        // Corner pixel (0, 0) should have R=0, G=0
        assert_eq!(buf[0], 0);
        assert_eq!(buf[1], 0);
    }
}
