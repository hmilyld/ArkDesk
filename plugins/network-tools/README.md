# 网络工具

网络请求调试工具集，含「接口测试」与「请求拦截」两个独立工具。

- 插件 id：`network-tools`
- 工具 id：`http-client`（`/tool/http-client`）、`http-interceptor`（`/tool/http-interceptor`）
- 数据库迁移 scope：`network-tools`（仅接口测试使用）
- 接口测试传输：复用框架 `@/core/http` 的 `http.send()`（Rust `http_send`）
- 请求拦截引擎：插件后端 `backend/intercept/`（hudsucker，见 `AGENTS.md`）

## 接口测试

模拟 GET/POST 等请求，自定义参数、请求头、Cookie、请求体并查看响应，适合对照浏览器 F12 抓包调试 API。

- **请求**：方法 / URL / Query / Headers / Cookies（可手动粘贴原始 Cookie 头）/ Body（Raw·表单·multipart 文件·二进制）/ 认证（Basic·Bearer·API Key）
- **变量**：`{{name}}` 引用环境变量，发送前替换；存在未解析变量时阻止发送
- **响应**：状态 / 耗时 / 大小、Body（Pretty·Raw）、Headers、Set-Cookie（可复制回 Cookie 编辑器）；二进制可另存为
- **集合与历史**：任意层级集合树；文件夹行的 `+` 直接在其内新建请求，行内「移动到文件夹」菜单或拖拽可归置请求；每次发送自动记录历史（超上限自动裁剪）
- **cURL**：生成 curl；粘贴 F12「Copy as cURL」一键导入
- **导入/导出**：自有 JSON 格式，支持单个请求 / 集合 / 全部（含环境变量）
- **取消**：长请求可中止
- **设置**：默认超时 / 重定向 / SSL 校验、响应截断阈值、历史开关与条数、代理（跟随全局或自定义）、User-Agent、环境变量

## 请求拦截

本地 MITM 代理（仅监听 `127.0.0.1`）：把系统/浏览器代理指向本机端口后，捕获 HTTP(S) 流量并查看、编辑、重放。

- **捕获**：请求/响应头 + 体（可配置上限，超出截断）；WebSocket 帧只读记录
- **列表**：按 Host / 方法 / 状态 / URL 过滤；内存环形缓冲（条数上限）
- **详情与重放**：复用 `RequestTabs` 编辑请求，经 `http_send` 重放；捕获响应只读对比
- **HTTPS**：首次启动自动生成本机根证书，需按提示安装信任（macOS/Windows/Firefox）；证书固定的 App 无法拦截
- **系统代理**：可一键把系统 HTTP(S) 代理指向本地（macOS 弹框授权 / Windows HKCU），停止代理时自动还原；未开启时需手动在系统或浏览器设置代理
- **限制**：不启用 HTTP/2、无上游代理、不支持 mTLS、仅绑定回环地址；忽略系统代理的应用（部分 Electron/Java/游戏）仍抓不到

> 使用须知：安装根证书需管理员权限；首次绑定可能触发防火墙授权。

## 设置项

插件级设置（`frontend/settings/Settings.vue`，存 `tools.network-tools`）：

- **接口测试**：默认超时、跟随重定向、校验 SSL、响应体上限、历史开关与条数、代理模式（跟随全局/自定义）、默认 User-Agent、当前环境
- **请求拦截**：监听端口（0 = 首次启动时询问）、是否抓体、体大小上限、流量条数上限、WebSocket 帧数上限、启动时自动设置系统代理

## 数据与命令

- 数据库表（scope `network-tools`）：`network_tools_collections` / `network_tools_requests` /
  `network_tools_drafts` / `network_tools_environments` / `network_tools_env_vars` / `network_tools_history`
- 后端命令（`backend/mod.rs`，`network_tools_` 前缀）：
  - 生命周期：`proxy_start` / `proxy_stop` / `proxy_status` / `proxy_configure`
  - 流量：`flows_list` / `flow_get` / `flows_clear` / `flow_body_temp`
  - WebSocket：`ws_list` / `ws_get` / `ws_clear`
  - 系统代理：`system_proxy_enable` / `system_proxy_disable` / `system_proxy_status`
  - 根证书：`ca_info` / `ca_export` / `ca_regenerate`
- 事件：`network-tools://flow`、`network-tools://ws`（批量推送，前端 `listen` 订阅）

## 开发

- 界面：`frontend/views/HttpClient.vue`、`frontend/views/Interceptor.vue`；设置：`frontend/settings/Settings.vue`
- 数据：`frontend/schema.ts` + `backend/migrations.rs`
- 纯逻辑：`frontend/curl.ts`（cURL 互转）、`frontend/variables.ts`（变量替换），单测在仓库根 `tests/`
- 启动：`pnpm tauri dev`（前后端均构建期自动注册）

> 架构与约定见仓库根 `README.md` / `AGENTS.md`，插件本地约定见同目录 `AGENTS.md`。
