//! daily-tools 常用工具插件：文件转换（anytomd + lopdf）、图片 OCR（PaddleOCR）、
//! 加解密与 JSON 表格（xlsx 读写）。

use crate::error::{code, AppError};
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;
use tauri::Manager;

pub mod crypto;
pub mod xlsx;

use self::crypto::dto::{
    AsymRequest, KdfRequest, KeyPair, SymmetricFileRequest, SymmetricKey, SymmetricRequest,
};
use self::crypto::runner::{build_sym_params, file_size, parse_operation, run_file_task, SymParamInput};

// ── 返回类型 ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConvertResult {
    pub markdown: String,
}

#[derive(Serialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
}

// ── OCR 引擎单例 ─────────────────────────────────────────────────

static OCR_ENGINE: OnceLock<ocr_rs::OcrEngine> = OnceLock::new();

fn get_or_init_ocr_engine(app: &tauri::AppHandle) -> Result<&'static ocr_rs::OcrEngine, AppError> {
    if let Some(engine) = OCR_ENGINE.get() {
        return Ok(engine);
    }

    // dev 模式下资源在 src-tauri/local-resources/，prod 模式下在 resource_dir()
    let resource_dir = if cfg!(debug_assertions) {
        let mut path = std::env::current_dir()
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("获取当前目录失败: {e}")))?;
        // tauri dev 的工作目录是 src-tauri/，只需拼 local-resources/
        path.push("local-resources");
        path
    } else {
        let base = app
            .path()
            .resource_dir()
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("获取资源目录失败: {e}")))?;
        // 打包后保留 local-resources/ 前缀；若被扁平化则退回 base
        let nested = base.join("local-resources");
        if nested.join("ocr-models").is_dir() {
            nested
        } else {
            base
        }
    };

    let det = resource_dir.join("ocr-models/PP-OCRv6_small_det.mnn");
    let rec = resource_dir.join("ocr-models/PP-OCRv6_small_rec.mnn");
    let charset = resource_dir.join("ocr-models/ppocr_keys_v6_small.txt");

    log::debug!("OCR 资源目录: {}", resource_dir.display());
    log::debug!("OCR 模型: det={}, exists={}", det.display(), det.exists());
    log::debug!("OCR 模型: rec={}, exists={}", rec.display(), rec.exists());
    log::debug!(
        "OCR 模型: charset={}, exists={}",
        charset.display(),
        charset.exists()
    );

    let engine = ocr_rs::OcrEngine::new(
        det.to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "det 模型路径包含非法字符"))?,
        rec.to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "rec 模型路径包含非法字符"))?,
        charset
            .to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "charset 路径包含非法字符"))?,
        None,
    )
    .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("OCR 引擎初始化失败: {e}")))?;

    // 尝试设置值（并发场景下可能已被其他线程设置）
    let _ = OCR_ENGINE.set(engine);
    Ok(OCR_ENGINE.get().unwrap())
}

// ── 文件转换 ──────────────────────────────────────────────────────

/// PDF 文本提取（lopdf）
fn convert_pdf(path: &str) -> Result<String, AppError> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("PDF 加载失败: {e}")))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        return Ok(String::new());
    }

    let page_numbers: Vec<u32> = pages.keys().copied().collect();
    let text = doc
        .extract_text(&page_numbers)
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("PDF 文本提取失败: {e}")))?;

    Ok(text)
}

/// 文件转换命令：根据扩展名分发到 anytomd 或 lopdf
#[tauri::command]
pub async fn daily_tools_convert_file(path: String) -> Result<ConvertResult, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("文件路径不能为空"));
    }

    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let markdown = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        match ext.as_str() {
            "pdf" => convert_pdf(&path),
            _ => {
                let result = anytomd::convert_file(&path, &anytomd::ConversionOptions::default())
                    .map_err(|e| {
                    AppError::custom(code::PLUGIN_ERROR, format!("文件转换失败: {e}"))
                })?;
                Ok(result.markdown)
            }
        }
    })
    .await
    .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("任务执行失败: {e}")))??;

    Ok(ConvertResult { markdown })
}

// ── 图片 OCR ──────────────────────────────────────────────────────

