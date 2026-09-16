//! 字体加载与字形绘制（ab_glyph；Noto Sans CJK OTF/CFF 可用）。
//!
//! 注意：PIL `ImageDraw.text` 的 y 坐标为「顶部（ascender）」，本模块以
//! `top_y` 为基准换算为基线，保持与 Python 版一致。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};

use crate::error::{code, AppError};

const REGULAR_FILE: &str = "NotoSansCJKsc-Regular.otf";
const BOLD_FILE: &str = "NotoSansCJKsc-Bold.otf";

/// 进程内字体缓存：字体解析较慢（每套约 16MB OTF），按路径组合复用
type FontCache = Mutex<HashMap<(String, String), Arc<FontKit>>>;
static FONT_CACHE: OnceLock<FontCache> = OnceLock::new();

pub struct FontKit {
    regular: FontVec,
    bold: FontVec,
    pub regular_path: String,
    pub bold_path: String,
}

impl FontKit {
    /// 载入并缓存字体；相同路径组合只解析一次。
    pub fn load_cached(
        resource_dir: &Path,
        regular_override: &str,
        bold_override: &str,
    ) -> Result<Arc<Self>, AppError> {
        let key = (regular_override.to_string(), bold_override.to_string());
        let cache = FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(fonts) = cache.lock().expect("字体缓存锁中毒").get(&key) {
            return Ok(fonts.clone());
        }
        let fonts = Arc::new(Self::load(resource_dir, regular_override, bold_override)?);
        cache
            .lock()
            .expect("字体缓存锁中毒")
            .insert(key, fonts.clone());
        Ok(fonts)
    }

    pub fn load(
        resource_dir: &Path,
        regular_override: &str,
        bold_override: &str,
    ) -> Result<Self, AppError> {
        let font_dir = resource_dir.join("fonts");
        let regular_path = resolve_font(
            regular_override,
            &font_dir.join(REGULAR_FILE),
            "中文字体（Regular）",
        )?;
        let bold_path = resolve_font(bold_override, &font_dir.join(BOLD_FILE), "中文字体（Bold）")
            .or_else(|_| resolve_font("", &font_dir.join(REGULAR_FILE), "中文字体（Regular）"))?;

        let regular = load_font(&regular_path)?;
        let bold = load_font(&bold_path)?;
        Ok(FontKit {
            regular,
            bold,
            regular_path: regular_path.to_string_lossy().to_string(),
            bold_path: bold_path.to_string_lossy().to_string(),
        })
    }

    fn font(&self, bold: bool) -> &FontVec {
        if bold {
            &self.bold
        } else {
            &self.regular
        }
    }

    fn px_scale(font: &FontVec, size: f32) -> PxScale {
        font.pt_to_px_scale(size)
            .unwrap_or_else(|| PxScale::from(size))
    }

    /// 文本宽度（px），逐字累加 advance（等价 PIL textlength，忽略 kerning）
    pub fn measure(&self, text: &str, size: f32, bold: bool) -> f32 {
        let font = self.font(bold);
        let scaled = font.as_scaled(Self::px_scale(font, size));
        text.chars()
            .map(|ch| scaled.h_advance(scaled.glyph_id(ch)))
            .sum()
    }

    /// 单字符 advance（px）
    pub fn advance(&self, ch: char, size: f32, bold: bool) -> f32 {
        let font = self.font(bold);
        let scaled = font.as_scaled(Self::px_scale(font, size));
        scaled.h_advance(scaled.glyph_id(ch))
    }

    /// 字体升降部（px）
    pub fn vertical_metrics(&self, size: f32, bold: bool) -> (f32, f32) {
        let font = self.font(bold);
        let scaled = font.as_scaled(Self::px_scale(font, size));
        (scaled.ascent(), scaled.descent())
    }

    /// 绘制一行文本；`top_y` 为文字顶部（与 PIL 对齐），内部换算基线
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &self,
        img: &mut RgbaImage,
        text: &str,
        x: f32,
        top_y: f32,
        size: f32,
        bold: bool,
        color: Rgba<u8>,
    ) {
        let font = self.font(bold);
        let scale = Self::px_scale(font, size);
        let scaled = font.as_scaled(scale);
        let baseline = top_y + scaled.ascent();
        let mut pen_x = x;
        for ch in text.chars() {
            let gid = scaled.glyph_id(ch);
            let glyph = gid.with_scale_and_position(scale, point(pen_x, baseline));
            if let Some(outlined) = scaled.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, coverage| {
                    let px = bounds.min.x + gx as f32;
                    let py = bounds.min.y + gy as f32;
                    blend_pixel(img, px, py, color, coverage);
                });
            }
            pen_x += scaled.h_advance(gid);
        }
    }
}

fn resolve_font(
    override_path: &str,
    default_path: &Path,
    label: &str,
) -> Result<PathBuf, AppError> {
    if !override_path.trim().is_empty() {
        let path = PathBuf::from(override_path.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::custom(
            code::NOT_FOUND,
            format!("{label} 路径不存在: {}", path.display()),
        ));
    }
    if default_path.is_file() {
        Ok(default_path.to_path_buf())
    } else {
        Err(AppError::custom(
            code::NOT_FOUND,
            format!("{label} 缺失: {}", default_path.display()),
        ))
    }
}

fn load_font(path: &Path) -> Result<FontVec, AppError> {
    let bytes = std::fs::read(path)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("字体读取失败: {err}")))?;
    FontVec::try_from_vec(bytes)
        .map_err(|err| AppError::custom(code::PLUGIN_ERROR, format!("字体解析失败: {err:?}")))
}

fn blend_pixel(img: &mut RgbaImage, x: f32, y: f32, color: Rgba<u8>, coverage: f32) {
    let xi = x.round();
    let yi = y.round();
    if xi < 0.0 || yi < 0.0 {
        return;
    }
    let (w, h) = img.dimensions();
    let (xu, yu) = (xi as u32, yi as u32);
    if xu >= w || yu >= h {
        return;
    }
    let alpha = (color[3] as f32 / 255.0) * coverage;
    if alpha <= 0.0 {
        return;
    }
    let px = img.get_pixel_mut(xu, yu);
    for i in 0..3 {
        px[i] = (px[i] as f32 * (1.0 - alpha) + color[i] as f32 * alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    px[3] = 255;
}
