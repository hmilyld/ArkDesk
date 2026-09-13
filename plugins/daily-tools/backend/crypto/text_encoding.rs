//! 文本编码转换（UTF-8 / GBK / GB18030 / Big5 / Shift_JIS …），基于 encoding_rs。

use crate::error::AppError;
use encoding_rs::Encoding;

/// 将 `bytes` 按 `from` 解码、再按 `to` 编码。
///
/// `lossy = false` 时，遇到非法源字节或无法映射的目标字符会报错；
/// `lossy = true` 时用替换字符（`U+FFFD`）容错。
pub fn convert(bytes: &[u8], from: &str, to: &str, lossy: bool) -> Result<Vec<u8>, AppError> {
    let from_enc = resolve(from)?;
    let to_enc = resolve(to)?;

    let (decoded, _, had_errors) = from_enc.decode(bytes);
    if had_errors && !lossy {
        return Err(AppError::invalid_input(format!(
            "输入不是合法的 {} 文本（可勾选“忽略错误”以替换非法字节）",
            from_enc.name()
        )));
    }

    let (encoded, _, had_errors) = to_enc.encode(&decoded);
    if had_errors && !lossy {
        return Err(AppError::invalid_input(format!(
            "存在无法映射到 {} 的字符",
            to_enc.name()
        )));
    }
    Ok(encoded.into_owned())
}

fn resolve(label: &str) -> Result<&'static Encoding, AppError> {
    Encoding::for_label(label.trim().as_bytes())
        .ok_or_else(|| AppError::invalid_input(format!("不支持的字符编码: {label}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_to_gbk_and_back() {
        let gbk = convert("你好".as_bytes(), "utf-8", "gbk", false).unwrap();
        assert_eq!(data_encoding::HEXLOWER.encode(&gbk), "c4e3bac3");

        let utf8 = convert(&gbk, "gbk", "utf-8", false).unwrap();
        assert_eq!(String::from_utf8(utf8).unwrap(), "你好");
    }

    #[test]
    fn invalid_input_reports_error_unless_lossy() {
        assert!(convert(&[0xff, 0xff], "gbk", "utf-8", false).is_err());
        assert!(convert(&[0xff, 0xff], "gbk", "utf-8", true).is_ok());
        assert!(resolve("not-a-charset").is_err());
    }
}
