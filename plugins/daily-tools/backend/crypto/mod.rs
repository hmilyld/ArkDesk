//! 加解密工具后端。
//!
//! 命令薄函数定义在 `backend/mod.rs`（构建期只扫描该文件），本目录承载算法实现与
//! 测试向量。前端传入的字符串可用 `utf8` / `hex` / `base64` 描述其字节含义。

pub mod asymmetric;
pub mod dto;
pub mod encoding;
pub mod fileio;
pub mod hash;
pub mod kdf;
pub mod misc;
pub mod passphrase;
pub mod progress;
pub mod runner;
pub mod symmetric;
pub mod text_encoding;

use crate::error::AppError;

/// 归一化算法/编码标识：大小写不敏感，忽略 `-`、`_` 与空格。
pub fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_', ' '], "")
}

/// 将前端字符串按指定格式还原为字节。
pub fn parse_bytes(data: &str, format: &str) -> Result<Vec<u8>, AppError> {
    match normalize(format).as_str() {
        "utf8" | "text" | "string" => Ok(data.as_bytes().to_vec()),
        "hex" => data_encoding::HEXLOWER_PERMISSIVE
            .decode(data.trim().as_bytes())
            .map_err(|_| {
                AppError::invalid_input(
                    "十六进制解码失败：当前所选格式为 Hex，内容只能包含 0-9、a-f；\
                     若要处理中文等普通文本，请把格式改为「文本 (UTF-8)」",
                )
            }),
        "base64" => decode_base64(data).map_err(|_| {
            AppError::invalid_input(
                "Base64 解码失败：当前所选格式为 Base64，内容需为合法 Base64；\
                 若要处理中文等普通文本，请把格式改为「文本 (UTF-8)」",
            )
        }),
        other => Err(AppError::invalid_input(format!("未知输入格式: {other}"))),
    }
}

/// 将字节按指定格式转为可展示字符串。
pub fn format_bytes(bytes: &[u8], format: &str) -> Result<String, AppError> {
    match normalize(format).as_str() {
        "utf8" | "text" | "string" => String::from_utf8(bytes.to_vec())
            .map_err(|_| AppError::invalid_input("结果不是合法 UTF-8，请改用 hex 或 base64 输出")),
        "hex" => Ok(data_encoding::HEXLOWER.encode(bytes)),
        "base64" => Ok(data_encoding::BASE64.encode(bytes)),
        other => Err(AppError::invalid_input(format!("未知输出格式: {other}"))),
    }
}

fn decode_base64(data: &str) -> Result<Vec<u8>, AppError> {
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    encoding::try_decode(
        &[
            &data_encoding::BASE64,
            &data_encoding::BASE64_NOPAD,
            &data_encoding::BASE64URL,
            &data_encoding::BASE64URL_NOPAD,
        ],
        &cleaned,
        "Base64",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_format_error_is_helpful() {
        let err = parse_bytes("啊但是发发", "hex").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Hex"), "{message}");
        assert!(message.contains("UTF-8"), "{message}");
    }

    #[test]
    fn utf8_and_hex_parse() {
        assert_eq!(parse_bytes("AB", "hex").unwrap(), vec![0xAB]);
        assert_eq!(parse_bytes("你好", "utf8").unwrap(), "你好".as_bytes());
    }
}
