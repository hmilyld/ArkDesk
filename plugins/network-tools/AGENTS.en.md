# network-tools Plugin — Development Conventions (AGENTS)

> Local conventions for AI/developers. For feature documentation see `README.en.md` in the same directory; project-level docs are referenced only.

## Tools

`plugin.json` declares the tools (same plugin, frontend and backend colocated):

- `http-client` "API Testing" → `frontend/views/HttpClient.vue`
- `http-interceptor` "Request Interception" → `frontend/views/Interceptor.vue` (an independent tool, not coupled to API Testing on the same page)

## Backend

- Commands are always defined in `backend/mod.rs` (`build.rs` only scans that file, and they are registered automatically
  at build time); the function name = the frontend call name, with a `network_tools_` prefix and returning `Result<T, AppError>`.
- "API Testing" has no commands of its own: HTTP goes through the framework's `http_send`, and data goes through `@/core/db`.
- "Request Interception" commands are thin wrappers around `backend/intercept/` (see below).
- `backend/migrations.rs` provides the migrations (scope = `network-tools`); `build.rs` only aggregates
  `migrations.rs` once it discovers `backend/mod.rs`, so that file must be kept.

## Request Interception Backend (intercept/)

| File              | Responsibility                                                                                                            |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`          | Lifecycle (start/stop/status/configure), batched event sending, command implementations                                   |
| `dto.rs`          | DTOs (camelCase) + `ProxyConfig`/`ProxyStatus`/`FlowRecord`/`Ws*`/`CaInfo`                                                |
| `state.rs`        | Shared state: flow ring buffer, WS records, config, event outlet (`Outbox`)                                               |
| `handler.rs`      | `HttpHandler`/`WebSocketHandler`: Tee body capture, request-response pairing, read-only WS                                |
| `body.rs`         | `TeeBody` (pass-through + copy, without breaking streaming), decompression by `Content-Encoding`, text-ification          |
| `ca.rs`           | Root certificate generation/loading/export/info (rcgen + ring, stored in `app_data_dir/network-tools`)                    |
| `system_proxy.rs` | One-click system proxy: macOS `networksetup` (osascript privilege escalation) / Windows HKCU; includes backup and restore |

Key conventions:

- **Binding and lifecycle**: binds on its own via `TcpListener::bind(("127.0.0.1", port))` (a port already in use returns a clear error); `with_graceful_shutdown(oneshot)` stops it; start/stop are idempotent.
- **Request-response pairing**: hudsucker clones a handler per request, and `handle_request`/`handle_response`/`handle_error` share the same instance → pairing uses the per-instance `pending: Option<PendingFlow>` (not relying on `client_addr`); HTTP/1.1 only, **http2 is not enabled**.
- **Body capture**: `TeeBody` copies to a `BodySink` while forwarding (limit `maxBodyKb`); when the response body stream ends it triggers the `on_end` callback to persist it. **The forwarded bytes and headers are not modified.**
- **Events**: batched (about every 200ms) sends to `network-tools://flow` / `network-tools://ws` (plugin-private, bypassing `core/events`); the frontend subscribes with `listen` from `@tauri-apps/api/event`.
- **Decompression**: `gzip/deflate/br/zstd` decompresses a **copy** for display, without affecting forwarding; the charset uses `encoding_rs`.
- **Binary replay**: `network_tools_flow_body_temp` writes the captured body into a cache directory and returns the path, which the frontend sets as a `binary` body type.
- **System proxy**: `network_tools_system_proxy_enable/disable/status`; before setting, the original proxy configuration is backed up to
  `app_data_dir/network-tools/system-proxy.json` and restored from the backup when reverting; `stop()` restores it automatically.
  On macOS it escalates via an osascript dialog, and on Windows it uses the HKCU registry (no administrator needed).
  `frontend/setup.ts` runs a self-check at startup: if it finds the system proxy still managed while the proxy is not running
  (usually leftover from a previous force-kill), it restores it automatically, so the network is not pointed at a dead port.
