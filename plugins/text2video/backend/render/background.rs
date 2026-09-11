//! 背景生成与装饰（移植自 Python `z2v/renderer/background.py`）。

use image::{Rgba, RgbaImage};

pub const DARK_PALETTES: [[&str; 2]; 8] = [
    ["#141a2e", "#1f2f4d"],
    ["#101c1a", "#1c3a33"],
    ["#231a2e", "#3a2450"],
    ["#2e1a1a", "#4d241f"],
    ["#0f1923", "#1a2d3d"],
    ["#1a1a2e", "#2d2d4e"],
    ["#1c1c2e", "#2a2a4a"],
    ["#0d1b2a", "#1b2838"],
];

/// 浅色模式：接近纯白，仅保留极淡的色调差异
pub const LIGHT_PALETTES: [[&str; 2]; 8] = [
    ["#ffffff", "#f7f8fb"],
    ["#ffffff", "#fbf8f4"],
    ["#ffffff", "#f5f9fc"],
    ["#ffffff", "#faf6fc"],
    ["#ffffff", "#f8f8f5"],
    ["#ffffff", "#f4faf7"],
    ["#ffffff", "#f8fbf4"],
    ["#ffffff", "#fdf5f6"],
];

pub fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(h.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(h.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(h.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    (r, g, b)
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// 竖向渐变背景
pub fn make_gradient(palette: &[&str; 2], w: u32, h: u32) -> RgbaImage {
    let (r0, g0, b0) = parse_hex(palette[0]);
    let (r1, g1, b1) = parse_hex(palette[1]);
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        let t = if h <= 1 {
            0.0
        } else {
            y as f32 / (h - 1) as f32
        };
        let color = Rgba([lerp(r0, r1, t), lerp(g0, g1, t), lerp(b0, b1, t), 255]);
        for x in 0..w {
            img.put_pixel(x, y, color);
        }
    }
    img
}

/// 选择背景（调色板 + 装饰）
pub fn get_background(index: usize, accent: &str, w: u32, h: u32, theme: &str) -> RgbaImage {
    let palettes: &[[&str; 2]; 8] = if theme == "dark" {
        &DARK_PALETTES
    } else {
        &LIGHT_PALETTES
    };
    let palette = &palettes[index % palettes.len()];
    let grad = make_gradient(palette, w, h);
    decorate(&grad, accent, theme)
}

/// 叠加光斑 / 斜线 / 蒙层（theme 控制配色）
pub fn decorate(img: &RgbaImage, accent: &str, theme: &str) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (wf, hf) = (w as f32, h as f32);
    let (ar, ag, ab) = parse_hex(accent);
    let mut overlay = RgbaImage::new(w, h);

    if theme == "dark" {
        fill_ellipse(
            &mut overlay,
            [wf * 0.45, -hf * 0.12, wf * 1.35, hf * 0.22],
            [ar, ag, ab, 46],
        );
        fill_ellipse(
            &mut overlay,
            [-wf * 0.35, hf * 0.72, wf * 0.55, hf * 1.15],
            [ar, ag, ab, 30],
        );
        for i in 0..3 {
            let x0 = wf * (0.08 + i as f32 * 0.05);
            draw_line(
                &mut overlay,
                (x0, hf),
                (x0 + wf * 0.5, 0.0),
                2.0,
                [255, 255, 255, 14],
            );
        }
        let blurred = image::imageops::fast_blur(&overlay, 60.0);
        let out = alpha_over(img, &blurred);
        let scrim = solid(w, h, [0, 0, 0, 70]);
        alpha_over(&out, &scrim)
    } else {
        fill_ellipse(
            &mut overlay,
            [wf * 0.5, -hf * 0.1, wf * 1.4, hf * 0.18],
            [ar, ag, ab, 35],
        );
        fill_ellipse(
            &mut overlay,
            [-wf * 0.3, hf * 0.75, wf * 0.5, hf * 1.12],
            [ar, ag, ab, 25],
        );
        for i in 0..3 {
            let x0 = wf * (0.08 + i as f32 * 0.05);
            draw_line(
                &mut overlay,
                (x0, hf),
                (x0 + wf * 0.5, 0.0),
                2.0,
                [180, 180, 190, 20],
            );
        }
        let blurred = image::imageops::fast_blur(&overlay, 60.0);
        let out = alpha_over(img, &blurred);
        let scrim = solid(w, h, [255, 255, 255, 30]);
        alpha_over(&out, &scrim)
    }
}

fn solid(w: u32, h: u32, color: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba(color))
}