/// 图片 OCR 识别命令：加载图片 → PaddleOCR → 返回纯文本
#[tauri::command]
pub async fn daily_tools_ocr_image(
    path: String,
    app: tauri::AppHandle,
) -> Result<OcrResult, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("图片路径不能为空"));
    }

    let engine = get_or_init_ocr_engine(&app)?;

    let results =
        tokio::task::spawn_blocking(move || -> Result<Vec<ocr_rs::OcrResult_>, AppError> {
            let img = image::open(&path)
                .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("图片加载失败: {e}")))?;
            engine
                .recognize(&img)
                .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("OCR 识别失败: {e}")))
        })
        .await
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("任务执行失败: {e}")))??;

    if results.is_empty() {
        return Ok(OcrResult {
            text: String::new(),
            confidence: 0.0,
        });
    }

    let text: String = results
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let confidence = results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32;

    Ok(OcrResult { text, confidence })
}

/// 保存 Markdown 文件
#[tauri::command]
pub async fn daily_tools_save_markdown(path: String, content: String) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("保存路径不能为空"));
    }
    std::fs::write(&path, content)?;
    log::info!("Markdown 已保存: {path}");
    Ok(())
}

// ── JSON 表格（xlsx 读写） ────────────────────────────────────────

/// 列出 .xlsx 工作簿中的工作表名
#[tauri::command]
pub fn daily_tools_xlsx_list_sheets(path: String) -> Result<Vec<String>, AppError> {
    xlsx::list_sheets(&path)
}

/// 读取单个工作表为表格（列 + 类型化单元格）
#[tauri::command]
pub fn daily_tools_xlsx_read(
    path: String,
    sheet: Option<String>,
    has_header: Option<bool>,
    fill_merged: Option<bool>,
    trim_empty: Option<bool>,
) -> Result<xlsx::SheetData, AppError> {
    xlsx::read_sheet(
        &path,
        sheet.as_deref(),
        has_header.unwrap_or(true),
        fill_merged.unwrap_or(false),
        trim_empty.unwrap_or(true),
    )
}

/// 写出工作簿（每个表一个工作表）
#[tauri::command]
pub fn daily_tools_xlsx_write(
    path: String,
    sheets: Vec<xlsx::SheetData>,
    style: Option<bool>,
) -> Result<(), AppError> {
    xlsx::write_workbook(&path, &sheets, style.unwrap_or(true))
}

// ── 加解密（P0：哈希 / 编码 / 文本编码） ────────────────────────────

/// 文本哈希：`input_encoding` 描述 `data` 的字节含义（utf8/hex/base64），
/// `output` 为 hex/base64。算法支持 md5/sha1/sha224/sha256/sha384/sha512/
/// sha3-224/sha3-256/sha3-384/sha3-512/sm3/crc32。
#[tauri::command]
pub fn daily_tools_hash(
    data: String,
    algorithm: String,
    input_encoding: Option<String>,
    output: Option<String>,
) -> Result<String, AppError> {
    let bytes = crypto::parse_bytes(&data, input_encoding.as_deref().unwrap_or("utf8"))?;
    let digest = crypto::hash::digest(&algorithm, &bytes)?;
    crypto::format_bytes(&digest, output.as_deref().unwrap_or("hex"))
}

/// HMAC：`key_encoding` 描述密钥字节含义。
#[tauri::command]
pub fn daily_tools_hmac(
    data: String,
    algorithm: String,
    key: String,
    key_encoding: Option<String>,
    input_encoding: Option<String>,
    output: Option<String>,
) -> Result<String, AppError> {
    let bytes = crypto::parse_bytes(&data, input_encoding.as_deref().unwrap_or("utf8"))?;
    let key_bytes = crypto::parse_bytes(&key, key_encoding.as_deref().unwrap_or("utf8"))?;
    let mac = crypto::hash::hmac(&algorithm, &key_bytes, &bytes)?;
    crypto::format_bytes(&mac, output.as_deref().unwrap_or("hex"))
}

/// 编码：scheme = base64 / base64url / base32 / hex / url。
#[tauri::command]
pub fn daily_tools_encode(
    data: String,
    scheme: String,
    input_encoding: Option<String>,
    padding: Option<bool>,
    uppercase: Option<bool>,
) -> Result<String, AppError> {
    let bytes = crypto::parse_bytes(&data, input_encoding.as_deref().unwrap_or("utf8"))?;
    crypto::encoding::encode(
        &bytes,
        &scheme,
        padding.unwrap_or(true),
        uppercase.unwrap_or(false),
    )
}

/// 解码：`output` 描述结果字节如何展示（utf8/hex/base64）。
#[tauri::command]
pub fn daily_tools_decode(
    data: String,
    scheme: String,
    output: Option<String>,
) -> Result<String, AppError> {
    let bytes = crypto::encoding::decode(&data, &scheme)?;
    crypto::format_bytes(&bytes, output.as_deref().unwrap_or("utf8"))
}

