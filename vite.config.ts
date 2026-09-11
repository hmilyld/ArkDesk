import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri` 与插件后端源码
      ignored: ['**/src-tauri/**', '**/plugins/**/backend/**'],
    },
  },

  // 代码拆分：第三方库按用途分块，避免主包过大（构建告警）
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-vue': ['vue', 'vue-router', 'pinia'],
          'vendor-ui': [
            '@lucide/vue',
            'reka-ui',
            'class-variance-authority',
            'clsx',
            'tailwind-merge',
            'vue-sonner',
          ],
          'vendor-markdown': ['marked'],
          'vendor-search': ['fuse.js'],
          'vendor-db': ['drizzle-orm'],
        },
      },
    },
  },
}));
