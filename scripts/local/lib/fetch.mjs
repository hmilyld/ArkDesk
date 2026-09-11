/**
 * 资源下载公共逻辑：GitHub 代理、下载、SHA-256 校验、按需跳过。
 * （本地层 fork-owned；base 不含本文件。）
 *
 * 国内网络直连 GitHub 常失败，默认经 https://gh.javaing.com/ 代理；可用
 * 环境变量 GITHUB_PROXY 覆盖（设为空字符串则直连）。非 GitHub 域名不受影响。
 */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

const GITHUB_HOSTS =
  /^https?:\/\/(raw\.githubusercontent\.com|github\.com|objects\.githubusercontent\.com|codeload\.github\.com)\//;
const PROXY = process.env.GITHUB_PROXY ?? 'https://gh.javaing.com/';

export function sha256(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}

/** 为 GitHub 域名拼接代理前缀（非 GitHub 域名原样返回） */
export function viaGithubProxy(url) {
  if (!PROXY) return url;
  return GITHUB_HOSTS.test(url) ? `${PROXY.replace(/\/+$/, '')}/${url}` : url;
}

async function download(url) {
  // 单次尝试 180s 上限，避免连接卡死时无限等待（失败会回退直连）
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 180_000);
  try {
    const res = await fetch(url, { redirect: 'follow', signal: controller.signal });
    if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`);
    return Buffer.from(await res.arrayBuffer());
  } finally {
    clearTimeout(timer);
  }
}

/**
 * 确保目标文件存在且 SHA-256 校验通过；缺失/不符则下载（先代理后直连）。
 * @returns {Promise<boolean>} 成功为 true，失败为 false（调用方决定退出码）
 */
export async function ensureFile({ url, dest, sha256: expected, label }) {
  if (existsSync(dest) && sha256(readFileSync(dest)) === expected) {
    console.log(`✓ ${label} 已存在且校验通过`);
    return true;
  }
  mkdirSync(dirname(dest), { recursive: true });

  const candidates = [...new Set([viaGithubProxy(url), url])];
  let lastError;
  for (const candidate of candidates) {
    const tmp = `${dest}.download`;
    try {
      console.log(`↓ 下载 ${label}`);
      const buffer = await download(candidate);
      const actual = sha256(buffer);
      if (actual !== expected) {
        throw new Error(`SHA-256 不匹配（期望 ${expected}，实际 ${actual}）`);
      }
      writeFileSync(tmp, buffer);
      renameSync(tmp, dest);
      console.log(`✓ ${label} 完成（${(buffer.length / 1024 / 1024).toFixed(1)} MB）`);
      return true;
    } catch (err) {
      rmSync(tmp, { force: true });
      lastError = err;
    }
  }
  console.error(`✗ ${label} 失败：${lastError?.message}`);
  return false;
}