/// 文本编码转换：`from`/`to` 为字符集标签（utf-8 / gbk / gb18030 / big5 …）。
#[tauri::command]
pub fn daily_tools_text_convert(
    data: String,
    from: String,
    to: String,
    input_encoding: Option<String>,
    output: Option<String>,
    lossy: Option<bool>,
) -> Result<String, AppError> {
    let bytes = crypto::parse_bytes(&data, input_encoding.as_deref().unwrap_or("utf8"))?;
    let converted = crypto::text_encoding::convert(&bytes, &from, &to, lossy.unwrap_or(false))?;
    crypto::format_bytes(&converted, output.as_deref().unwrap_or("utf8"))
}

/// 文件哈希（流式，带进度与取消）。
#[tauri::command]
pub async fn daily_tools_hash_file(
    app: tauri::AppHandle,
    task_id: Option<String>,
    path: String,
    algorithm: String,
    output: Option<String>,
) -> Result<String, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("文件路径不能为空"));
    }
    let total = file_size(&path);
    let output = output.unwrap_or_else(|| "hex".to_string());
    run_file_task(app, task_id, total, move |observer| {
        let digest = crypto::fileio::hash_file(&path, &algorithm, observer)?;
        crypto::format_bytes(&digest, &output)
    })
    .await
}

/// 文件编码（流式，带进度与取消）。
///
/// 提供 `output_path` 时写入文件并返回空串，否则返回编码文本。
#[tauri::command]
pub async fn daily_tools_encode_file(
    app: tauri::AppHandle,
    task_id: Option<String>,
    path: String,
    scheme: String,
    padding: Option<bool>,
    uppercase: Option<bool>,
    output_path: Option<String>,
) -> Result<String, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("文件路径不能为空"));
    }
    let total = file_size(&path);
    run_file_task(app, task_id, total, move |observer| {
        let padding = padding.unwrap_or(true);
        let uppercase = uppercase.unwrap_or(false);
        match output_path {
            Some(output) if !output.trim().is_empty() => {
                crypto::fileio::encode_file_to(
                    &path, &scheme, padding, uppercase, &output, observer,
                )?;
                Ok(String::new())
            }
            _ => crypto::fileio::encode_file(&path, &scheme, padding, uppercase, observer),
        }
    })
    .await
}

// ── 加解密（P1：对称） ────────────────────────────────────────────

/// 对称加解密（文本，一次性）。GCM 密文尾部附 16 字节 tag。
#[tauri::command]
pub fn daily_tools_symmetric(req: SymmetricRequest) -> Result<String, AppError> {
    let SymmetricRequest {
        operation,
        algorithm,
        mode,
        data,
        data_encoding,
        output,
        key,
        key_encoding,
        iv,
        iv_encoding,
        passphrase,
        kdf,
        salt,
        salt_encoding,
        rounds,
        aad,
        aad_encoding,
    } = req;

    let enc = parse_operation(&operation)?;
    let data_bytes = crypto::parse_bytes(&data, data_encoding.as_deref().unwrap_or("utf8"))?;
    let params = build_sym_params(SymParamInput {
        algorithm,
        mode,
        key,
        key_encoding,
        iv,
        iv_encoding,
        passphrase,
        kdf,
        salt,
        salt_encoding,
        rounds,
        aad,
        aad_encoding,
    })?;
    let result = crypto::symmetric::text_crypt(enc, &params, &data_bytes)?;
    crypto::format_bytes(&result, output.as_deref().unwrap_or("hex"))
}

/// 对称加解密（文件，流式；仅 ECB/CBC/CTR）。带进度与取消。
#[tauri::command]
pub async fn daily_tools_symmetric_file(
    app: tauri::AppHandle,
    task_id: Option<String>,
    req: SymmetricFileRequest,
) -> Result<(), AppError> {
    let SymmetricFileRequest {
        operation,
        algorithm,
        mode,
        input_path,
        output_path,
        key,
        key_encoding,
        iv,
        iv_encoding,
        passphrase,
        kdf,
        salt,
        salt_encoding,
        rounds,
    } = req;

    if input_path.trim().is_empty() || output_path.trim().is_empty() {
        return Err(AppError::invalid_input("输入/输出路径不能为空"));
    }
    let enc = parse_operation(&operation)?;
    let params = build_sym_params(SymParamInput {
        algorithm,
        mode,
        key,
        key_encoding,
        iv,
        iv_encoding,
        passphrase,
        kdf,
        salt,
        salt_encoding,
        rounds,
        aad: None,
        aad_encoding: None,
    })?;
    let total = file_size(&input_path);

    run_file_task(app, task_id, total, move |observer| {
        crypto::symmetric::file_crypt(enc, &params, &input_path, &output_path, observer)
    })
    .await
}

