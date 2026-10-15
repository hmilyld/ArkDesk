# daily-tools Common Tools Plugin

## Features

- **File conversion**: convert docx/xlsx/html/csv/pdf and other formats into Markdown source, with support for exporting .md files
- **Image OCR**: recognize text in images (paste or click to select), output as plain text
- **Encryption/decryption**: a single page with multiple tabs, covering digest/encoding/symmetric/asymmetric/key derivation/misc; everything is implemented in the Rust backend
- **JSON table**: convert between JSON and Markdown / CSV / Excel tables (keys become headers), see below

## JSON Table Tool

Frontend `frontend/views/JsonTable.vue` + `frontend/lib/table/*` (the conversion core, pure logic and unit-testable);
the backend is only responsible for reading and writing xlsx: `backend/xlsx/*`, with thin command functions in `backend/mod.rs`.

Unified intermediate representation `Table { name, columns, rows }`, cell `Cell { t: s|n|b|d|e, v, n?, f? }`
(string/number/boolean/date/Excel error value); each of the four formats implements its own "parse" and "serialize", and any two can be converted to each other.

| Direction    | Description                                                                                                                                                                                                           |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JSON → Table | When the root is an array, use it directly; otherwise specify the array via a **data path** (`data.list`, `$.a.b`, `items[0].rows`, `list[*]`); on parse failure it lists candidate array paths for one-click fill-in |
| Table → JSON | Cells are restored by type (number/boolean/empty); dates are output per the selected mode; multiple tables output `{ tableName: [...] }`                                                                              |
| Markdown     | GFM pipe table; `\|` is escaped and in-cell newlines are written as `<br>` (can be turned off, and restored in reverse); inline markup is preserved as-is                                                             |
| CSV          | UTF-8, RFC 4180, CRLF; by default export prefixes strings starting with `= + - @` with `'` to prevent formula injection                                                                                               |
| Excel        | Single-sheet read (optional worksheet / first-row header / merged-cell fill / trimming empty rows and columns), multiple tables write to multiple sheets; bold headers + frozen first row + auto column width         |

Conventions and limits:

- A column = the union of object keys (in first-appearance order); nested objects/arrays are `JSON.stringify`-ed into the cell as a whole.
- **Type inference is off by default**: cells parsed from Markdown / CSV are always strings; when enabled, numbers and booleans are recognized by conservative rules
  (rejects leading zeros, `NaN`/`Infinity`/`0x`/scientific notation, and keeps integers with more than 15 digits as text).
- Three date choices: ISO 8601 (default) / Excel serial number / custom format (`yyyy-MM-dd HH:mm:ss`).
  The date format only affects text output; the Excel target always writes native date cells.
- Previews are truncated at 200 rows; copy and export are always in full; xlsx is limited to 200,000 rows / 2048 columns per table.
- Integers above 2^53 have already lost precision at `JSON.parse`; the UI does not handle this further.
- The anti-injection `'` prefix only affects CSV text (Excel does not execute formulas when opening); turn the option off when lossless round-tripping is needed.
- `calamine` only treats a cell as a date when it carries a date format; a serial number without a format is read as a number.
- Text file reads and writes reuse the framework's `file_read_text` / `file_write_text`; dropping files onto the window is supported.

## Encryption/Decryption Tool

Frontend `frontend/views/CryptoTool.vue` + `frontend/components/crypto/*`; backend
`backend/crypto/*`, with thin command functions in `backend/mod.rs` (the `daily_tools_*` prefix).

| Tab            | Capability                                                                                                                                                           |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Digest         | MD5 / SHA-1/2 / SHA-3 / SM3 / CRC32, HMAC; text and file (streaming)                                                                                                 |
| Encoding       | Base64 (standard/URL-safe/no padding), Base32, Hex, URL; text charset conversion (GBK/GB18030/Big5…); file streaming encoding                                        |
| Symmetric      | AES-128/192/256, SM4, DES, 3DES; ECB/CBC/CTR, AES-GCM; raw key or OpenSSL `enc` passphrase mode (`Salted__` + EVP_BytesToKey or `-pbkdf2`); file streaming (non-GCM) |
| Asymmetric     | RSA (PKCS#8/SPKI and PKCS#1, OAEP/PKCS#1v15, PSS/PKCS#1v15); SM2 (ASN.1 DER and raw C1C3C2, sign/verify)                                                             |
| Key derivation | PBKDF2-HMAC-SHA256 / scrypt / Argon2id                                                                                                                               |
| Misc           | UUID v4/v7, timestamp conversion, Unicode escape/unescape                                                                                                            |

Conventions: keys are used only in memory, never written to disk and never logged; DES/3DES/MD5/SHA-1 are marked as insecure (compatibility only).
SM2 is based on `gmcrypto-core` (RustCrypto `sm2` does not support public-key encryption/decryption); see [`docs/en/local.md`](../../docs/en/local.md) for the dependency list.

## Dependencies

| crate     | Version | Purpose                       | Notes                       |
| --------- | ------- | ----------------------------- | --------------------------- |
| `anytomd` | 1       | docx/xlsx/html/csv → Markdown | Pure Rust                   |
| `lopdf`   | 0.44    | PDF text extraction           | Pure Rust                   |
| `ocr-rs`  | 2.4     | PaddleOCR image OCR           | Requires the MNN C++ engine |
| `image`   | 0.25    | Image loading                 | ocr-rs dependency           |

The JSON table tool reuses dependencies the framework already has: `calamine` (read xlsx), `rust_xlsxwriter` (write xlsx),
and the frontend reuses `marked` (GFM table parsing); no new dependencies were added.

For the encryption/decryption-related RustCrypto / `gmcrypto-core` dependencies see [`docs/en/local.md`](../../docs/en/local.md) (the fork local layer).

## Build Notes

### macOS

`ocr-rs` depends on the MNN C++ inference engine, which needs C++ standard library headers at build time. Newer macOS Xcode Command Line Tools moved the headers into an SDK subdirectory, so you need to set in `~/.zshrc`:

```bash
export CXXFLAGS="-std=c++14 -I$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1"
```

Already written to the project `~/.zshrc`, so new terminals pick it up automatically.

### Windows

- **MSVC target**: no special requirements; MSVC ships its own C++ standard library
- **GNU target**: the C++ runtime is already statically linked via the `static-cpp-runtime` feature, so no extra DLLs are needed

## OCR Models

Model files live in `src-tauri/resources/ocr-models/` and ship with the app:

- `PP-OCRv6_small_det.mnn` (~5MB) — text detection
- `PP-OCRv6_small_rec.mnn` (~10MB) — text recognition
- `ppocr_keys_v6_small.txt` — character set

Source: https://github.com/zibo-chen/rust-paddle-ocr
