<!--
  图文视频工具设置面板：所有渲染/内容/编码参数均可配置，附「恢复默认」。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RotateCcw, FolderOpen, File } from '@lucide/vue';
import { COVER_STYLE_OPTIONS, DEFAULTS, PRESET_OPTIONS, THEME_OPTIONS, settings } from '../shared';

const bannedText = computed({
  get: () => settings.value.bannedKeywords.join('\n'),
  set: (value: string) => {
    settings.value.bannedKeywords = value
      .split(/[\n,，]/)
      .map((item) => item.trim())
      .filter(Boolean);
  },
});

function reset(): void {
  settings.value = JSON.parse(JSON.stringify(DEFAULTS));
}

async function pickDir(target: 'outputDir' | 'bgmDir'): Promise<void> {
  const path = await openDialog({ directory: true, multiple: false });
  if (typeof path === 'string') settings.value[target] = path;
}

async function pickFile(target: 'ffmpegPath' | 'fontRegularPath' | 'fontBoldPath'): Promise<void> {
  const path = await openDialog({ directory: false, multiple: false });
  if (typeof path === 'string') settings.value[target] = path;
}
</script>

<template>
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 space-y-4">
      <!-- 输出 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">输出</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
          <div class="space-y-1.5 lg:col-span-2">
            <Label>输出目录（留空 = 系统下载目录）</Label>
            <div class="flex gap-2">
              <Input v-model="settings.outputDir" placeholder="留空则输出到「下载」" />
              <Button variant="outline" size="icon" @click="pickDir('outputDir')">
                <FolderOpen class="size-4" />
              </Button>
            </div>
          </div>
          <div class="flex items-center justify-between gap-4">
            <Label>生成后自动打开目录</Label>
            <Switch v-model="settings.openAfterRender" />
          </div>
          <div class="space-y-1.5">
            <Label>主题</Label>
            <Select v-model="settings.themeMode">
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="opt in THEME_OPTIONS" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </section>

      <!-- 内容 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">内容过滤</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
          <div class="space-y-1.5">
            <Label>正文最少字数</Label>
            <Input v-model.number="settings.minChars" type="number" min="0" />
          </div>
          <div class="space-y-1.5">
            <Label>正文最多字数</Label>
            <Input v-model.number="settings.maxChars" type="number" min="0" />
          </div>
          <div class="space-y-1.5">
            <Label>最短句长</Label>
            <Input v-model.number="settings.minSentenceChars" type="number" min="0" />
          </div>
          <div class="space-y-1.5">
            <Label>最长句长</Label>
            <Input v-model.number="settings.maxSentenceChars" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>最短段落字数</Label>
            <Input v-model.number="settings.minBlockChars" type="number" min="0" />
          </div>
          <div class="space-y-1.5 lg:col-span-3">
            <Label>敏感词（每行或逗号分隔）</Label>
            <Textarea v-model="bannedText" :rows="3" placeholder="命中即跳过该文章" />
          </div>
        </div>
      </section>

      <!-- 画面 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">画面</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
          <div class="space-y-1.5">
            <Label>宽度</Label>
            <Input v-model.number="settings.width" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>高度</Label>
            <Input v-model.number="settings.height" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>帧率</Label>
            <Input v-model.number="settings.fps" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>强调色</Label>
            <div class="flex gap-2">
              <input
                v-model="settings.accentColor"
                type="color"
                class="h-9 w-12 shrink-0 cursor-pointer rounded-md border bg-transparent"
              />
              <Input v-model="settings.accentColor" />
            </div>
          </div>
          <div class="space-y-1.5">
            <Label>左右边距</Label>
            <Input v-model.number="settings.margin" type="number" min="0" />
          </div>
          <div class="space-y-1.5">
            <Label>上边距</Label>
            <Input v-model.number="settings.marginTop" type="number" min="0" />
          </div>
          <div class="space-y-1.5">
            <Label>下边距</Label>
            <Input v-model.number="settings.marginBottom" type="number" min="0" />
          </div>
        </div>
      </section>

      <!-- 排版 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">排版</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-4">
          <div class="space-y-1.5">
            <Label>标题字号</Label>
            <Input v-model.number="settings.titleFontSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>元信息字号</Label>
            <Input v-model.number="settings.metaFontSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>正文字号</Label>
            <Input v-model.number="settings.bodyFontSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>页脚字号</Label>
            <Input v-model.number="settings.footerFontSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>角标字号</Label>
            <Input v-model.number="settings.chipFontSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>标题行高</Label>
            <Input v-model.number="settings.titleLineHeight" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>正文行高</Label>
            <Input v-model.number="settings.bodyLineHeight" type="number" step="any" />
          </div>
        </div>
      </section>

      <!-- 节奏 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">滚动节奏</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-4">
          <div class="space-y-1.5">
            <Label>阅读速度（字/秒）</Label>
            <Input v-model.number="settings.readCps" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>片头停留（秒）</Label>
            <Input v-model.number="settings.leadSec" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>片尾停留（秒）</Label>
            <Input v-model.number="settings.tailSec" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>最长时长（秒）</Label>
            <Input v-model.number="settings.maxDurationSec" type="number" step="any" />
          </div>
        </div>
      </section>

      <!-- 编码 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">编码与配乐</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
          <div class="space-y-1.5">
            <Label>CRF（画质，越小越好）</Label>
            <Input v-model.number="settings.crf" type="number" min="0" max="51" />
          </div>
          <div class="space-y-1.5">
            <Label>preset</Label>
            <Select v-model="settings.preset">
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="p in PRESET_OPTIONS" :key="p" :value="p">{{ p }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>音频码率</Label>
            <Input v-model="settings.audioBitrate" />
          </div>
          <div class="space-y-1.5 lg:col-span-2">
            <Label>背景音乐目录（可选，随机选一首）</Label>
            <div class="flex gap-2">
              <Input v-model="settings.bgmDir" placeholder="留空则无背景音乐" />
              <Button variant="outline" size="icon" @click="pickDir('bgmDir')">
                <FolderOpen class="size-4" />
              </Button>
            </div>
          </div>
          <div class="space-y-1.5">
            <Label>配乐音量（0–1）</Label>
            <Input v-model.number="settings.bgmVolume" type="number" step="any" min="0" max="1" />
          </div>
          <div class="space-y-1.5">
            <Label>配乐淡出（秒）</Label>
            <Input v-model.number="settings.bgmFadeSec" type="number" step="any" min="0" />
          </div>
        </div>
      </section>

      <!-- 封面 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">封面</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
          <div class="space-y-1.5">
            <Label>封面样式</Label>
            <Select v-model="settings.coverStyle">
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="opt in COVER_STYLE_OPTIONS" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>封面宽度</Label>
            <Input v-model.number="settings.coverWidth" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>封面高度</Label>
            <Input v-model.number="settings.coverHeight" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>封面标题字号</Label>
            <Input v-model.number="settings.coverTitleSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>封面标题最小字号</Label>
            <Input v-model.number="settings.coverTitleMinSize" type="number" step="any" />
          </div>
          <div class="space-y-1.5">
            <Label>封面 JPEG 质量</Label>
            <Input v-model.number="settings.coverQuality" type="number" min="1" max="100" />
          </div>
        </div>
      </section>

      <!-- 环境 -->
      <section class="space-y-4 rounded-lg border bg-card p-4">
        <h3 class="text-sm font-semibold">环境与资源</h3>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <div class="space-y-1.5">
            <Label>ffmpeg 路径（留空 = 自动查找）</Label>
            <div class="flex gap-2">
              <Input v-model="settings.ffmpegPath" placeholder="自动查找 PATH / 常见目录" />
              <Button variant="outline" size="icon" @click="pickFile('ffmpegPath')">
                <File class="size-4" />
              </Button>
            </div>
          </div>
          <div class="space-y-1.5">
            <Label>中文字体 Regular（留空 = 内置）</Label>
            <div class="flex gap-2">
              <Input v-model="settings.fontRegularPath" placeholder="内置 Noto Sans CJK SC" />
              <Button variant="outline" size="icon" @click="pickFile('fontRegularPath')">
                <File class="size-4" />
              </Button>
            </div>
          </div>
          <div class="space-y-1.5">
            <Label>中文字体 Bold（留空 = 内置）</Label>
            <div class="flex gap-2">
              <Input v-model="settings.fontBoldPath" placeholder="内置 Noto Sans CJK SC Bold" />
              <Button variant="outline" size="icon" @click="pickFile('fontBoldPath')">
                <File class="size-4" />
              </Button>
            </div>
          </div>
        </div>
      </section>

      <div class="flex justify-end">
        <Button variant="outline" size="sm" @click="reset">
          <RotateCcw class="mr-1 size-3.5" />
          恢复默认
        </Button>
      </div>
    </div>
  </div>
</template>
