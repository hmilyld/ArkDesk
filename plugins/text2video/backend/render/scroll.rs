//! 滚动模板渲染与封面（移植自 Python `z2v/renderer/scroll.py` + `cover.py`）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use image::codecs::jpeg::JpegEncoder;
use image::{Rgba, RgbaImage};

use crate::error::{code, AppError};
use crate::plugins::text2video::ffmpeg::run_cmd;
use crate::plugins::text2video::models::{Content, Settings};
use crate::plugins::text2video::render::background::{self, fill_rect, fill_rounded_rect};
use crate::plugins::text2video::render::fonts::FontKit;
use crate::plugins::text2video::render::text;

type Rgb = [u8; 3];

struct Palette {
    white: Rgb,
    gray: Rgb,
    dark: Rgb,
}

fn scroll_palette(theme: &str) -> Palette {
    if theme == "dark" {
        Palette {
            white: [242, 243, 245],
            gray: [150, 158, 168],
            dark: [14, 17, 24],
        }
    } else {
        Palette {
            white: [30, 30, 40],
            gray: [80, 85, 95],
            dark: [245, 246, 248],
        }
    }
}

fn rgba(c: Rgb) -> Rgba<u8> {
    Rgba([c[0], c[1], c[2], 255])
}

/// 渲染滚动视频 + 封面，返回产物路径与时长
#[allow(clippy::too_many_arguments)]
pub fn render_scroll_video(
    content: &Content,
    settings: &Settings,
    fonts: &FontKit,
    ffmpeg: &Path,
    out_dir: &Path,
    bg_index: usize,
    theme: &str,
    bgm: Option<&Path>,
    cancel: &AtomicBool,
) -> Result<(PathBuf, PathBuf, f64), AppError> {
    let w = settings.width;
    let h = settings.height;
    let fps = settings.fps;
    let margin = settings.margin as f32;
    let margin_top = settings.margin_top as f32;
    let margin_bottom = settings.margin_bottom as f32;
    let text_w = w as f32 - margin * 2.0;

    let title_size = settings.title_font_size;
    let meta_size = settings.meta_font_size;
    let body_size = settings.body_font_size;
    let footer_size = settings.footer_font_size;
    let chip_size = settings.chip_font_size;
    let tlh = (title_size * settings.title_line_height).trunc();
    let blh = (body_size * settings.body_line_height).trunc();

    let pal = scroll_palette(theme);
    let (ar, ag, ab) = background::parse_hex(&settings.accent_color);
    let accent = [ar, ag, ab, 255];

    // ── 布局 ──
    let (tag, _) = text::source_brand(&content.source_type);
    let chip_w = fonts.measure(tag, chip_size, true) + 44.0;
    let title_lines = text::wrap_text(&content.title, fonts, title_size, true, text_w);

    let mut meta_parts = vec![format!("@{}", content.author)];
    if content.voteups > 0 {
        meta_parts.push(format!("赞同 {}", content.voteups));
    }
    let credit = text::credit_line(&content.source_type, &content.author);
    let credit_tail = credit
        .split_once("· ")
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_else(|| credit.clone());
    meta_parts.push(credit_tail);
    let meta = meta_parts.join(" · ");

    let mut y = margin_top;
    let chip_y = y;
    y += 58.0;
    let title_y = y;
    // 标题与作者行留出更明显的间距
    y += title_lines.len() as f32 * tlh + 44.0;
    let meta_y = y;
    // 作者行与正文之间以留白分隔（不再画分隔条）
    y += 76.0;
    let mut body_items: Vec<(f32, Vec<String>)> = Vec::new();
    for para in &content.paragraphs {
        let lines = text::wrap_text(para, fonts, body_size, false, text_w);
        let line_count = lines.len();
        body_items.push((y, lines));
        y += line_count as f32 * blh + 34.0;
    }
    y += 26.0;
    let footer_y = y;
    y += 48.0;
    // 长图高度不得小于视口高度，否则 crop 无法配置（短内容时静态展示）
    let long_h = ((y + margin_bottom) as u32).max(h);

    // ── 背景长图 ──
    let bg = background::get_background(bg_index, &settings.accent_color, w, h, theme);
    let resized = image::imageops::resize(&bg, w, long_h, image::imageops::FilterType::Lanczos3);
    let blurred = image::imageops::fast_blur(&resized, 6.0);
    let mut canvas = background::decorate(&blurred, &settings.accent_color, theme);

    // ── 绘制 ──
    // 角标
    fill_rounded_rect(
        &mut canvas,
        margin,
        chip_y,
        margin + chip_w,
        chip_y + 52.0,
        14.0,
        accent,
    );
    let (asc, desc) = fonts.vertical_metrics(chip_size, true);
    let chip_top = chip_y + (52.0 - (asc - desc)) / 2.0;
    fonts.draw_text(
        &mut canvas,
        tag,
        margin + 22.0,
        chip_top,
        chip_size,
        true,
        rgba(pal.dark),
    );

    // 标题
    let mut ty = title_y;
    for line in &title_lines {
        fonts.draw_text(
            &mut canvas,
            line,
            margin,
            ty,
            title_size,
            true,
            rgba(pal.white),
        );
        ty += tlh;
    }
    // 元信息
    fonts.draw_text(
        &mut canvas,
        &meta,
        margin,
        meta_y,
        meta_size,
        false,
        rgba(pal.gray),
    );
    // 正文
    for (by, lines) in &body_items {
        let mut ly = *by;
        for line in lines {
            fonts.draw_text(
                &mut canvas,
                line,
                margin,
                ly,
                body_size,
                false,
                rgba(pal.white),
            );
            ly += blh;
        }
    }
    // 页脚
    let footer = "本视频由程序自动生成 · 仅作学习交流";
    let fw = fonts.measure(footer, footer_size, false);
    fonts.draw_text(
        &mut canvas,
        footer,
        (w as f32 - fw) / 2.0,
        footer_y,
        footer_size,
        false,
        rgba(pal.gray),
    );

    // ── 保存长图 ──
    std::fs::create_dir_all(out_dir)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("创建输出目录失败: {err}")))?;
    let frame_png = out_dir.join("_frame.png");
    canvas
        .save(&frame_png)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("长图保存失败: {err}")))?;

    // ── 时长与滚动表达式 ──
    let cps = settings.read_cps.max(0.1);
    let char_count = content.char_count() as f64;
    let mut read_dur = (char_count / cps).max(8.0);
    let lead = settings.lead_sec;
    let tail = settings.tail_sec;
    let mut duration = lead + read_dur + tail;
    if duration > settings.max_duration_sec {
        duration = settings.max_duration_sec;
        read_dur = (duration - lead - tail).max(0.1);
    }
    let y_expr = format!("min(ih-oh\\,(ih-oh)*max(0\\,(t-{lead:.2}))/{read_dur:.3})");
    let vf = format!("crop={w}:{h}:0:'{y_expr}'[v]");

    // ── ffmpeg 编码 ──
    let mut cmd = Command::new(ffmpeg);
    cmd.current_dir(out_dir);
    cmd.args([
        "-y",
        "-v",
        "error",
        "-loop",
        "1",
        "-framerate",
        &fps.to_string(),
        "-i",
        "_frame.png",
    ]);

    if let Some(bgm_path) = bgm {
        cmd.args(["-stream_loop", "-1", "-i"]).arg(bgm_path);
        let fade = settings.bgm_fade_sec;
        let fade_start = (duration - fade).max(0.0);
        let af = format!(
            "[1:a]atrim=0:{duration:.2},asetpts=PTS-STARTPTS,afade=t=out:st={fade_start:.2}:d={fade:.2},volume={vol:.2}[a]",
            vol = settings.bgm_volume
        );
        let filter = format!("{vf};{af}");
        cmd.args(["-filter_complex", &filter, "-map", "[v]", "-map", "[a]"]);
    } else {
        cmd.args(["-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo"]);
        cmd.args(["-filter_complex", &vf, "-map", "[v]", "-map", "1:a"]);
    }

    let video_name = "video.mp4";
    cmd.args([
        "-t",
        &format!("{duration:.2}"),
        "-c:v",
        "libx264",
        "-preset",
        &settings.preset,
        "-crf",
        &settings.crf.to_string(),
        "-pix_fmt",
        "yuv420p",
        "-r",
        &fps.to_string(),
        "-c:a",
        "aac",
        "-b:a",
        &settings.audio_bitrate,
        "-movflags",
        "+faststart",
        video_name,
    ]);

    log::info!("text2video 渲染: 时长 {duration:.1}s / 长图 {long_h}px / 阅读 {char_count:.0} 字");
    let result = run_cmd(&mut cmd, cancel);
    let _ = std::fs::remove_file(&frame_png);
    result?;

    // ── 封面 ──
    let cover_path = out_dir.join("cover.jpg");
    render_cover(content, settings, fonts, &bg, &cover_path, theme)?;

    Ok((out_dir.join(video_name), cover_path, duration))
}

