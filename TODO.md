# TODO

> 待办清单。**完成即删除对应行**；新增项按 `- [ ] YYYY-MM-DD …` 格式登记（日期为登记日）。
> 适用范围见 [`docs/README.md`](docs/README.md)；改动归属见 [`docs/ownership.json`](docs/ownership.json)。

## 待办

- [ ] 2026-09-17 包管理器双轨：CI 用 `npm ci` + `package-lock.json`，本地与文档用 pnpm —— 择一统一（属上游）
- [ ] 2026-09-17 上游缺口：`plugins/<id>/frontend/preview.ts` 未纳入 `plugins/README.md` 白名单，暂以 `lint-plugins-ignore` 豁免；上游纳入后删除豁免
- [ ] 2026-09-17 自动化：`pnpm sync:upstream`（按 `docs/ownership.json` 导出框架补丁供 PocketArk 应用）尚未实现
- [ ] 2026-09-17 network-tools：启动 / 停止 / 系统代理 / CA 结果仍用 toast（系统级操作暂无就地位置）；侧栏「不能移动到自身或其子级」受组件契约限制
- [ ] 2026-09-17 tender-optimizer：蒙特卡洛测算无进度与取消（需后端加 channel/task 参数）；「评分参数」面板在预览夹具下 `paramsRow` 为空
- [ ] 2026-09-17 真机验证：macOS vibrancy 与 Windows Mica Alt 观感，按 [`docs/design-macos.md`](docs/design-macos.md) / [`docs/design-windows.md`](docs/design-windows.md) §11 逐项确认并回填
