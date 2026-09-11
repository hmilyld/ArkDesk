/**
 * 提交信息规范：Conventional Commits。
 * 允许中文描述；类型枚举见 @commitlint/config-conventional
 * （feat / fix / docs / style / refactor / perf / test / build / ci / chore / revert）。
 */
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // 中文描述较长，放宽标题长度
    'header-max-length': [2, 'always', 120],
    // 正文/脚注不限制行宽（常含代码与中文）
    'body-max-line-length': [0],
    'footer-max-line-length': [0],
  },
};
