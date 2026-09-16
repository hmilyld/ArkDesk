/**
 * text2video 插件共享：设置默认值、类型与常量。
 *
 * 设置项与 Rust 侧 `models::Settings` 一一对应（camelCase），
 * 通过模块级单例 `settings` 在各视图/设置面板间共享同一响应式对象。
 */

import { toast } from 'vue-sonner';
import { useToolSettings } from '@/core/plugins';
import { ipc } from '@/core/ipc';
import { normalizeError } from '@/core/errors';

/**
 * 提取用户可读的错误信息。
 *
 * `ipc` 抛出的是规范化后的 `AppError` 对象（非 Error 实例），直接 `String(err)`
 * 会得到 `[object Object]`；此函数统一兼容 AppError / Error / 字符串 / 未知值。
 */
export function errorMessage(value: unknown): string {
  return normalizeError(value).message;
}

export interface Text2VideoSettings {
  // 输出
  outputDir: string;
  openAfterRender: boolean;
  themeMode: 'random' | 'dark' | 'light';

  // 内容
  minChars: number;
  maxChars: number;
  bannedKeywords: string[];
  minSentenceChars: number;
  maxSentenceChars: number;
  minBlockChars: number;

  // 画面
  width: number;
  height: number;
  fps: number;
  accentColor: string;
  margin: number;
  marginTop: number;
  marginBottom: number;

  // 排版
  titleFontSize: number;
  metaFontSize: number;
  bodyFontSize: number;
  footerFontSize: number;
  chipFontSize: number;
  titleLineHeight: number;
  bodyLineHeight: number;

  // 节奏
  readCps: number;
  leadSec: number;
  tailSec: number;
  maxDurationSec: number;

  // 编码
  crf: number;
  preset: string;
  audioBitrate: string;
  bgmDir: string;
  bgmVolume: number;
  bgmFadeSec: number;

  // 封面
  coverWidth: number;
  coverHeight: number;
  coverTitleSize: number;
  coverTitleMinSize: number;
  coverQuality: number;
  coverStyle: string;

  // 环境
  ffmpegPath: string;
  fontRegularPath: string;
  fontBoldPath: string;
}

export const DEFAULTS: Text2VideoSettings = {
  outputDir: '',
  openAfterRender: false,
  themeMode: 'random',

  minChars: 200,
  maxChars: 8000,
  bannedKeywords: [],
  minSentenceChars: 8,
  maxSentenceChars: 42,
  minBlockChars: 4,

  width: 1080,
  height: 1920,
  fps: 30,
  accentColor: '#7eb8ff',
  margin: 80,
  marginTop: 70,
  marginBottom: 430,

  titleFontSize: 68,
  metaFontSize: 32,
  bodyFontSize: 44,
  footerFontSize: 28,
  chipFontSize: 30,
  titleLineHeight: 1.38,
  bodyLineHeight: 1.66,

  readCps: 6,
  leadSec: 1.2,
  tailSec: 2.2,
  maxDurationSec: 300,

  crf: 20,
  preset: 'medium',
  audioBitrate: '160k',
  bgmDir: '',
  bgmVolume: 0.85,
  bgmFadeSec: 2,

  coverWidth: 1080,
  coverHeight: 1440,
  coverTitleSize: 96,
  coverTitleMinSize: 64,
  coverQuality: 92,
  coverStyle: 'minimal',

  ffmpegPath: '',
  fontRegularPath: '',
  fontBoldPath: '',
};

/** 模块级单例：生成页 / 设置页共享同一响应式配置 */
export const settings = useToolSettings<Text2VideoSettings>('text2video', DEFAULTS);

// ── 类型 ─────────────────────────────────────────────────────────

export interface ManualInput {
  title: string;
  author: string;
  content: string;
  /** manual | ai */
  source?: string;
  /** 草稿箱批量生成时传入，生成成功后由后端移除对应草稿 */
  draftId?: number;
}

/** 标题 / 作者 / 正文（手动输入、草稿编辑共用的表单值） */
export interface ArticleValue {
  title: string;
  author: string;
  content: string;
}