fn cover_text_color(theme: &str) -> Rgb {
    if theme == "dark" {
        [245, 246, 248]
    } else {
        [30, 30, 40]
    }
}

/// 封面样式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverStyle {
    /// 纯色极简：主题底色（浅色=白）+ 居中标题
    Minimal,
    /// 渐变：带装饰的渐变底色 + 居中标题
    Gradient,
    /// 强调色底：整幅强调色 + 高对比标题
    AccentSolid,
    /// 顶部色条：主题底色 + 顶部强调色条
    TopBar,
}

impl CoverStyle {
    pub fn from_name(name: &str) -> Self {
        match name {
            "gradient" => CoverStyle::Gradient,
            "accent" => CoverStyle::AccentSolid,
            "topbar" => CoverStyle::TopBar,
            _ => CoverStyle::Minimal,
        }
    }
}

fn theme_solid(theme: &str) -> Rgb {
    if theme == "dark" {
        [22, 22, 28]
    } else {
        [255, 255, 255]
    }
}

/// 依据强调色明度选择高对比前景色
fn contrast_color(accent: (u8, u8, u8)) -> Rgb {
    let lum = 0.299 * accent.0 as f32 + 0.587 * accent.1 as f32 + 0.114 * accent.2 as f32;
    if lum > 165.0 {
        [28, 28, 36]
    } else {
        [245, 246, 248]
    }
}

