# Network Tools

A collection of network request debugging tools, containing two independent tools: "API Testing" and "Request Interception".

- Plugin id: `network-tools`
- Tool ids: `http-client` (`/tool/http-client`), `http-interceptor` (`/tool/http-interceptor`)
- Database migration scope: `network-tools` (used only by API Testing)
- API Testing transport: reuses the framework's `@/core/http` `http.send()` (Rust `http_send`)
- Request Interception engine: the plugin backend `backend/intercept/` (hudsucker, see `AGENTS.en.md`)

## API Testing

Simulate GET/POST and other requests, customize parameters, headers, cookies and the request body, and view the response; well suited to debugging APIs against browser F12 network captures.

- **Request**: method / URL / Query / Headers / Cookies (you can paste a raw Cookie header manually) / Body (Raw · form · multipart file · binary) / Auth (Basic · Bearer · API Key)
- **Variables**: `{{name}}` references environment variables and is substituted before sending; sending is blocked while any unresolved variable remains
- **Response**: status / time / size, Body (Pretty · Raw), Headers, Set-Cookie (can be copied back into the Cookie editor); binaries can be saved as a file
- **Collections and history**: a collection tree of any depth; the `+` on a folder row creates a new request directly inside it, and the in-row "Move to folder" menu or drag-and-drop files requests away; every send is recorded to history automatically (trimmed automatically when over the limit)
- **cURL**: generate curl; paste F12 "Copy as cURL" for one-click import
- **Import/export**: its own JSON format, supporting a single request / a collection / everything (including environment variables)
- **Cancel**: long-running requests can be aborted
- **Settings**: default timeout / redirects / SSL verification, response truncation threshold, history toggle and count, proxy (follow global or custom), User-Agent, environment variables

## Request Interception

A local MITM proxy (listening only on `127.0.0.1`): point the system/browser proxy at the local port, then capture HTTP(S) traffic to view, edit and replay it.

- **Capture**: request/response headers + body (configurable limit, truncated beyond it); WebSocket frames are recorded read-only
- **List**: filter by Host / method / status / URL; in-memory ring buffer (with a count limit)
- **Detail and replay**: reuse `RequestTabs` to edit the request and replay it via `http_send`; the captured response is available read-only for comparison
- **HTTPS**: a local root certificate is generated automatically on first start and must be installed/trusted as prompted (macOS/Windows/Firefox); apps with certificate pinning cannot be intercepted
- **System proxy**: one click points the system HTTP(S) proxy at the local one (macOS authorization dialog / Windows HKCU), and stopping the proxy restores it automatically; when not enabled you must set the proxy manually in the system or browser
- **Limitations**: HTTP/2 is not enabled, there is no upstream proxy, mTLS is unsupported, and it only binds to the loopback address; apps that ignore the system proxy (some Electron/Java/games) still cannot be captured

> Usage notes: installing the root certificate requires administrator privileges; the first bind may trigger a firewall authorization prompt.

## Settings

Plugin-level settings (`frontend/settings/Settings.vue`, stored under `tools.network-tools`):

- **API Testing**: default timeout, follow redirects, verify SSL, response body limit, history toggle and count, proxy mode (follow global/custom), default User-Agent, current environment
- **Request Interception**: listen port (0 = ask on first start), whether to capture bodies, body size limit, flow count limit, WebSocket frame limit, automatically set the system proxy on start

## Data and Commands

- Database tables (scope `network-tools`): `network_tools_collections` / `network_tools_requests` /
  `network_tools_drafts` / `network_tools_environments` / `network_tools_env_vars` / `network_tools_history`
- Backend commands (`backend/mod.rs`, `network_tools_` prefix):
  - Lifecycle: `proxy_start` / `proxy_stop` / `proxy_status` / `proxy_configure`
  - Flows: `flows_list` / `flow_get` / `flows_clear` / `flow_body_temp`
  - WebSocket: `ws_list` / `ws_get` / `ws_clear`
  - System proxy: `system_proxy_enable` / `system_proxy_disable` / `system_proxy_status`
  - Root certificate: `ca_info` / `ca_export` / `ca_regenerate`
- Events: `network-tools://flow`, `network-tools://ws` (batched push, subscribed on the frontend via `listen`)

## Development

- UI: `frontend/views/HttpClient.vue`, `frontend/views/Interceptor.vue`; settings: `frontend/settings/Settings.vue`
- Data: `frontend/schema.ts` + `backend/migrations.rs`
- Pure logic: `frontend/lib/curl.ts` (cURL conversion), `frontend/lib/variables.ts` (variable substitution), with unit tests at the repo root `tests/`
- Startup: `pnpm tauri dev` (both frontend and backend are auto-registered at build time)

> For architecture and conventions see the repo root `README.en.md` / `AGENTS.en.md`; for plugin-local conventions see `AGENTS.en.md` in the same directory.
