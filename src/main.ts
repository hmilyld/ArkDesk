/**
 * 应用启动引导，固定顺序：
 * 日志 → 异常处理 → Pinia → 设置加载 → 主题 → 插件注册 → 路由 → 挂载
 */
// 浏览器预览桥必须在其余模块之前安装（仅 dev + 非 Tauri 环境生效，见 src/dev/preview-bridge.ts）
import '@/dev/preview-bridge';
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { toast } from 'vue-sonner';

import App from './App.vue';
import '@/assets/index.css';
import { initLogger } from '@/core/logger';
import { installErrorHandlers, setGlobalErrorReporter } from '@/core/errors';
import { initTheme } from '@/core/theme';
import { createPluginContext, registerPlugins, runPluginSetups } from '@/core/plugins';
import { initUpdater } from '@/core/updater';
import { initOpenWith, onOpenFiles } from '@/core/open-with';
import { initTasks } from '@/core/tasks';
import { isMac } from '@/core/platform';
import { createAppRouter } from '@/router';
import { useSettingsStore } from '@/stores/settings';

async function bootstrap(): Promise<void> {
  await initLogger();

  const app = createApp(App);
  const pinia = createPinia();
  app.use(pinia);

  installErrorHandlers(app);

  // 禁止 webview 浏览器右键菜单（Reload / Inspect Element 等非应用菜单）；
  // 可编辑元素放行，保留系统复制粘贴
  document.addEventListener('contextmenu', (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest('input, textarea, [contenteditable="true"], [contenteditable=""]')) return;
    event.preventDefault();
  });

  // macOS 标记：供平台差异样式使用（如需平台特判的 CSS 可用 html.mac 选择器）
  if (isMac) document.documentElement.classList.add('mac');

  // 全局错误通知通道：toast（错误详情已由 errors 模块写入日志）
  // 错误需要更长阅读时间：6s（其余类型走 Toaster 全局默认 3.5s）
  setGlobalErrorReporter((error) => {
    toast.error(error.message, {
      description: `错误码: ${error.code}`,
      duration: 6000,
    });
  });

  initTheme();

  const settings = useSettingsStore(pinia);
  await settings.init();

  // 在线更新：仅主窗口注册监听与检查（避免多窗口重复）
  if (getCurrentWindow().label === 'main') {
    void initUpdater();
  }

  // 打开内容分发：默认仅在界面提示，具体处理由插件/页面注册 onOpenFiles 覆盖
  onOpenFiles((paths, source) => {
    toast.info(`收到 ${paths.length} 个文件（来源：${source}）`, {
      description: paths.slice(0, 3).join('、'),
    });
  });
  void initOpenWith();
  // 后台任务：注册任务状态监听（进度联动任务栏/Dock）
  void initTasks();

  registerPlugins();
  const ctx = createPluginContext();
  await runPluginSetups(ctx);

  const router = createAppRouter();
  app.use(router);
  await router.isReady();

  app.mount('#app');
}

void bootstrap().catch((err) => {
  // 启动早期（日志管道未就绪）失败的兜底：至少在控制台可见
  console.error('应用启动失败', err);
});
