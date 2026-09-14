<!--
  图文视频工具设置面板：所有渲染/内容/编码参数均可配置，附「恢复默认」。
  UI 统一使用 @/components/settings 的 SettingsSection / SettingsRow / SettingsField。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { SettingsField, SettingsRow, SettingsSection } from '@/components/settings';
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
  <div class="space-y-6">
    <!-- 输出 -->
    <SettingsSection title="输出">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-3">
        <SettingsField label="输出目录（留空 = 系统下载目录）" class="lg:col-span-2">
          <div class="flex gap-2">
            <Input v-model="settings.outputDir" placeholder="留空则输出到「下载」" />
            <Button variant="outline" size="icon" @click="pickDir('outputDir')">
              <FolderOpen class="size-4" />
            </Button>
          </div>
        </SettingsField>
        <SettingsField label="主题">
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
        </SettingsField>
      </div>
      <SettingsRow title="生成后自动打开目录">
        <Switch v-model="settings.openAfterRender" />
      </SettingsRow>
    </SettingsSection>

    <!-- 内容 -->
    <SettingsSection title="内容过滤">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-3">
        <SettingsField label="正文最少字数">
          <Input v-model.number="settings.minChars" type="number" min="0" />
        </SettingsField>
        <SettingsField label="正文最多字数">
          <Input v-model.number="settings.maxChars" type="number" min="0" />
        </SettingsField>
        <SettingsField label="最短句长">
          <Input v-model.number="settings.minSentenceChars" type="number" min="0" />
        </SettingsField>
        <SettingsField label="最长句长">
          <Input v-model.number="settings.maxSentenceChars" type="number" min="1" />
        </SettingsField>
        <SettingsField label="最短段落字数">
          <Input v-model.number="settings.minBlockChars" type="number" min="0" />
        </SettingsField>
        <SettingsField label="敏感词（每行或逗号分隔）" class="lg:col-span-3">
          <Textarea v-model="bannedText" :rows="3" placeholder="命中即跳过该文章" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 画面 -->
    <SettingsSection title="画面">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-3">
        <SettingsField label="宽度">
          <Input v-model.number="settings.width" type="number" min="1" />
        </SettingsField>
        <SettingsField label="高度">
          <Input v-model.number="settings.height" type="number" min="1" />
        </SettingsField>
        <SettingsField label="帧率">
          <Input v-model.number="settings.fps" type="number" min="1" />
        </SettingsField>
        <SettingsField label="强调色">
          <div class="flex gap-2">
            <input
              v-model="settings.accentColor"
              type="color"
              class="h-9 w-12 shrink-0 cursor-pointer rounded-md border bg-transparent"
            />
            <Input v-model="settings.accentColor" />
          </div>
        </SettingsField>
        <SettingsField label="左右边距">
          <Input v-model.number="settings.margin" type="number" min="0" />
        </SettingsField>
        <SettingsField label="上边距">
          <Input v-model.number="settings.marginTop" type="number" min="0" />
        </SettingsField>
        <SettingsField label="下边距">
          <Input v-model.number="settings.marginBottom" type="number" min="0" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 排版 -->
    <SettingsSection title="排版">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-4">
        <SettingsField label="标题字号">
          <Input v-model.number="settings.titleFontSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="元信息字号">
          <Input v-model.number="settings.metaFontSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="正文字号">
          <Input v-model.number="settings.bodyFontSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="页脚字号">
          <Input v-model.number="settings.footerFontSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="角标字号">
          <Input v-model.number="settings.chipFontSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="标题行高">
          <Input v-model.number="settings.titleLineHeight" type="number" step="any" />
        </SettingsField>
        <SettingsField label="正文行高">
          <Input v-model.number="settings.bodyLineHeight" type="number" step="any" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 节奏 -->
    <SettingsSection title="滚动节奏">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-4">
        <SettingsField label="阅读速度（字/秒）">
          <Input v-model.number="settings.readCps" type="number" step="any" />
        </SettingsField>
        <SettingsField label="片头停留（秒）">
          <Input v-model.number="settings.leadSec" type="number" step="any" />
        </SettingsField>
        <SettingsField label="片尾停留（秒）">
          <Input v-model.number="settings.tailSec" type="number" step="any" />
        </SettingsField>
        <SettingsField label="最长时长（秒）">
          <Input v-model.number="settings.maxDurationSec" type="number" step="any" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 编码 -->
    <SettingsSection title="编码与配乐">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-3">
        <SettingsField label="CRF（画质，越小越好）">
          <Input v-model.number="settings.crf" type="number" min="0" max="51" />
        </SettingsField>
        <SettingsField label="preset">
          <Select v-model="settings.preset">
            <SelectTrigger class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="p in PRESET_OPTIONS" :key="p" :value="p">{{ p }}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsField>
        <SettingsField label="音频码率">
          <Input v-model="settings.audioBitrate" />
        </SettingsField>
        <SettingsField label="背景音乐目录（可选，随机选一首）" class="lg:col-span-2">
          <div class="flex gap-2">
            <Input v-model="settings.bgmDir" placeholder="留空则无背景音乐" />
            <Button variant="outline" size="icon" @click="pickDir('bgmDir')">
              <FolderOpen class="size-4" />
            </Button>
          </div>
        </SettingsField>
        <SettingsField label="配乐音量（0–1）">
          <Input v-model.number="settings.bgmVolume" type="number" step="any" min="0" max="1" />
        </SettingsField>
        <SettingsField label="配乐淡出（秒）">
          <Input v-model.number="settings.bgmFadeSec" type="number" step="any" min="0" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 封面 -->
    <SettingsSection title="封面">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-3">
        <SettingsField label="封面样式">
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
        </SettingsField>
        <SettingsField label="封面宽度">
          <Input v-model.number="settings.coverWidth" type="number" min="1" />
        </SettingsField>
        <SettingsField label="封面高度">
          <Input v-model.number="settings.coverHeight" type="number" min="1" />
        </SettingsField>
        <SettingsField label="封面标题字号">
          <Input v-model.number="settings.coverTitleSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="封面标题最小字号">
          <Input v-model.number="settings.coverTitleMinSize" type="number" step="any" />
        </SettingsField>
        <SettingsField label="封面 JPEG 质量">
          <Input v-model.number="settings.coverQuality" type="number" min="1" max="100" />
        </SettingsField>
      </div>
    </SettingsSection>

    <!-- 环境 -->
    <SettingsSection title="环境与资源">
      <div class="grid grid-cols-1 gap-3 p-3 lg:grid-cols-2">
        <SettingsField label="ffmpeg 路径（留空 = 自动查找）">
          <div class="flex gap-2">
            <Input v-model="settings.ffmpegPath" placeholder="自动查找 PATH / 常见目录" />
            <Button variant="outline" size="icon" @click="pickFile('ffmpegPath')">
              <File class="size-4" />
            </Button>
          </div>
        </SettingsField>
        <SettingsField label="中文字体 Regular（留空 = 内置）">
          <div class="flex gap-2">
            <Input v-model="settings.fontRegularPath" placeholder="内置 Noto Sans CJK SC" />
            <Button variant="outline" size="icon" @click="pickFile('fontRegularPath')">
              <File class="size-4" />
            </Button>
          </div>
        </SettingsField>
        <SettingsField label="中文字体 Bold（留空 = 内置）">
          <div class="flex gap-2">
            <Input v-model="settings.fontBoldPath" placeholder="内置 Noto Sans CJK SC Bold" />
            <Button variant="outline" size="icon" @click="pickFile('fontBoldPath')">
              <File class="size-4" />
            </Button>
          </div>
        </SettingsField>
      </div>
    </SettingsSection>

    <div class="flex justify-end">
      <Button variant="outline" size="sm" @click="reset">
        <RotateCcw class="mr-1 size-3.5" />
        恢复默认
      </Button>
    </div>
  </div>
</template>