export interface Draft {
  id: number;
  title: string;
  author: string;
  content: string;
  source: string;
  /** 已生成视频对应的处理记录 refId（null/undefined = 尚未生成） */
  generatedRefId?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AiConfig {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface AiGenerateRequest {
  mode: 'structured' | 'free';
  topic?: string;
  style?: string;
  wordCount?: number;
  prompt?: string;
}

export interface AiArticle {
  title: string;
  body: string;
}

export interface EnvStatus {
  ffmpegOk: boolean;
  ffmpegPath: string;
  ffprobePath: string;
  fontsOk: boolean;
  fontRegular: string;
  fontBold: string;
  outputDir: string;
  message: string;
}

export interface ProgressMsg {
  stage: string;
  current: number;
  total: number;
  message: string;
}

export interface RunResult {
  refId: string;
  title: string;
  status: string;
  detail: string;
  video: string;
  cover: string;
}

export interface RunSummary {
  rendered: number;
  cancelled: boolean;
  outputDir: string;
  results: RunResult[];
}

export interface HistoryRow {
  refId: string;
  kind: string;
  title: string;
  status: string;
  detail: string;
  video: string;
  author: string;
  /** manual | ai */
  source: string;
  /** 生成时使用的文章正文（文字素材备份） */
  content: string;
  createdAt: string;
}

// ── 常量 ─────────────────────────────────────────────────────────

export const THEME_OPTIONS = [
  { value: 'random', label: '随机' },
  { value: 'dark', label: '深色' },
  { value: 'light', label: '浅色' },
] as const;

export const COVER_STYLE_OPTIONS = [
  { value: 'minimal', label: '纯色极简' },
  { value: 'gradient', label: '渐变' },
  { value: 'accent', label: '强调色底' },
  { value: 'topbar', label: '顶部色条' },
] as const;

export const PRESET_OPTIONS = [
  'ultrafast',
  'superfast',
  'veryfast',
  'faster',
  'fast',
  'medium',
  'slow',
  'slower',
  'veryslow',
] as const;

/** 处理状态 → 中文标签 */
export const STATUS_LABELS: Record<string, string> = {
  done: '已完成',
  render_failed: '渲染失败',
  skipped_empty: '无正文',
  skipped_short: '过短',
  skipped_long: '过长',
  skipped_sensitive: '含敏感词',
};

export function statusLabel(status: string): string {
  return STATUS_LABELS[status] ?? status;
}

/** 打开产物 / 在文件夹中显示（各视图共用，统一错误提示） */
export async function openArtifact(path: string, reveal = false): Promise<void> {
  if (!path) return;
  try {
    await ipc('text2video_open', { path, reveal });
  } catch (err) {
    toast.error(`打开失败：${errorMessage(err)}`);
  }
}

export const STAGE_LABELS: Record<string, string> = {
  cleaning: '清洗',
  rendering: '渲染',
  done: '完成',
  skip: '跳过',
  error: '错误',
};

/** 需要数值兜底（输入框被清空时 v-model.number 会得到空串）的设置项 */
const NUMERIC_SETTING_KEYS = [
  'minChars',
  'maxChars',
  'minSentenceChars',
  'maxSentenceChars',
  'minBlockChars',
  'width',
  'height',
  'fps',
  'margin',
  'marginTop',
  'marginBottom',
  'titleFontSize',
  'metaFontSize',
  'bodyFontSize',
  'footerFontSize',
  'chipFontSize',
  'titleLineHeight',
  'bodyLineHeight',
  'readCps',
  'leadSec',
  'tailSec',
  'maxDurationSec',
  'crf',
  'bgmVolume',
  'bgmFadeSec',
  'coverWidth',
  'coverHeight',
  'coverTitleSize',
  'coverTitleMinSize',
  'coverQuality',
] as const satisfies readonly (keyof Text2VideoSettings)[];

/**
 * 规范化设置对象：数值字段若非有限数字则回退默认值，并做必要下限钳制，
 * 避免空输入框（''）导致 Rust 侧反序列化失败。
 */
export function normalizeSettings(input: Text2VideoSettings): Text2VideoSettings {
  const out = { ...input } as Text2VideoSettings;
  const record = out as unknown as Record<string, unknown>;
  for (const key of NUMERIC_SETTING_KEYS) {
    const value = Number(record[key]);
    record[key] = Number.isFinite(value) ? value : DEFAULTS[key];
  }
  out.width = Math.max(1, Math.round(out.width));
  out.height = Math.max(1, Math.round(out.height));
  out.fps = Math.max(1, Math.round(out.fps));
  out.crf = Math.min(51, Math.max(0, Math.round(out.crf)));
  out.coverWidth = Math.max(1, Math.round(out.coverWidth));
  out.coverHeight = Math.max(1, Math.round(out.coverHeight));
  out.coverQuality = Math.min(100, Math.max(1, Math.round(out.coverQuality)));
  out.maxSentenceChars = Math.max(1, Math.round(out.maxSentenceChars));
  return out;
}
