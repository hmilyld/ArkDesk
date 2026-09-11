import globals from 'globals';
import pluginVue from 'eslint-plugin-vue';
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript';
import prettierConfig from 'eslint-config-prettier/flat';

export default defineConfigWithVueTs(
  {
    ignores: [
      'dist/**',
      'src-tauri/**',
      'node_modules/**',
      'plugins/**/backend/**',
      'src/core/ipc/commands.gen.ts',
    ],
  },
  pluginVue.configs['flat/recommended'],
  vueTsConfigs.recommended,
  {
    // shadcn-vue CLI 生成组件：不手改，豁免部分样式规则
    files: ['src/components/ui/**'],
    rules: {
      'vue/require-default-prop': 'off',
    },
  },
  {
    files: ['**/*.{ts,vue}'],
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
  prettierConfig
);
