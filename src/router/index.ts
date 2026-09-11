/**
 * 应用路由：由工具插件注册表驱动生成。
 *
 * - 工具路由：/tool/:id（懒加载）
 * - 设置路由：/settings（框架页面）
 * - hash 模式：适配 Tauri 文件协议加载
 */
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';
import { getAllTools } from '@/core/plugins';
import Home from '@/layouts/Home.vue';
import Settings from '@/layouts/Settings.vue';

export function createAppRouter() {
  const toolRoutes: RouteRecordRaw[] = getAllTools().map((tool) => ({
    path: `/tool/${tool.meta.id}`,
    name: `tool-${tool.meta.id}`,
    component: tool.component,
    meta: {
      toolId: tool.meta.id,
      title: tool.meta.name,
      keepAlive: tool.keepAlive ?? true,
    },
  }));

  const routes: RouteRecordRaw[] = [
    ...toolRoutes,
    { path: '/settings', name: 'settings', component: Settings, meta: { title: '设置' } },
    // 首页：启动默认落地页（不再默认跳转首个工具，避免显示已隐藏的示例工具）
    { path: '/', name: 'home', component: Home, meta: { title: '首页', keepAlive: false } },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ];

  return createRouter({
    history: createWebHashHistory(),
    routes,
  });
}
