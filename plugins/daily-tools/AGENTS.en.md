# daily-tools Plugin — Development Conventions (AGENTS)

> Local conventions for AI/developers. For feature documentation see `README.en.md` in the same directory; project-level docs are referenced only.

## Tools

`plugin.json` declares four tools (same plugin, frontend and backend colocated):

- `file-converter` → `frontend/views/FileConverter.vue`
- `image-ocr` → `frontend/views/ImageOcr.vue`
- `crypto` → `frontend/views/CryptoTool.vue` (encryption/decryption, see below)
- `json-table` → `frontend/views/JsonTable.vue` (JSON table, see below)

## JSON Table (json-table)

The unified intermediate representation `Table` / `Cell` is defined in `frontend/lib/table/model.ts`: each of the
four formats implements its own parsing and serialization, and any two can be converted to each other. **All text-based
formats live in the frontend**; Rust only reads and writes xlsx.

| File                                         | Responsibility                                                                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `frontend/lib/table/model.ts`                | `Table`/`Cell`/`TableError`, column union and column-name normalization, row padding                                          |
| `frontend/lib/table/options.ts`              | Format enums, date mode, preview row count, limits and other constants (frontend and backend limits must be aligned manually) |
| `frontend/lib/table/path.ts`                 | Data path parsing (`$.a.b`, `items[0]`, `[*]`) and candidate array path scanning                                              |
| `frontend/lib/table/json-convert.ts`         | JSON ↔ Table (key union, nested JSON serialization, type restoration)                                                         |
| `frontend/lib/table/markdown.ts`             | Table ↔ GFM pipe table (`marked` parsing; `\|` and `<br>` escaping)                                                           |
| `frontend/lib/table/csv.ts`                  | Table ↔ CSV (RFC 4180 state machine, formula injection prevention)                                                            |
| `frontend/lib/table/cell-text.ts`            | Text ↔ cell (conservative type inference, rendering per date mode)                                                            |
| `frontend/lib/table/convert.ts`              | Unified entry points for text-source parsing / text-target serialization + preview truncation                                 |
| `frontend/lib/table/xlsx.ts`                 | Frontend-side IPC wrapper (column-name normalization is filled in after the backend reads back)                               |
| `frontend/components/table/TablePreview.vue` | Grid preview for the Excel target                                                                                             |
| `backend/xlsx/{dto,read,write}.rs`           | calamine read / rust_xlsxwriter write; thin command functions in `backend/mod.rs`                                             |
| `backend/xlsx/tests.rs`                      | Temp-file round-trip tests (types, dates, headers, trimming, injection, invalid input)                                        |

Conventions: `t` takes the values `s|n|b|d|e`; `v === null` means an empty cell; for dates `v` is ISO 8601 and `n`
is the raw serial number (used only for the "serial number" output mode). Writing xlsx always uses `write_string`/`write_number`/
`write_boolean`/`write_datetime_with_format`, and **must not use `write_formula`** (naturally injection-free).
Frontend pure-logic tests are at the repo root `tests/daily-tools-json-table.spec.ts`.

## Crypto Backend Layout

Commands **must** be written in `backend/mod.rs` (`build.rs` only scans that file), always with the `daily_tools_*`
prefix and returning `Result<T, AppError>`. Logic and tests go in submodules:

