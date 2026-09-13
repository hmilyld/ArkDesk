//! 杂项：UUID、时间戳转换、Unicode 转义。

use crate::error::AppError;

/// 生成 UUID。`version` = 4 / 7。
pub fn uuid(version: u32, count: usize) -> Result<Vec<String>, AppError> {
    let count = count.clamp(1, 1000);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let value = match version {
            4 => uuid::Uuid::new_v4(),
            7 => uuid::Uuid::now_v7(),
            other => return Err(AppError::invalid_input(format!("不支持的 UUID 版本: {other}"))),
        };
        out.push(value.to_string());
    }
    Ok(out)
}

/// 时间戳 ↔ 日期时间。
///
/// - `mode = "auto" | "s" | "ms"`：输入数字 → 本地与 UTC 时间字符串。
/// - `mode = "to"`：输入日期字符串（`YYYY-MM-DD HH:MM:SS`）→ 秒/毫秒时间戳。
pub fn timestamp(input: &str, mode: &str) -> Result<String, AppError> {
    use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

    let input = input.trim();
    if mode.eq_ignore_ascii_case("to") {
        let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|_| AppError::invalid_input("日期格式应为 YYYY-MM-DD HH:MM:SS"))?;
        let local = Local
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| AppError::invalid_input("本地时间不明确（可能处于夏令时切换点）"))?;
        return Ok(format!(
            "秒: {}\n毫秒: {}",
            local.timestamp(),
            local.timestamp_millis()
        ));
    }

    let value: i64 = input
        .parse()
        .map_err(|_| AppError::invalid_input("时间戳应为整数"))?;
    let millis = match super::normalize(mode).as_str() {
        "s" | "sec" | "seconds" => value * 1000,
        "ms" | "millis" | "milliseconds" => value,
        _ => {
            if input.len() >= 13 {
                value
            } else {
                value * 1000
            }
        }
    };
    let utc: DateTime<Utc> = Utc
        .timestamp_millis_opt(millis)
        .single()
        .ok_or_else(|| AppError::invalid_input("时间戳超出可表示范围"))?;
    let local = utc.with_timezone(&Local);
    Ok(format!(
        "本地: {}\nUTC: {}",
        local.format("%Y-%m-%d %H:%M:%S"),
        utc.format("%Y-%m-%d %H:%M:%S")
    ))
}

/// Unicode 转义。`style = "u"` → `\uXXXX`（BMP 外使用代理对）；`style = "brace"` → `\u{XXXX}`。
pub fn unicode_escape(input: &str, style: &str) -> String {
    let brace = super::normalize(style) == "brace";
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let code = ch as u32;
        if brace {
            out.push_str(&format!("\\u{{{code:04X}}}"));
        } else if code <= 0xFFFF {
            out.push_str(&format!("\\u{code:04X}"));
        } else {
            let v = code - 0x10000;
            let high = 0xD800 + (v >> 10);
            let low = 0xDC00 + (v & 0x3FF);
            out.push_str(&format!("\\u{high:04X}\\u{low:04X}"));
        }
    }
    out
}

/// 还原 `\uXXXX`、`\u{...}`、`\xXX` 转义（支持 UTF-16 代理对）。
pub fn unicode_unescape(input: &str) -> Result<String, AppError> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'u' => {
                    if i + 2 < chars.len() && chars[i + 2] == '{' {
                        let close = chars[i + 3..]
                            .iter()
                            .position(|&c| c == '}')
                            .ok_or_else(|| AppError::invalid_input("缺少 `}`"))?
                            + i
                            + 3;
                        let hex: String = chars[i + 3..close].iter().collect();
                        out.push(parse_code(&hex)?);
                        i = close + 1;
                    } else {
                        let hi = parse_u16(&take_hex(&chars, i + 2, 4)?)?;
                        // 高代理项需与随后的低代理项组合
                        if (0xD800..=0xDBFF).contains(&hi)
                            && chars.get(i + 6) == Some(&'\\')
                            && chars.get(i + 7) == Some(&'u')
                        {
                            let lo = parse_u16(&take_hex(&chars, i + 8, 4)?)?;
                            let code =
                                0x10000 + (((hi as u32 - 0xD800) << 10) | (lo as u32 - 0xDC00));
                            out.push(
                                char::from_u32(code)
                                    .ok_or_else(|| AppError::invalid_input("非法代理对"))?,
                            );
                            i += 12;
                        } else {
                            out.push(
                                char::from_u32(hi as u32)
                                    .ok_or_else(|| AppError::invalid_input("非法代理项"))?,
                            );
                            i += 6;
                        }
                    }
                }
                'x' => {
                    let hex = take_hex(&chars, i + 2, 2)?;
                    out.push(parse_code(&hex)?);
                    i += 4;
                }
                other => {
                    out.push(other);
                    i += 2;
                }
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn take_hex(chars: &[char], start: usize, len: usize) -> Result<String, AppError> {
    chars
        .get(start..start + len)
        .map(|slice| slice.iter().collect())
        .ok_or_else(|| AppError::invalid_input("转义序列不完整"))
}

fn parse_u16(hex: &str) -> Result<u16, AppError> {
    u16::from_str_radix(hex, 16)
        .map_err(|_| AppError::invalid_input(format!("非法十六进制: {hex}")))
}

fn parse_code(hex: &str) -> Result<char, AppError> {
    let code = u32::from_str_radix(hex, 16)
        .map_err(|_| AppError::invalid_input(format!("非法 Unicode 码点: {hex}")))?;
    char::from_u32(code).ok_or_else(|| AppError::invalid_input(format!("非法 Unicode 码点: {hex}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_versions() {
        let v4 = uuid(4, 1).unwrap();
        assert_eq!(v4[0].len(), 36);
        assert_eq!(v4[0].chars().nth(14), Some('4'));
        let v7 = uuid(7, 1).unwrap();
        assert_eq!(v7[0].chars().nth(14), Some('7'));
    }

    #[test]
    fn timestamp_roundtrip() {
        let out = timestamp("2024-01-02 03:04:05", "to").unwrap();
        assert!(out.contains("秒:"));
        let ts = out
            .lines()
            .next()
            .unwrap()
            .trim_start_matches("秒: ")
            .to_string();
        let back = timestamp(&ts, "s").unwrap();
        assert!(back.contains("本地:"));
    }

    #[test]
    fn unicode_roundtrip() {
        let escaped = unicode_escape("你a😀", "u");
        assert_eq!(escaped, "\\u4F60\\u0061\\uD83D\\uDE00");
        assert_eq!(unicode_unescape(&escaped).unwrap(), "你a😀");
        assert_eq!(unicode_unescape("\\u{1F600}").unwrap(), "😀");
    }
}
