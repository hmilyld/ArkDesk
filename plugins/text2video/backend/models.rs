//! text2video 数据结构与设置项。
//!
//! 设置项由前端 `useToolSettings` 传入，字段 camelCase（与 `shared.ts` 的默认值对齐）；
//! 渲染/清洗的细粒度几何常量保留为代码内置，不进入设置。

use serde::{Deserialize, Serialize};

/// 抓取阶段产出的原始草稿（未清洗）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDraft {
    pub title: String,
    pub html: String,
    pub author: String,
    pub voteups: i64,
    pub source_url: String,
    pub source_type: String,
    pub ref_id: String,
}

/// 清洗后的正文（分段落 + 分句）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub title: String,
    pub author: String,
    pub voteups: i64,
    pub source_url: String,
    pub source_type: String,
    pub ref_id: String,
    pub paragraphs: Vec<String>,
    pub sentence_groups: Vec<Vec<String>>,
}

impl Content {
    /// 正文总字数（按 Unicode 码点计，对齐 Python `len(str)`）
    pub fn char_count(&self) -> usize {
        self.paragraphs.iter().map(|p| p.chars().count()).sum()
    }
}

/// 手动输入内容（标题 / 作者 / 正文，正文按空行分段）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInput {
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub content: String,
    /// 来源类型：manual（原创）/ ai（AI 创作）
    #[serde(default = "default_manual_source")]
    pub source: String,
    /// 对应草稿 id（草稿箱批量生成时传入），生成成功后据此移除草稿
    #[serde(default)]
    pub draft_id: Option<i64>,
}

fn default_manual_source() -> String {
    "manual".into()
}

/// 草稿写入参数（有 id 则更新，无则新增）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub content: String,
    #[serde(default = "default_manual_source")]
    pub source: String,
}

/// 草稿
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub content: String,
    pub source: String,
    /// 已生成视频对应的处理记录 ref_id（NULL = 尚未生成）
    pub generated_ref_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// AI 接口配置（来自全局设置）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// AI 生成文章请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerateRequest {
    /// structured | free
    pub mode: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub prompt: String,
}

/// AI 生成结果
#[derive(Debug, Clone, Serialize)]
pub struct AiArticle {
    pub title: String,
    pub body: String,
}

/// meta.json 结构
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMeta {
    pub raw_title: String,
    pub suggested_title: String,
    pub original_url: String,
    pub author: String,
    pub voteups: i64,
    pub source_type: String,
    pub template: String,
    pub duration_sec: f64,
    pub char_count: usize,
    pub created_at: String,
    pub files: MetaFiles,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaFiles {
    pub video: String,
    pub cover: String,
}

/// 单条生成结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub ref_id: String,
    pub title: String,
    pub status: String,
    pub detail: String,
    pub video: String,
    pub cover: String,
}

/// 一次运行汇总
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub rendered: usize,
    pub cancelled: bool,
    pub output_dir: String,
    pub results: Vec<RunResult>,
}

/// 进度消息（经 Channel 流式上报）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMsg {
    /// 阶段：cleaning / rendering / done / skip / error
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

/// 环境检查结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub ffmpeg_ok: bool,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub fonts_ok: bool,
    pub font_regular: String,
    pub font_bold: String,
    pub output_dir: String,
    pub message: String,
}

/// 历史记录行
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRow {
    pub ref_id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub detail: String,
    pub video: String,
    pub author: String,
    pub source: String,
    pub content: String,
    pub created_at: String,
}

/// 生成参数（前端传入，字段 camelCase）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateOptions {
    #[serde(default)]
    pub template: String,
    pub settings: Settings,
}