| File                      | Responsibility                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------ |
| `crypto/dto.rs`           | Request/response DTOs (camelCase, aligned with the frontend fields)                        |
| `crypto/runner.rs`        | File task progress/cancel (`run_file_task`), `build_sym_params`, `parse_operation`         |
| `crypto/progress.rs`      | `StreamObserver` abstraction + `CallbackObserver` (debounced) / `NoopObserver`             |
| `crypto/passphrase.rs`    | OpenSSL `enc` passphrase: `Salted__`, EVP_BytesToKey(MD5), PBKDF2                          |
| `crypto/hash.rs`          | MD5/SHA-1/2/3/SM3/CRC32 + HMAC, streaming `Hasher`                                         |
| `crypto/encoding.rs`      | Base64 (standard/URL-safe/padding), Base32, Hex, URL                                       |
| `crypto/text_encoding.rs` | Charset conversion (encoding_rs)                                                           |
| `crypto/fileio.rs`        | Streaming file hashing/encoding (chunking + padding alignment + cleanup of partial output) |
| `crypto/symmetric.rs`     | AES/SM4/DES/3DES, ECB/CBC/CTR + AES-GCM                                                    |
| `crypto/asymmetric.rs`    | RSA, SM2 (`gmcrypto-core`)                                                                 |
| `crypto/kdf.rs`           | PBKDF2 / scrypt / Argon2id                                                                 |
| `crypto/misc.rs`          | UUID, timestamp, Unicode escape                                                            |

Conventions: command names are globally unique (`build.rs` checks for duplicates at build time); errors are always
`AppError` (in Chinese, in plain language); keys/derived material are never written to disk and never logged, and the
derived key/iv are cleared with `zeroize`.

## Progress and Cancellation

File-based commands (`daily_tools_hash_file` / `daily_tools_encode_file` / `daily_tools_symmetric_file`)
additionally receive the framework-injected `app: AppHandle` and the frontend-passed `task_id`, going through `runner::run_file_task`:

- `crate::tasks::begin` registers a cancellation token, and `StreamObserver::advance` checks for cancellation on every chunk and reports progress back;
- The frontend's `core/tasks` `tasksState` reads status, and `cancelTask` cancels;
- On cancellation/failure the unfinished output file is deleted, and `AppError("CANCELLED", "operation cancelled")` is returned.

## Frontend Conventions

- `frontend/views/CryptoTool.vue`: a single page with multiple tabs (digest/encoding/symmetric/asymmetric/key derivation/misc).
- Shared interaction components (`frontend/components/crypto/`):
  - Output always uses `ResultBox` (panel + title bar + copy in the top-right corner);
  - Things that are both input and often a generated result (keys) use `CopyableTextarea`;
  - Copy buttons always use `CopyButton`; file task progress uses `TaskProgress` (reads `core/tasks`).
- Calls: always through `composables/useCryptoCall.ts` (`ipc` + loading/error + request sequence number to prevent out-of-order responses).
- Error messages: `frontend/shared.ts`'s `errorMessage()` (compatible with `AppError` objects, translates Tauri argument errors);
  a non-UTF-8 input format shows a yellow hint when an option other than "Text (UTF-8)" is selected.
- Option constants are centralized in `frontend/lib/crypto-shared.ts`; do not redefine them in components.
- Input fields have a fixed height + scroll on overflow (`field-sizing: fixed` overrides the shadcn Textarea auto-grow).

### Options Area (following Apple HIG's grouped form)

Options uniformly use `@/components/settings`' `SettingsSection` (group subheading + separator rows) and `SettingsRow`
(left title + grey second-line description / right control); when placed inside a Panel, pass
`class="rounded-md bg-transparent"` to `SettingsSection` to avoid a card inside a card.

- **One setting per row; the title and the control must be on the same row**: switches use `Switch size="sm"` (HIG: grouped
  forms use a mini switch, with the row height matching a button). Do not use `justify-between` to pull the Label and Switch to
  the two ends of a row — in a wide panel it just looks like "a block of text with no control" (HIG is explicit: a switch goes
  only in a list row, with the row content providing the context).
- **In-group descriptions** are written as the row's `description` (second-line small text); **group-level descriptions** go in
  `SettingsSection`'s `#footer` (grey explanatory text), not as a filled colour block and not mixed into option rows.
