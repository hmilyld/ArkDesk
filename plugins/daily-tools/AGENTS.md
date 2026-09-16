# daily-tools 插件 — 开发约定（AGENTS）

> 面向 AI/开发者的本地约定。功能说明见同目录 `README.md`；项目级文档只做引用。

## 工具

`plugin.json` 声明四个工具（同一插件、前后端同处）：

- `file-converter` → `frontend/views/FileConverter.vue`
- `image-ocr` → `frontend/views/ImageOcr.vue`
- `crypto` → `frontend/views/CryptoTool.vue`（加解密，见下）
- `json-table` → `frontend/views/JsonTable.vue`（JSON 表格，见下）

## JSON 表格（json-table）

统一中间表示 `Table` / `Cell` 定义在 `frontend/table/model.ts`：四种格式各自实现解析与
序列化，任意两两互转。**文本类格式全部在前端**，Rust 只做 xlsx 读写。

| 文件                                         | 职责                                                             |
| -------------------------------------------- | ---------------------------------------------------------------- |
| `frontend/table/model.ts`                    | `Table`/`Cell`/`TableError`、列并集与列名规范化、行补齐          |
| `frontend/table/options.ts`                  | 格式枚举、日期模式、预览行数、上限等常量（前后端上限需人工对齐） |
| `frontend/table/path.ts`                     | 数据路径解析（`$.a.b`、`items[0]`、`[*]`）与候选数组路径扫描     |
| `frontend/table/json-convert.ts`             | JSON ↔ Table（键并集、嵌套 JSON 序列化、类型还原）               |
| `frontend/table/markdown.ts`                 | Table ↔ GFM 管道表（`marked` 解析；`\|` 与 `<br>` 转义）         |
| `frontend/table/csv.ts`                      | Table ↔ CSV（RFC 4180 状态机、防公式注入）                       |
| `frontend/table/cell-text.ts`                | 文本 ↔ 单元格（保守类型推断、按日期模式渲染）                    |
| `frontend/table/convert.ts`                  | 文本源解析 / 文本目标序列化的统一入口 + 预览截断                 |
| `frontend/table/xlsx.ts`                     | 前端侧 IPC 包装（列名规范化在后端读回后补齐）                    |
| `frontend/components/table/TablePreview.vue` | Excel 目标的网格预览                                             |
| `backend/xlsx/{dto,read,write}.rs`           | calamine 读 / rust_xlsxwriter 写；命令薄函数在 `backend/mod.rs`  |
| `backend/xlsx/tests.rs`                      | 临时文件回环测试（类型、日期、表头、裁剪、注入、非法输入）       |

约定：`t` 取值 `s|n|b|d|e`；`v === null` 表示空单元格；日期 `v` 为 ISO 8601、`n` 为
原始序列号（仅用于「序列号」输出模式）。写出 xlsx 一律 `write_string`/`write_number`/
`write_boolean`/`write_datetime_with_format`，**不得使用 `write_formula`**（天然免注入）。
前端纯逻辑测试在仓库根 `tests/daily-tools-json-table.spec.ts`。

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

### 选项区（按 Apple HIG 的 grouped form）

选项统一用 `@/components/settings` 的 `SettingsSection`（分组小标题 + 分隔行）与 `SettingsRow`
（左标题 + 第二行灰色说明 / 右控件）；放进 Panel 内时给 `SettingsSection` 传
`class="rounded-md bg-transparent"`，避免卡片套卡片。

- **一行一个设置，标题与控件必须在同一行内**：开关用 `Switch size="sm"`（HIG：分组表单用 mini
  switch，行高与按钮一致）。禁止用 `justify-between` 把 Label 与 Switch 拉到一行两端——在宽面板里
  看起来就是「一段没有控件的文字」（HIG 明确：switch 只放在列表行里，由行内容提供语境）。
- **组内说明**写成行的 `description`（第二行小字）；**组级说明**放 `SettingsSection` 的 `#footer`
  （灰色说明文字），不要做成填充色块、也不要混进选项行。
- 互斥多选（如「日期输出」三选一）用 `Select`（即平台的 pop-up button），不要用一排开关；
  需要分组嵌套/依赖关系时才考虑 checkbox 缩进（HIG：开关不要替代 checkbox）。
- 每个开关的标题要说清它控制什么；`hint`/说明只补充语境，不代替标题。

### 模块面板（Panel）

工具页里**每一个功能模块（设置 / 输入 / 输出 / 结果）都用 `@/components/tool/Panel` 包裹**，
不在页面上裸露模块。Panel 是框架级单一事实源（外框 + 头部条 + 正文），约定见根 `AGENTS.md`：

```vue
<Panel title="输入" :hint="格式名" body-class="space-y-4">
  <template #actions>
    <Button variant="secondary" size="sm">选择文件</Button>
  </template>
  <!-- 正文：默认 space-y-3 p-4，满幅场景用 body-class 覆盖 -->
</Panel>
```

- props：`title` / `hint`（标题后的补充信息，如格式名或行列统计）/ `body-class` / `header-class`；
  插槽：`default` / `title` / `actions`。
- **Panel 内不再嵌套卡片**：`ResultBox` / `CopyableTextarea` / `MarkdownPreview` / `TablePreview`
  自身要么已用 Panel、要么不带外框（`TablePreview` 为满幅网格），外框一律由所在 Panel 提供。
- 动作按钮放 `#actions`（`size="sm"`、图标 `size-3.5` + `mr-1`）；说明文案用
  `text-xs text-muted-foreground`；错误条用
  `rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive`。
- Panel 不预设栅格跨度：作为 `grid grid-cols-12` 的**直接子项**时必须自己在 `class` 上写跨度
  （如 `col-span-12 lg:col-span-6`），否则会塌成 1/12 宽；宽内容（表格 / 长文本）加 `min-w-0`
  让其内部滚动而不是撑破栅格。
- 全插件已按此收敛：`views/{FileConverter,ImageOcr,JsonTable}.vue`、`components/crypto/*`（含
  `ResultBox` / `CopyableTextarea`）、`components/MarkdownPreview.vue`。

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
- JSON 表格：类型推断默认关；日期格式仅影响文本输出；xlsx 读取上限 20 万行 / 2048 列
  （前后端上限分别在 `frontend/table/options.ts` 与 `backend/xlsx/read.rs`，修改需同步）；
  CSV 防注入的 `'` 前缀会改变字面文本（需无损往返时关闭）。

## 校验

```bash
pnpm lint && pnpm build          # 前端
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs   # Rust
```