/// 生成随机密钥与 IV（GCM 模式为 12 字节 nonce）。
#[tauri::command]
pub fn daily_tools_symmetric_generate(
    algorithm: String,
    encoding: Option<String>,
    mode: Option<String>,
) -> Result<SymmetricKey, AppError> {
    let (key, iv) = crypto::symmetric::generate(&algorithm, mode.as_deref().unwrap_or("cbc"))?;
    let encoding = encoding.as_deref().unwrap_or("hex");
    Ok(SymmetricKey {
        key: crypto::format_bytes(&key, encoding)?,
        iv: crypto::format_bytes(&iv, encoding)?,
    })
}

// ── 加解密（P2：非对称 / KDF / 杂项） ────────────────────────────

/// RSA 生成密钥对。
#[tauri::command]
pub fn daily_tools_rsa_generate(bits: Option<usize>, format: Option<String>) -> Result<KeyPair, AppError> {
    let (private_key, public_key) =
        crypto::asymmetric::rsa_generate(bits.unwrap_or(2048), format.as_deref().unwrap_or("pkcs8"))?;
    Ok(KeyPair {
        private_key,
        public_key,
    })
}

/// RSA 加密。
#[tauri::command]
pub fn daily_tools_rsa_encrypt(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let out = crypto::asymmetric::rsa_encrypt(
        &req.key,
        &data,
        req.padding.as_deref().unwrap_or("oaep"),
        req.hash.as_deref().unwrap_or("sha256"),
    )?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("hex"))
}

/// RSA 解密。
#[tauri::command]
pub fn daily_tools_rsa_decrypt(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("hex"))?;
    let out = crypto::asymmetric::rsa_decrypt(
        &req.key,
        &data,
        req.padding.as_deref().unwrap_or("oaep"),
        req.hash.as_deref().unwrap_or("sha256"),
    )?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("utf8"))
}

/// RSA 签名。
#[tauri::command]
pub fn daily_tools_rsa_sign(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let out = crypto::asymmetric::rsa_sign(
        &req.key,
        &data,
        req.padding.as_deref().unwrap_or("pss"),
        req.hash.as_deref().unwrap_or("sha256"),
    )?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("hex"))
}

/// RSA 验签。
#[tauri::command]
pub fn daily_tools_rsa_verify(req: AsymRequest) -> Result<bool, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let signature = crypto::parse_bytes(
        req.signature.as_deref().unwrap_or(""),
        req.signature_encoding.as_deref().unwrap_or("hex"),
    )?;
    crypto::asymmetric::rsa_verify(
        &req.key,
        &data,
        &signature,
        req.padding.as_deref().unwrap_or("pss"),
        req.hash.as_deref().unwrap_or("sha256"),
    )
}

/// SM2 生成密钥对。
#[tauri::command]
pub fn daily_tools_sm2_generate() -> Result<KeyPair, AppError> {
    let (private_key, public_key) = crypto::asymmetric::sm2_generate()?;
    Ok(KeyPair {
        private_key,
        public_key,
    })
}

/// SM2 加密。
#[tauri::command]
pub fn daily_tools_sm2_encrypt(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let out = crypto::asymmetric::sm2_encrypt(
        &req.key,
        &data,
        req.format.as_deref().unwrap_or("asn1"),
    )?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("hex"))
}

/// SM2 解密。
#[tauri::command]
pub fn daily_tools_sm2_decrypt(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("hex"))?;
    let out = crypto::asymmetric::sm2_decrypt(
        &req.key,
        &data,
        req.format.as_deref().unwrap_or("asn1"),
    )?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("utf8"))
}

/// SM2 签名。
#[tauri::command]
pub fn daily_tools_sm2_sign(req: AsymRequest) -> Result<String, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let out = crypto::asymmetric::sm2_sign(&req.key, req.id.as_deref().unwrap_or(""), &data)?;
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("hex"))
}

/// SM2 验签。
#[tauri::command]
pub fn daily_tools_sm2_verify(req: AsymRequest) -> Result<bool, AppError> {
    let data = crypto::parse_bytes(&req.data, req.data_encoding.as_deref().unwrap_or("utf8"))?;
    let signature = crypto::parse_bytes(
        req.signature.as_deref().unwrap_or(""),
        req.signature_encoding.as_deref().unwrap_or("hex"),
    )?;
    crypto::asymmetric::sm2_verify(&req.key, req.id.as_deref().unwrap_or(""), &data, &signature)
}

