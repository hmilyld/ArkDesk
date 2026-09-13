//! 编码/解码：Base64（标准/URL-safe）、Base32、Hex、URL percent。

use crate::error::AppError;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// RFC 3986 unreserved（`A-Za-z0-9-._~`）不编码，其余百分号编码。
const URL_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// 字节 → 文本。
pub fn encode(
    bytes: &[u8],
    scheme: &str,
    padding: bool,
    uppercase: bool,
) -> Result<String, AppError> {
    match super::normalize(scheme).as_str() {
        "base64" => Ok(if padding {
            data_encoding::BASE64.encode(bytes)
        } else {
            data_encoding::BASE64_NOPAD.encode(bytes)
        }),
        "base64url" => Ok(if padding {
            data_encoding::BASE64URL.encode(bytes)
        } else {
            data_encoding::BASE64URL_NOPAD.encode(bytes)
        }),
        "base32" => {
            let text = if padding {
                data_encoding::BASE32.encode(bytes)
            } else {
                data_encoding::BASE32_NOPAD.encode(bytes)
            };
            Ok(if uppercase {
                text
            } else {
                text.to_ascii_lowercase()
            })
        }
        "hex" => {
            let text = data_encoding::HEXLOWER.encode(bytes);
            Ok(if uppercase {
                text.to_ascii_uppercase()
            } else {
                text
            })
        }
        "url" | "urlencode" | "percent" => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| AppError::invalid_input("URL 编码仅支持文本（UTF-8）输入"))?;
            Ok(utf8_percent_encode(text, URL_SET).to_string())
        }
        other => Err(AppError::invalid_input(format!("不支持的编码方式: {other}"))),
    }
}

/// 文本 → 字节。
pub fn decode(data: &str, scheme: &str) -> Result<Vec<u8>, AppError> {
    match super::normalize(scheme).as_str() {
        "base64" => try_decode(
            &[
                &data_encoding::BASE64,
                &data_encoding::BASE64_NOPAD,
                &data_encoding::BASE64URL,
                &data_encoding::BASE64URL_NOPAD,
            ],
            data,
            "Base64",
        ),
        "base64url" => try_decode(
            &[
                &data_encoding::BASE64URL,
                &data_encoding::BASE64URL_NOPAD,
                &data_encoding::BASE64,
                &data_encoding::BASE64_NOPAD,
            ],
            data,
            "Base64URL",
        ),
        "base32" => {
            let upper = data.trim().to_ascii_uppercase();
            try_decode(
                &[&data_encoding::BASE32, &data_encoding::BASE32_NOPAD],
                &upper,
                "Base32",
            )
        }
        "hex" => data_encoding::HEXLOWER_PERMISSIVE
            .decode(data.trim().as_bytes())
            .map_err(|e| AppError::invalid_input(format!("十六进制解码失败: {e}"))),
        "url" | "urlencode" | "percent" => Ok(percent_decode_str(data).collect()),
        other => Err(AppError::invalid_input(format!("不支持的编码方式: {other}"))),
    }
}

/// 依次尝试多个引擎解码（用于自动兼容有无 padding / URL-safe 变体）。
pub fn try_decode(
    engines: &[&data_encoding::Encoding],
    data: &str,
    label: &str,
) -> Result<Vec<u8>, AppError> {
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    let raw = cleaned.as_bytes();
    for engine in engines {
        if let Ok(bytes) = engine.decode(raw) {
            return Ok(bytes);
        }
    }
    Err(AppError::invalid_input(format!("{label} 解码失败")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_and_variants() {
        assert_eq!(encode(b"hello", "base64", true, false).unwrap(), "aGVsbG8=");
        assert_eq!(encode(b"hello", "base64", false, false).unwrap(), "aGVsbG8");
        assert_eq!(decode("aGVsbG8=", "base64").unwrap(), b"hello");
        assert_eq!(decode("aGVsbG8", "base64").unwrap(), b"hello");
        assert_eq!(encode(b"\xfb\xff", "base64url", true, false).unwrap(), "-_8=");
        assert_eq!(decode("-_8=", "base64url").unwrap(), vec![0xfb, 0xff]);
    }

    #[test]
    fn base32_rfc4648() {
        assert_eq!(
            encode(b"foobar", "base32", true, true).unwrap(),
            "MZXW6YTBOI======"
        );
        assert_eq!(decode("MZXW6YTBOI======", "base32").unwrap(), b"foobar");
    }

    #[test]
    fn hex_and_url() {
        assert_eq!(encode(&[0xde, 0xad], "hex", true, true).unwrap(), "DEAD");
        assert_eq!(decode("dead", "hex").unwrap(), vec![0xde, 0xad]);
        assert_eq!(encode(b"a b/c?", "url", true, false).unwrap(), "a%20b%2Fc%3F");
        assert_eq!(decode("a%20b%2Fc%3F", "url").unwrap(), b"a b/c?");
    }
}