/// 全部可配置项（默认值与前端 `shared.ts` 的 DEFAULTS 保持一致）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    // 输出
    #[serde(default)]
    pub output_dir: String,
    #[serde(default)]
    pub open_after_render: bool,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,

    // 内容
    #[serde(default = "default_min_chars")]
    pub min_chars: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    #[serde(default)]
    pub banned_keywords: Vec<String>,
    #[serde(default = "default_min_sentence_chars")]
    pub min_sentence_chars: usize,
    #[serde(default = "default_max_sentence_chars")]
    pub max_sentence_chars: usize,
    #[serde(default = "default_min_block_chars")]
    pub min_block_chars: usize,

    // 画面
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_accent")]
    pub accent_color: String,
    #[serde(default = "default_margin")]
    pub margin: u32,
    #[serde(default = "default_margin_top")]
    pub margin_top: u32,
    #[serde(default = "default_margin_bottom")]
    pub margin_bottom: u32,

    // 排版
    #[serde(default = "default_title_font")]
    pub title_font_size: f32,
    #[serde(default = "default_meta_font")]
    pub meta_font_size: f32,
    #[serde(default = "default_body_font")]
    pub body_font_size: f32,
    #[serde(default = "default_footer_font")]
    pub footer_font_size: f32,
    #[serde(default = "default_chip_font")]
    pub chip_font_size: f32,
    #[serde(default = "default_title_lh")]
    pub title_line_height: f32,
    #[serde(default = "default_body_lh")]
    pub body_line_height: f32,

    // 节奏
    #[serde(default = "default_read_cps")]
    pub read_cps: f64,
    #[serde(default = "default_lead")]
    pub lead_sec: f64,
    #[serde(default = "default_tail")]
    pub tail_sec: f64,
    #[serde(default = "default_max_duration")]
    pub max_duration_sec: f64,

    // 编码
    #[serde(default = "default_crf")]
    pub crf: u32,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default = "default_audio_bitrate")]
    pub audio_bitrate: String,
    #[serde(default)]
    pub bgm_dir: String,
    #[serde(default = "default_bgm_volume")]
    pub bgm_volume: f64,
    #[serde(default = "default_bgm_fade")]
    pub bgm_fade_sec: f64,

    // 封面
    #[serde(default = "default_cover_width")]
    pub cover_width: u32,
    #[serde(default = "default_cover_height")]
    pub cover_height: u32,
    #[serde(default = "default_cover_title_size")]
    pub cover_title_size: f32,
    #[serde(default = "default_cover_title_min_size")]
    pub cover_title_min_size: f32,
    #[serde(default = "default_cover_quality")]
    pub cover_quality: u8,
    #[serde(default = "default_cover_style")]
    pub cover_style: String,

    // 环境
    #[serde(default)]
    pub ffmpeg_path: String,
    #[serde(default)]
    pub font_regular_path: String,
    #[serde(default)]
    pub font_bold_path: String,
}

fn default_theme_mode() -> String {
    "random".into()
}
fn default_min_chars() -> usize {
    200
}
fn default_max_chars() -> usize {
    8000
}
fn default_min_sentence_chars() -> usize {
    8
}
fn default_max_sentence_chars() -> usize {
    42
}
fn default_min_block_chars() -> usize {
    4
}
fn default_width() -> u32 {
    1080
}
fn default_height() -> u32 {
    1920
}
fn default_fps() -> u32 {
    30
}
fn default_accent() -> String {
    "#7eb8ff".into()
}
fn default_margin() -> u32 {
    80
}
fn default_margin_top() -> u32 {
    70
}
fn default_margin_bottom() -> u32 {
    430
}
fn default_title_font() -> f32 {
    68.0
}
fn default_meta_font() -> f32 {
    32.0
}
fn default_body_font() -> f32 {
    44.0
}
fn default_footer_font() -> f32 {
    28.0
}
fn default_chip_font() -> f32 {
    30.0
}
fn default_title_lh() -> f32 {
    1.38
}
fn default_body_lh() -> f32 {
    1.66
}
fn default_read_cps() -> f64 {
    6.0
}
fn default_lead() -> f64 {
    1.2
}
fn default_tail() -> f64 {
    2.2
}
fn default_max_duration() -> f64 {
    300.0
}
fn default_crf() -> u32 {
    20
}
fn default_preset() -> String {
    "medium".into()
}
fn default_audio_bitrate() -> String {
    "160k".into()
}
fn default_bgm_volume() -> f64 {
    0.85
}
fn default_bgm_fade() -> f64 {
    2.0
}
fn default_cover_width() -> u32 {
    1080
}
fn default_cover_height() -> u32 {
    1440
}
fn default_cover_title_size() -> f32 {
    96.0
}
fn default_cover_title_min_size() -> f32 {
    64.0
}
fn default_cover_quality() -> u8 {
    92
}
fn default_cover_style() -> String {
    "minimal".into()
}