/// 口令派生（PBKDF2 / scrypt / Argon2）。
#[tauri::command]
pub fn daily_tools_kdf(req: KdfRequest) -> Result<String, AppError> {
    let salt = crypto::parse_bytes(&req.salt, req.salt_encoding.as_deref().unwrap_or("hex"))?;
    let length = req.length.clamp(1, 1024);
    let password = req.password.as_bytes();
    let out = match crypto::normalize(&req.algorithm).as_str() {
        "pbkdf2" => crypto::kdf::pbkdf2(password, &salt, req.rounds.unwrap_or(10000), length),
        "scrypt" => crypto::kdf::scrypt(
            password,
            &salt,
            req.log_n.unwrap_or(15),
            req.r.unwrap_or(8),
            req.p.unwrap_or(1),
            length,
        )?,
        "argon2" | "argon2id" => crypto::kdf::argon2(
            password,
            &salt,
            req.memory_kib.unwrap_or(19456),
            req.iterations.unwrap_or(2),
            req.parallelism.unwrap_or(1),
            length,
        )?,
        other => return Err(AppError::invalid_input(format!("不支持的 KDF: {other}"))),
    };
    crypto::format_bytes(&out, req.output.as_deref().unwrap_or("hex"))
}

/// 生成 UUID。
#[tauri::command]
pub fn daily_tools_uuid(version: Option<u32>, count: Option<usize>) -> Result<Vec<String>, AppError> {
    crypto::misc::uuid(version.unwrap_or(4), count.unwrap_or(1))
}

/// 时间戳转换。
#[tauri::command]
pub fn daily_tools_timestamp(input: String, mode: Option<String>) -> Result<String, AppError> {
    crypto::misc::timestamp(&input, mode.as_deref().unwrap_or("auto"))
}

/// Unicode 转义。
#[tauri::command]
pub fn daily_tools_unicode_escape(input: String, style: Option<String>) -> Result<String, AppError> {
    Ok(crypto::misc::unicode_escape(
        &input,
        style.as_deref().unwrap_or("u"),
    ))
}

/// Unicode 反转义。
#[tauri::command]
pub fn daily_tools_unicode_unescape(input: String) -> Result<String, AppError> {
    crypto::misc::unicode_unescape(&input)
}

#[cfg(test)]
mod crypto_contract_tests {
    use super::crypto::dto::{AsymRequest, KdfRequest, KeyPair, SymmetricRequest};

    #[test]
    fn key_pair_serializes_camel_case() {
        let kp = KeyPair {
            private_key: "a".into(),
            public_key: "b".into(),
        };
        let value = serde_json::to_value(&kp).unwrap();
        assert!(value.get("privateKey").is_some(), "缺 privateKey: {value}");
        assert!(value.get("publicKey").is_some(), "缺 publicKey: {value}");
    }

    #[test]
    fn symmetric_request_accepts_camel_case() {
        let json = serde_json::json!({
            "operation": "encrypt",
            "algorithm": "aes-256",
            "mode": "cbc",
            "data": "hi",
            "dataEncoding": "utf8",
            "output": "hex",
            "keyEncoding": "hex",
            "ivEncoding": "hex",
            "kdf": "evp",
            "saltEncoding": "hex",
            "rounds": 0
        });
        let req: SymmetricRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.data_encoding.as_deref(), Some("utf8"));
        assert_eq!(req.key_encoding.as_deref(), Some("hex"));
    }

    #[test]
    fn asymmetric_request_accepts_camel_case() {
        let json = serde_json::json!({
            "key": "pem",
            "data": "x",
            "dataEncoding": "utf8",
            "output": "hex",
            "signatureEncoding": "hex"
        });
        let req: AsymRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.data_encoding.as_deref(), Some("utf8"));
        assert_eq!(req.signature_encoding.as_deref(), Some("hex"));
    }

    #[test]
    fn asymmetric_request_tolerates_missing_key() {
        // 缺失 key 不应触发 Tauri 原始反序列化错误，交由业务层友好提示
        let json = serde_json::json!({ "data": "x" });
        let req: AsymRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.key, "");
    }

    #[test]
    fn kdf_request_accepts_camel_case() {
        let json = serde_json::json!({
            "password": "p",
            "salt": "s",
            "saltEncoding": "utf8",
            "algorithm": "pbkdf2",
            "length": 32,
            "logN": 15,
            "memoryKib": 1024
        });
        let req: KdfRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.log_n, Some(15));
        assert_eq!(req.memory_kib, Some(1024));
    }
}