- **Limitations**: no upstream proxy / mTLS unsupported / HTTP/2 not enabled; apps with certificate pinning cannot be intercepted;
  apps that ignore the system proxy (some Electron/Java/games) cannot be captured; truly "all traffic" would require a TUN transparent proxy (not implemented).

> The dependencies belong to the local layer (`hudsucker` + `http-body-util` + `flate2`/`brotli`/`zstd`);
> note that hudsucker pulls in `aws-lc-sys` indirectly (see [`docs/en/local.md`](../../docs/en/local.md)).

## Frontend

- `frontend/views/HttpClient.vue`: a single workspace (collections/history on the left, request + response on the right).
- Data-layer composables (`frontend/composables/`): `useSend` / `useCollections` / `useHistory` / `useEnvironments` / `useDraft`.
- Shared model: `frontend/shared.ts` (`HttpRequestSpec` / `HttpResponseView` / setting defaults / `buildSendOptions`).
  The send page and the future interceptor page share the same model and components.
- Pure logic: `frontend/lib/curl.ts` (cURL conversion), `frontend/lib/variables.ts` (variable substitution), `frontend/lib/transfer.ts` (its own JSON import/export format);
  Tauri dependencies **must not** be introduced, so that vitest (node environment) can test it. For error formatting use `frontend/lib/error.ts` (which may depend on `@/core/errors`).
- Settings singleton: `httpSettings` in `frontend/lib/settings-store.ts` (shared by the send page and the settings page; do not call `useToolSettings` separately in each).
- Components (`frontend/components/`) are always props-driven with no business state, and are reused by the interceptor page:
  `KeyValueEditor` / `BodyEditor` / `AuthEditor` / `RequestTabs` / `ResponsePanel` / `CollectionsSidebar` / `CurlDialog`.
- Request Interception frontend: `views/Interceptor.vue` + `components/intercept/` (`FlowList`/`FlowDetail`/`WsDetail`/`CaDialog`/`PortDialog`)
  - `composables/useInterceptor.ts` (state/commands/event subscription); pure logic is in `intercept-shared.ts` (filtering, `flowToSpec`, `flowToResponseView`).
- Startup hook: `frontend/setup.ts` (called during framework startup; see the "System proxy" self-check).
- Shared UI helpers are centralized in `frontend/shared.ts`: `httpMethodClass` / `statusBadgeVariant` / `urlPath` / `copyToClipboard`; do not reimplement them inside components.

## Key Conventions

- **Cookie**: there is no automatic cookie jar (`cookieMode: 'none'`). The Cookie editor is the single source of truth; if a `Cookie` header is written manually,
  its content is merged with the editor's on send (`buildSendOptions`). The response `Set-Cookie` is display-only and can be copied back into the editor with one click.
- **Auth**: Authorization/API Key is synthesized on send; if a header with the same name is written manually, the manual one wins and a warning is shown.
- **Content-Type**: an explicit request header wins; otherwise it is set automatically according to the body type; the multipart Content-Type is generated by the framework (any user override is ignored).
- **Variables**: only enabled fields are resolved; sending is **blocked** while any unresolved variable remains.
- **History**: stores a snapshot of the request with variables already substituted at send time; the response body is stored truncated to the threshold.

## Settings

`HttpSettings` from `frontend/shared.ts`, stored under `tools.network-tools` via `useToolSettings` (see `settings-store.ts` for the singleton).

- API Testing: `timeoutMs` / `followRedirects` / `verifySsl` / `maxResponseKb` / `historyEnabled` /
  `historyLimit` / `proxyMode` / `proxyUrl` / `defaultUserAgent` / `activeEnvironmentId`
- Request Interception: `interceptorPort` (0 = ask on first start) / `interceptorRecordBodies` /
  `interceptorMaxBodyKb` / `interceptorMaxFlows` / `interceptorMaxWsFrames` /
  `interceptorAutoSystemProxy`

## Validation

```bash
pnpm lint && pnpm build                          # frontend
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs      # Rust (intercept engine + migration aggregation)
pnpm test                                        # vitest (curl / variables / import-export / interceptor pure logic)
```

End-to-end smoke test (ignored; needs network/loopback):

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib proxy_ -- --ignored
```