/// 生成封面图像（不落盘，便于预览）
pub fn build_cover(
    content: &Content,
    settings: &Settings,
    fonts: &FontKit,
    bg: &RgbaImage,
    theme: &str,
) -> RgbaImage {
    let cw = settings.cover_width;
    let ch = settings.cover_height;
    let (ar, ag, ab) = background::parse_hex(&settings.accent_color);
    let accent = [ar, ag, ab, 255];
    let style = CoverStyle::from_name(&settings.cover_style);
    let margin = 90.0f32;
    let max_w = cw as f32 - margin * 2.0;

    let (mut img, title_color) = match style {
        CoverStyle::Minimal => (
            solid_image(cw, ch, theme_solid(theme)),
            cover_text_color(theme),
        ),
        CoverStyle::Gradient => {
            let base = image::imageops::resize(bg, cw, ch, image::imageops::FilterType::Lanczos3);
            let deco = background::decorate(&base, &settings.accent_color, theme);
            let img = if theme == "dark" {
                blend_rgb(&deco, [8, 10, 15], 0.25)
            } else {
                deco
            };
            (img, cover_text_color(theme))
        }
        CoverStyle::AccentSolid => (
            solid_image(cw, ch, [ar, ag, ab]),
            contrast_color((ar, ag, ab)),
        ),
        CoverStyle::TopBar => {
            let mut img = solid_image(cw, ch, theme_solid(theme));
            let bar_h = (ch as f32 * 0.02).max(20.0);
            fill_rect(&mut img, 0.0, 0.0, cw as f32, bar_h, accent);
            (img, cover_text_color(theme))
        }
    };

    // 标题：自适应字号，水平居中、垂直居中（略偏上）
    let mut size = settings.cover_title_size;
    let lines = loop {
        let wrapped = text::wrap_text(&content.title, fonts, size, true, max_w);
        if wrapped.len() <= 6 || size <= settings.cover_title_min_size {
            break wrapped;
        }
        size -= 8.0;
    };
    let lh = (size * 1.3).trunc();
    let total = lines.len() as f32 * lh;
    let mut y = ch as f32 * 0.46 - total / 2.0;
    for line in &lines {
        let line_w = fonts.measure(line, size, true);
        let x = ((cw as f32 - line_w) / 2.0).max(margin);
        fonts.draw_text(&mut img, line, x, y, size, true, rgba(title_color));
        y += lh;
    }

    img
}

fn solid_image(w: u32, h: u32, color: Rgb) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba([color[0], color[1], color[2], 255]))
}