/// 填充矩形（直接覆写像素）
pub fn fill_rect(img: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: [u8; 4]) {
    let (w, h) = img.dimensions();
    let bx0 = x0.floor().max(0.0) as u32;
    let bx1 = (x1.ceil().max(0.0) as u32).min(w);
    let by0 = y0.floor().max(0.0) as u32;
    let by1 = (y1.ceil().max(0.0) as u32).min(h);
    for y in by0..by1 {
        for x in bx0..bx1 {
            img.put_pixel(x, y, Rgba(color));
        }
    }
}

/// 填充圆角矩形（直接覆写像素）
pub fn fill_rounded_rect(
    img: &mut RgbaImage,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
    color: [u8; 4],
) {
    let (w, h) = img.dimensions();
    let r = radius.max(0.0);
    let bx0 = x0.floor().max(0.0) as u32;
    let bx1 = (x1.ceil().max(0.0) as u32).min(w);
    let by0 = y0.floor().max(0.0) as u32;
    let by1 = (y1.ceil().max(0.0) as u32).min(h);
    for y in by0..by1 {
        for x in bx0..bx1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = (x0 + r - px).max(px - (x1 - r)).max(0.0);
            let dy = (y0 + r - py).max(py - (y1 - r)).max(0.0);
            if dx * dx + dy * dy <= r * r {
                img.put_pixel(x, y, Rgba(color));
            }
        }
    }
}

fn alpha_over(bottom: &RgbaImage, top: &RgbaImage) -> RgbaImage {
    let mut out = bottom.clone();
    image::imageops::overlay(&mut out, top, 0, 0);
    out
}

/// 填充椭圆（bbox = [x0, y0, x1, y1]，直接覆写像素，与 PIL ImageDraw 一致）
fn fill_ellipse(img: &mut RgbaImage, bbox: [f32; 4], color: [u8; 4]) {
    let (w, h) = img.dimensions();
    let cx = (bbox[0] + bbox[2]) / 2.0;
    let cy = (bbox[1] + bbox[3]) / 2.0;
    let rx = ((bbox[2] - bbox[0]) / 2.0).abs();
    let ry = ((bbox[3] - bbox[1]) / 2.0).abs();
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let x0 = bbox[0].floor().max(0.0) as u32;
    let x1 = (bbox[2].ceil().max(0.0) as u32).min(w);
    let y0 = bbox[1].floor().max(0.0) as u32;
    let y1 = (bbox[3].ceil().max(0.0) as u32).min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                img.put_pixel(x, y, Rgba(color));
            }
        }
    }
}

/// 画线（按点到线段距离填充，支持线宽）
fn draw_line(img: &mut RgbaImage, p0: (f32, f32), p1: (f32, f32), width: f32, color: [u8; 4]) {
    let (w, h) = img.dimensions();
    let half = width / 2.0;
    let x0 = (p0.0.min(p1.0) - half).floor().max(0.0) as u32;
    let x1 = (p0.0.max(p1.0) + half).ceil().max(0.0) as u32;
    let y0 = (p0.1.min(p1.1) - half).floor().max(0.0) as u32;
    let y1 = (p0.1.max(p1.1) + half).ceil().max(0.0) as u32;
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            if dist_to_segment(x as f32 + 0.5, y as f32 + 0.5, p0, p1) <= half {
                img.put_pixel(x, y, Rgba(color));
            }
        }
    }
}

fn dist_to_segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (ax, ay) = a;
    let (bx, by) = b;
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f32::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = ax + t * dx;
    let proj_y = ay + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}
