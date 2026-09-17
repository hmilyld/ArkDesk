/**
 * 纯逻辑示例（无 Vue 依赖）：格式、解析、校验、转换等放本目录。
 * 组件与视图通过相对路径引用：`import { greet } from '../lib/example'`。
 */
export function greet(name: string, template?: string): string {
  const trimmed = name.trim();
  if (trimmed === '') throw new Error('名称不能为空');
  return (template ?? '你好，{name}！').replace('{name}', trimmed);
}
