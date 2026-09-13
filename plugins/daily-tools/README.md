# daily-tools 常用工具插件

## 功能

- **文件转换**：将 docx/xlsx/html/csv/pdf 等格式转换为 Markdown 源码，支持导出 .md 文件
- **图片 OCR**：识别图片中的文字（支持粘贴、点击选择），输出纯文本
- **加解密**：单页多标签，覆盖摘要/编码/对称/非对称/口令派生/杂项；全部在 Rust 后端实现

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
