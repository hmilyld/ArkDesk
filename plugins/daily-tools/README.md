# daily-tools 常用工具插件

## 功能

- **文件转换**：将 docx/xlsx/html/csv/pdf 等格式转换为 Markdown 源码，支持导出 .md 文件
- **图片 OCR**：识别图片中的文字（支持粘贴、点击选择），输出纯文本
- **加解密**：单页多标签，覆盖摘要/编码/对称/非对称/口令派生/杂项；全部在 Rust 后端实现
- **JSON 表格**：JSON 与 Markdown / CSV / Excel 表格互转（键为表头），见下

## JSON 表格工具

前端 `frontend/views/JsonTable.vue` + `frontend/lib/table/*`（转换核心，纯逻辑、可单测）；
后端仅负责 xlsx 读写：`backend/xlsx/*`，命令薄函数在 `backend/mod.rs`。

统一中间表示 `Table { name, columns, rows }`，单元格 `Cell { t: s|n|b|d|e, v, n?, f? }`
（字符串/数字/布尔/日期/Excel 错误值）；四种格式各自实现「解析」与「序列化」，两两互转。

| 方向        | 说明                                                                                                                                     |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| JSON → 表格 | 根为数组时直接使用；否则用**数据路径**（`data.list`、`$.a.b`、`items[0].rows`、`list[*]`）指定数组；解析失败会列出候选数组路径供一键填入 |
| 表格 → JSON | 单元格按类型还原（数字/布尔/空），日期按所选模式输出；多表输出 `{ 表名: [...] }`                                                         |
| Markdown    | GFM 管道表；`\|` 转义、单元格换行写为 `<br>`（可关，反向可还原）；内联标记原样保留                                                       |
| CSV         | UTF-8、RFC 4180、CRLF；导出默认对 `= + - @` 开头的字符串加 `'` 前缀防公式注入                                                            |
| Excel       | 单 Sheet 读取（可选工作表/首行表头/合并单元格填充/裁除空行列），多表写出多 Sheet；表头加粗 + 冻结首行 + 自动列宽                         |

约定与限制：

- 列 = 对象键并集（首次出现顺序）；嵌套对象/数组整体 `JSON.stringify` 进单元格。
- **类型推断默认关闭**：Markdown / CSV 解析出的单元格一律字符串；开启后按保守规则识别
  数字与布尔（拒绝前导零、`NaN`/`Infinity`/`0x`/科学计数法，整数位 > 15 位保留文本）。
- 日期三选：ISO 8601（默认）/ Excel 序列号 / 自定义格式（`yyyy-MM-dd HH:mm:ss`）。
  日期格式只影响文本输出；Excel 目标一律写为原生日期单元格。
- 预览截断 200 行，复制与导出始终全量；xlsx 单表上限 20 万行 / 2048 列。
- 超 2^53 的整数在 `JSON.parse` 阶段即已丢失精度，界面不额外处理。
- 防注入的 `'` 前缀只影响 CSV 文本（Excel 打开不执行公式），需要无损往返时关闭该选项。
- `calamine` 仅当单元格带日期格式时才判定为日期；无格式的序列号按数字读取。
- 文本文件读写复用框架 `file_read_text` / `file_write_text`；支持拖拽文件到窗口。

## 加解密工具

前端 `frontend/views/CryptoTool.vue` + `frontend/components/crypto/*`；后端
`backend/crypto/*`，命令薄函数在 `backend/mod.rs`（`daily_tools_*` 前缀）。

| 标签     | 能力                                                                                                                                                     |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 摘要     | MD5 / SHA-1/2 / SHA-3 / SM3 / CRC32、HMAC；文本与文件（流式）                                                                                            |
| 编码     | Base64（标准/URL-safe/去 padding）、Base32、Hex、URL；文本字符集转换（GBK/GB18030/Big5…）；文件流式编码                                                  |
| 对称     | AES-128/192/256、SM4、DES、3DES；ECB/CBC/CTR，AES-GCM；原始密钥或 OpenSSL `enc` 口令模式（`Salted__` + EVP_BytesToKey 或 `-pbkdf2`）；文件流式（非 GCM） |
| 非对称   | RSA（PKCS#8/SPKI 与 PKCS#1、OAEP/PKCS#1v15、PSS/PKCS#1v15）；SM2（ASN.1 DER 与 raw C1C3C2，签名/验签）                                                   |
| 口令派生 | PBKDF2-HMAC-SHA256 / scrypt / Argon2id                                                                                                                   |
| 杂项     | UUID v4/v7、时间戳转换、Unicode 转义/反转义                                                                                                              |

约定：密钥仅在内存中使用、不落盘、不写日志；DES/3DES/MD5/SHA-1 标注为不安全（仅兼容）。
SM2 基于 `gmcrypto-core`（RustCrypto `sm2` 不支持公钥加解密），依赖清单见根 `LOCAL.md`。

## 依赖

| crate     | 版本 | 用途                          | 备注              |
| --------- | ---- | ----------------------------- | ----------------- |
| `anytomd` | 1    | docx/xlsx/html/csv → Markdown | 纯 Rust           |
| `lopdf`   | 0.44 | PDF 文本提取                  | 纯 Rust           |
| `ocr-rs`  | 2.4  | PaddleOCR 图片 OCR            | 需要 MNN C++ 引擎 |
| `image`   | 0.25 | 图片加载                      | ocr-rs 依赖       |

JSON 表格工具复用框架已有依赖：`calamine`（读 xlsx）、`rust_xlsxwriter`（写 xlsx），
前端复用 `marked`（GFM 表格解析），未新增依赖。

加解密相关 RustCrypto / `gmcrypto-core` 依赖见根 `LOCAL.md`（fork 本地层）。

## 编译注意

### macOS

`ocr-rs` 依赖 MNN C++ 推理引擎，编译时需要 C++ 标准库头文件。新版 macOS Xcode Command Line Tools 把头文件移到了 SDK 子目录，需在 `~/.zshrc` 中设置：

```bash
export CXXFLAGS="-std=c++14 -I$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1"
```

已写入项目 `~/.zshrc`，新终端自动生效。

### Windows

- **MSVC 目标**：无特殊要求，MSVC 自带 C++ 标准库
- **GNU 目标**：已通过 `static-cpp-runtime` feature 静态链接 C++ 运行时，无需额外 DLL

## OCR 模型

模型文件位于 `src-tauri/resources/ocr-models/`，随应用分发：

- `PP-OCRv6_small_det.mnn`（~5MB）— 文字检测
- `PP-OCRv6_small_rec.mnn`（~10MB）— 文字识别
- `ppocr_keys_v6_small.txt` — 字符集

来源：https://github.com/zibo-chen/rust-paddle-ocr