- Mutually exclusive selection (such as the three-way "date output") uses `Select` (i.e. the platform's pop-up button), not a
  row of switches; only consider checkbox indentation when you need nested groups or dependencies (HIG: a switch must not
  replace a checkbox).
- Each switch's title must state clearly what it controls; `hint`/descriptions only add context, they do not replace the title.

### Module Panels (Panel)

In tool pages, **every functional module (settings / input / output / result) is wrapped in `@/components/tool/Panel`**;
do not leave modules bare on the page. Panel is the framework-level single source of truth (outer frame + header bar + body);
see the root `AGENTS.en.md` for the conventions:

```vue
<Panel title="Input" :hint="formatName" body-class="space-y-4">
  <template #actions>
    <Button variant="secondary" size="sm">Select file</Button>
  </template>
  <!-- Body: defaults to space-y-3 p-4; use body-class to override for full-bleed cases -->
</Panel>
```

- props: `title` / `hint` (supplementary info after the title, such as the format name or row/column stats) / `body-class` / `header-class`;
  slots: `default` / `title` / `actions`.
- **No nested cards inside a Panel**: `ResultBox` / `CopyableTextarea` / `MarkdownPreview` / `TablePreview`
  either already use a Panel themselves or carry no outer frame (`TablePreview` is a full-bleed grid); the outer frame is always
  provided by the containing Panel.
- Action buttons go in `#actions` (`size="sm"`, icon `size-3.5` + `mr-1`); explanatory text uses
  `text-xs text-muted-foreground`; error bars use
  `rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive`.
- Panel does not preset a grid span: as a **direct child** of `grid grid-cols-12` it must write its own span in `class`
  (e.g. `col-span-12 lg:col-span-6`), otherwise it collapses to 1/12 wide; give wide content (tables / long text) `min-w-0`
  so it scrolls internally instead of bursting the grid.
- The whole plugin has been brought in line with this: `views/{FileConverter,ImageOcr,JsonTable}.vue`, `components/crypto/*` (including
  `ResultBox` / `CopyableTextarea`), `components/MarkdownPreview.vue`.

## Dependencies (fork local layer)

Crypto-related Rust dependencies (the `local plugin deps` section of `src-tauri/Cargo.toml`):

- Digest/encoding: `digest` `md-5` `sha1` `sha2` `sha3` `sm3` `hmac` `data-encoding` `percent-encoding` `encoding_rs` `crc32fast`
- Symmetric: `aes` `des` `sm4` `cbc` `ecb` `ctr` `aes-gcm` `cipher` `block-padding` `pbkdf2` `zeroize` `getrandom`
- Asymmetric/KDF/misc: `rsa` `gmcrypto-core` `rand_core` `scrypt` `argon2` `uuid`

> SM2 encryption/decryption does not use RustCrypto `sm2` (which only does SM2DSA signing); it uses `gmcrypto-core`
> (constant-time, including ASN.1/raw C1C3C2, PKCS#8/SPKI/PEM).
> `rand_core` (getrandom feature) is used for RSA's `OsRng`; `getrandom` is used for key generation and the SM2 signing RNG.

## Known Limitations

- GCM is AES-only and text-only; SM4 provides ECB/CBC/CTR (no GCM); file mode is ECB/CBC/CTR only.
- The file "encoding preview" returns the entire result as one block; for very large files use "encode and save to file".
- Asymmetric is text-only; RSA key generation and Argon2 are synchronous commands (there is already a loading state, so the webview does not freeze).
- Keys/passphrases in input fields are ordinary strings (only derived material is `zeroize`d).
- JSON table: type inference is off by default; the date format only affects text output; the xlsx read limit is 200,000 rows / 2048 columns
  (frontend and backend limits are in `frontend/lib/table/options.ts` and `backend/xlsx/read.rs` respectively; changes must be kept in sync);
  the CSV anti-injection `'` prefix changes the literal text (turn it off when lossless round-tripping is needed).

## Validation

```bash
pnpm lint && pnpm build          # frontend
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs   # Rust
```
