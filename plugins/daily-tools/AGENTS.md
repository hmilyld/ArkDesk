# daily-tools 插件 — 开发约定（AGENTS）

> 面向 AI/开发者的本地约定。功能说明见同目录 `README.md`；项目级文档只做引用。

## 工具

`plugin.json` 声明三个工具（同一插件、前后端同处）：

- `file-converter` → `frontend/views/FileConverter.vue`
- `image-ocr` → `frontend/views/ImageOcr.vue`
- `crypto` → `frontend/views/CryptoTool.vue`（加解密，见下）

## 加解密后端布局

命令**必须**写在 `backend/mod.rs`（`build.rs` 只扫描该文件），一律 `daily_tools_*` 前缀、
返回 `Result<T, AppError>`。逻辑与测试放子模块：

| 文件                      | 职责                                                                        |
| ------------------------- | --------------------------------------------------------------------------- |
| `crypto/dto.rs`           | 请求/响应 DTO（camelCase，与前端字段对齐）                                  |
| `crypto/runner.rs`        | 文件任务进度/取消（`run_file_task`）、`build_sym_params`、`parse_operation` |
| `crypto/progress.rs`      | `StreamObserver` 抽象 + `CallbackObserver`（去抖）/`NoopObserver`           |
| `crypto/passphrase.rs`    | OpenSSL `enc` 口令：`Salted__`、EVP_BytesToKey(MD5)、PBKDF2                 |
| `crypto/hash.rs`          | MD5/SHA-1/2/3/SM3/CRC32 + HMAC，流式 `Hasher`                               |
| `crypto/encoding.rs`      | Base64（标准/URL-safe/padding）、Base32、Hex、URL                           |
| `crypto/text_encoding.rs` | 字符集转换（encoding_rs）                                                   |
| `crypto/fileio.rs`        | 文件流式哈希/编码（分块 + padding 对齐 + 清理部分输出）                     |
| `crypto/symmetric.rs`     | AES/SM4/DES/3DES，ECB/CBC/CTR + AES-GCM                                     |
| `crypto/asymmetric.rs`    | RSA、SM2（`gmcrypto-core`）                                                 |
| `crypto/kdf.rs`           | PBKDF2 / scrypt / Argon2id                                                  |
| `crypto/misc.rs`          | UUID、时间戳、Unicode 转义                                                  |

约定：命令名全局唯一（`build.rs` 构建期查重）；错误一律 `AppError`（中文、通俗）；
密钥/派生材料不落盘、不写日志，派生 key/iv 用 `zeroize` 清理。

## 进度与取消

文件类命令（`daily_tools_hash_file` / `daily_tools_encode_file` / `daily_tools_symmetric_file`）
额外接收框架注入的 `app: AppHandle` 与前端传入的 `task_id`，经 `runner::run_file_task`：

- `crate::tasks::begin` 注册取消令牌，`StreamObserver::advance` 每块检查取消并回传进度；
- 前端 `core/tasks` 的 `tasksState` 读取状态、`cancelTask` 取消；
- 取消/失败时删除未完成的输出文件，返回 `AppError("CANCELLED", "操作已取消")`。

## 前端约定

- `frontend/views/CryptoTool.vue`：单页多标签（摘要/编码/对称/非对称/口令派生/杂项）。
- 统一交互组件（`frontend/components/crypto/`）：
  - 输出一律用 `ResultBox`（面板 + 标题栏 + 右上角复制）；
  - 既是输入又常作为生成结果的（密钥）用 `CopyableTextarea`；
  - 复制按钮统一 `CopyButton`；文件任务进度用 `TaskProgress`（读 `core/tasks`）。
- 调用：一律经 `composables/useCryptoCall.ts`（`ipc` + loading/error + 请求序号防乱序）。
- 错误提示：`frontend/shared.ts` 的 `errorMessage()`（兼容 `AppError` 对象，翻译 Tauri 参数错误）；
  非 UTF-8 输入格式会在选择非「文本 (UTF-8)」时给出黄色提示。
- 选项常量集中在 `frontend/crypto-shared.ts`；不在组件里重复定义。
- 输入框固定高度 + 超出滚动（`field-sizing: fixed` 覆盖 shadcn Textarea 的自动增高）。

## 依赖（fork 本地层）

加解密相关 Rust 依赖（`src-tauri/Cargo.toml` 的 `local plugin deps` 段）：

- 摘要/编码：`digest` `md-5` `sha1` `sha2` `sha3` `sm3` `hmac` `data-encoding` `percent-encoding` `encoding_rs` `crc32fast`
- 对称：`aes` `des` `sm4` `cbc` `ecb` `ctr` `aes-gcm` `cipher` `block-padding` `pbkdf2` `zeroize` `getrandom`
- 非对称/KDF/杂项：`rsa` `gmcrypto-core` `rand_core` `scrypt` `argon2` `uuid`

> SM2 加解密不用 RustCrypto `sm2`（仅 SM2DSA 签名），改用 `gmcrypto-core`
> （constant-time，含 ASN.1/raw C1C3C2、PKCS#8/SPKI/PEM）。
> `rand_core`（getrandom feature）用于 RSA 的 `OsRng`；`getrandom` 用于密钥生成与 SM2 签名 RNG。

## 已知限制

- GCM 仅 AES、仅文本；SM4 提供 ECB/CBC/CTR（无 GCM）；文件模式仅 ECB/CBC/CTR。
- 文件「编码预览」会把结果整段返回，超大文件请用「编码并保存为文件」。
- 非对称仅文本；RSA 密钥生成与 Argon2 为同步命令（已有 loading，不冻结 webview）。
- 输入框中的密钥/口令为普通字符串（仅派生材料 `zeroize`）。

## 校验

```bash
pnpm lint && pnpm build          # 前端
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs   # Rust
```
