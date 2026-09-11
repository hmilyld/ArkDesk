# daily-tools 常用工具插件

## 功能

- **文件转换**：将 docx/xlsx/html/csv/pdf 等格式转换为 Markdown 源码，支持导出 .md 文件
- **图片 OCR**：识别图片中的文字（支持粘贴、点击选择），输出纯文本

## 依赖

| crate     | 版本 | 用途                          | 备注              |
| --------- | ---- | ----------------------------- | ----------------- |
| `anytomd` | 1    | docx/xlsx/html/csv → Markdown | 纯 Rust           |
| `lopdf`   | 0.44 | PDF 文本提取                  | 纯 Rust           |
| `ocr-rs`  | 2.4  | PaddleOCR 图片 OCR            | 需要 MNN C++ 引擎 |
| `image`   | 0.25 | 图片加载                      | ocr-rs 依赖       |

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