fn render_cover(
    content: &Content,
    settings: &Settings,
    fonts: &FontKit,
    bg: &RgbaImage,
    out_jpg: &Path,
    theme: &str,
) -> Result<(), AppError> {
    let img = build_cover(content, settings, fonts, bg, theme);
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let file = std::fs::File::create(out_jpg)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("封面创建失败: {err}")))?;
    let mut encoder = JpegEncoder::new_with_quality(file, settings.cover_quality);
    encoder
        .encode_image(&rgb)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("封面保存失败: {err}")))?;
    Ok(())
}

fn blend_rgb(img: &RgbaImage, scrim: Rgb, alpha: f32) -> RgbaImage {
    let mut out = img.clone();
    for px in out.pixels_mut() {
        for i in 0..3 {
            px[i] = (px[i] as f32 * (1.0 - alpha) + scrim[i] as f32 * alpha)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        px[3] = 255;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::text2video::models::{Content, Settings};

    fn sample_content() -> Content {
        Content {
            title: "测试一个视频".to_string(),
            author: "测试作者".to_string(),
            voteups: 0,
            source_url: String::new(),
            source_type: "manual".to_string(),
            ref_id: "manual:test".to_string(),
            paragraphs: vec!["这是第一段测试正文，用于验证渲染流程可以正常完成。".to_string()],
            sentence_groups: vec![vec![
                "这是第一段测试正文，用于验证渲染流程可以正常完成。".to_string()
            ]],
        }
    }

    /// 端到端渲染冒烟测试：需要本机 ffmpeg 与内置字体。
    /// 运行：cargo test --manifest-path src-tauri/Cargo.toml render_smoke -- --ignored --nocapture
    #[test]
    #[ignore = "依赖本机 ffmpeg 与内置字体"]
    fn render_smoke() {
        let fonts = FontKit::load(Path::new("local-resources"), "", "").expect("字体加载失败");
        let settings: Settings = serde_json::from_value(serde_json::json!({})).expect("默认设置");
        let ffmpeg = crate::plugins::text2video::ffmpeg::locate_ffmpeg("").expect("ffmpeg 未找到");
        let out_dir = std::env::temp_dir().join("text2video-smoke");
        let _ = std::fs::remove_dir_all(&out_dir);
        let cancel = AtomicBool::new(false);

        let started = std::time::Instant::now();
        let (video, cover, duration) = render_scroll_video(
            &sample_content(),
            &settings,
            &fonts,
            &ffmpeg,
            &out_dir,
            0,
            "dark",
            None,
            &cancel,
        )
        .expect("渲染失败");
        println!(
            "渲染完成: {:?} ({:.1}s), 用时 {:.2}s",
            video,
            duration,
            started.elapsed().as_secs_f32()
        );
        assert!(video.exists());
        assert!(cover.exists());
    }

    /// 生成深/浅色视频背景对比图。
    /// 运行：cargo test --manifest-path src-tauri/Cargo.toml background_samples -- --ignored --nocapture
    #[test]
    #[ignore = "用于人工预览视频背景"]
    fn background_samples() {
        let (w, h, long_h) = (1080u32, 1920u32, 2400u32);
        let accent = "#7eb8ff";
        let mut panels: Vec<RgbaImage> = Vec::new();
        for theme in ["dark", "light"] {
            let bg = background::get_background(0, accent, w, h, theme);
            let resized =
                image::imageops::resize(&bg, w, long_h, image::imageops::FilterType::Lanczos3);
            let blurred = image::imageops::fast_blur(&resized, 6.0);
            let canvas = background::decorate(&blurred, accent, theme);
            panels.push(image::imageops::resize(
                &canvas,
                405,
                900,
                image::imageops::FilterType::Triangle,
            ));
        }
        let mut sheet = RgbaImage::from_pixel(830, 900, Rgba([10, 10, 12, 255]));
        image::imageops::replace(&mut sheet, &panels[0], 0, 0);
        image::imageops::replace(&mut sheet, &panels[1], 425, 0);

        let out_dir = std::env::temp_dir().join("text2video-covers");
        let _ = std::fs::create_dir_all(&out_dir);
        let path = out_dir.join("_background-dark-light.jpg");
        let rgb = image::DynamicImage::ImageRgba8(sheet).to_rgb8();
        let file = std::fs::File::create(&path).expect("背景预览写入失败");
        JpegEncoder::new_with_quality(file, 92)
            .encode_image(&rgb)
            .expect("背景预览编码失败");
        println!("背景预览已生成: {}（左深右浅）", path.display());
    }

    /// 生成封面样式预览拼图（4 样式 × 2 主题）。
    /// 运行：cargo test --manifest-path src-tauri/Cargo.toml cover_samples -- --ignored --nocapture
    #[test]
    #[ignore = "用于人工预览封面样式"]
    fn cover_samples() {
        let fonts = FontKit::load(Path::new("local-resources"), "", "").expect("字体加载失败");
        let content = Content {
            title: "如何评价一个普通人的一生：从平凡到不凡的自我修养之路".to_string(),
            author: "测试作者".to_string(),
            voteups: 0,
            source_url: String::new(),
            source_type: "manual".to_string(),
            ref_id: "manual:sample".to_string(),
            paragraphs: vec!["示例正文。".to_string()],
            sentence_groups: vec![vec!["示例正文。".to_string()]],
        };

        let styles = [
            ("minimal", "纯色极简", CoverStyle::Minimal),
            ("gradient", "渐变", CoverStyle::Gradient),
            ("accent", "强调色底", CoverStyle::AccentSolid),
            ("topbar", "顶部色条", CoverStyle::TopBar),
        ];
        let themes = [("dark", "深色"), ("light", "浅色")];

        let (tw, th, label_h, gap, pad, header) = (360u32, 480u32, 34u32, 16u32, 20u32, 56u32);
        let cols = styles.len() as u32;
        let rows = themes.len() as u32;
        let sheet_w = pad * 2 + cols * tw + (cols - 1) * gap;
        let sheet_h = pad + header + rows * (th + label_h) + (rows - 1) * gap + pad;
        let mut sheet = RgbaImage::from_pixel(sheet_w, sheet_h, Rgba([18, 18, 22, 255]));

        let title_label = "封面样式预览";
        let tl_w = fonts.measure(title_label, 30.0, true);
        fonts.draw_text(
            &mut sheet,
            title_label,
            (sheet_w as f32 - tl_w) / 2.0,
            pad as f32 + 8.0,
            30.0,
            true,
            Rgba([240, 242, 246, 255]),
        );

        let out_dir = std::env::temp_dir().join("text2video-covers");
        let _ = std::fs::create_dir_all(&out_dir);

        for (r, (theme, theme_label)) in themes.iter().enumerate() {
            for (c, (key, style_label, _)) in styles.iter().enumerate() {
                let mut settings: Settings =
                    serde_json::from_value(serde_json::json!({ "coverStyle": key })).unwrap();
                settings.accent_color = "#7eb8ff".to_string();
                let bg = background::get_background(0, &settings.accent_color, 1080, 1920, theme);
                let cover = build_cover(&content, &settings, &fonts, &bg, theme);

                // 单张全尺寸预览
                let rgb = image::DynamicImage::ImageRgba8(cover.clone()).to_rgb8();
                let file = std::fs::File::create(out_dir.join(format!("{key}-{theme}.jpg")))
                    .expect("封面写入失败");
                JpegEncoder::new_with_quality(file, 90)
                    .encode_image(&rgb)
                    .expect("封面编码失败");

                // 拼图缩略图
                let thumb =
                    image::imageops::resize(&cover, tw, th, image::imageops::FilterType::Triangle);
                let x = pad + c as u32 * (tw + gap);
                let y = pad + header + r as u32 * (th + label_h + gap);
                image::imageops::replace(&mut sheet, &thumb, x as i64, y as i64);

                // 标签条
                fill_rect(
                    &mut sheet,
                    x as f32,
                    (y + th) as f32,
                    (x + tw) as f32,
                    (y + th + label_h) as f32,
                    [34, 34, 40, 255],
                );
                let label = format!("{style_label} · {theme_label}");
                let lw = fonts.measure(&label, 22.0, false);
                fonts.draw_text(
                    &mut sheet,
                    &label,
                    x as f32 + (tw as f32 - lw) / 2.0,
                    (y + th) as f32 + 6.0,
                    22.0,
                    false,
                    Rgba([230, 232, 238, 255]),
                );
            }
        }

        let sheet_path = out_dir.join("_contact-sheet.jpg");
        let rgb = image::DynamicImage::ImageRgba8(sheet).to_rgb8();
        let file = std::fs::File::create(&sheet_path).expect("拼图写入失败");
        JpegEncoder::new_with_quality(file, 92)
            .encode_image(&rgb)
            .expect("拼图编码失败");
        println!("封面预览已生成: {}", sheet_path.display());
    }
}
