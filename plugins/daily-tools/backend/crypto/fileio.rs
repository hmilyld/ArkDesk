//! 文件流式处理：哈希与编码，分块读取避免整文件驻留内存。

use std::fs::File;
use std::io::{BufReader, Read, Write};

use crate::error::{code, AppError};

use super::hash::Hasher;
use super::progress::StreamObserver;

const CHUNK: usize = 1024 * 1024;

fn open(path: &str) -> Result<BufReader<File>, AppError> {
    let file = File::open(path)
        .map_err(|e| AppError::custom(code::IO_ERROR, format!("打开文件失败: {e}")))?;
    Ok(BufReader::with_capacity(CHUNK, file))
}

/// 流式计算文件哈希，返回摘要字节；每块上报进度并检查取消。
pub fn hash_file(
    path: &str,
    algorithm: &str,
    observer: &mut dyn StreamObserver,
) -> Result<Vec<u8>, AppError> {
    let mut hasher = Hasher::new(algorithm)?;
    let mut reader = open(path)?;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let read = reader
            .read(&mut buf)
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("读取文件失败: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        observer.advance(read as u64)?;
    }
    Ok(hasher.finalize())
}

/// 流式编码文件内容为文本（Base64/Base32/Hex）。
///
/// 按编码单元分组：中间批次不带 padding，仅末尾批次按 `padding` 补齐，
/// 保证结果与一次性编码完全一致。
pub fn encode_file(
    path: &str,
    scheme: &str,
    padding: bool,
    uppercase: bool,
    observer: &mut dyn StreamObserver,
) -> Result<String, AppError> {
    let mut out: Vec<u8> = Vec::new();
    encode_file_chunks(path, scheme, padding, uppercase, observer, |chunk| {
        out.extend_from_slice(chunk);
        Ok(())
    })?;
    String::from_utf8(out)
        .map_err(|_| AppError::custom(code::UNKNOWN, "编码结果不是合法 UTF-8"))
}

/// 流式编码并直接写入目标文件（适合大文件，避免整段驻留内存/经 IPC 传输）。
///
/// 失败（含取消）时删除未完成的输出文件。
pub fn encode_file_to(
    path: &str,
    scheme: &str,
    padding: bool,
    uppercase: bool,
    output: &str,
    observer: &mut dyn StreamObserver,
) -> Result<(), AppError> {
    let result = (|| -> Result<(), AppError> {
        let mut writer = std::io::BufWriter::with_capacity(
            CHUNK,
            File::create(output)
                .map_err(|e| AppError::custom(code::IO_ERROR, format!("创建输出文件失败: {e}")))?,
        );
        encode_file_chunks(path, scheme, padding, uppercase, observer, |chunk| {
            writer
                .write_all(chunk)
                .map_err(|e| AppError::custom(code::IO_ERROR, format!("写入失败: {e}")))
        })?;
        writer
            .flush()
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("写入失败: {e}")))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(output);
    }
    result
}

/// 按编码单元分块编码，逐段交给 `sink`。
fn encode_file_chunks<F>(
    path: &str,
    scheme: &str,
    padding: bool,
    uppercase: bool,
    observer: &mut dyn StreamObserver,
    mut sink: F,
) -> Result<(), AppError>
where
    F: FnMut(&[u8]) -> Result<(), AppError>,
{
    let unit = match super::normalize(scheme).as_str() {
        "base64" | "base64url" => 3,
        "base32" => 5,
        "hex" => 1,
        other => {
            return Err(AppError::invalid_input(format!(
                "文件不支持该编码方式: {other}"
            )))
        }
    };

    let mut reader = open(path)?;
    let mut buf = vec![0u8; CHUNK];
    let mut carry: Vec<u8> = Vec::with_capacity(CHUNK + unit);

    loop {
        let read = reader
            .read(&mut buf)
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("读取文件失败: {e}")))?;
        if read == 0 {
            break;
        }
        observer.advance(read as u64)?;
        carry.extend_from_slice(&buf[..read]);
        let usable = carry.len() - carry.len() % unit;
        if usable > 0 {
            let chunk = super::encoding::encode(&carry[..usable], scheme, false, uppercase)?;
            sink(chunk.as_bytes())?;
            carry.drain(..usable);
        }
    }

    if !carry.is_empty() {
        let chunk = super::encoding::encode(&carry, scheme, padding, uppercase)?;
        sink(chunk.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::progress::NoopObserver;
    use super::*;
    use std::io::Write;

    #[test]
    fn file_hash_and_encode() {
        let path = std::env::temp_dir().join(format!("arkdesk_crypto_{}.bin", std::process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(b"abc").unwrap();
        drop(file);

        let digest = hash_file(path.to_str().unwrap(), "md5", &mut NoopObserver).unwrap();
        assert_eq!(
            data_encoding::HEXLOWER.encode(&digest),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            encode_file(path.to_str().unwrap(), "base64", true, false, &mut NoopObserver).unwrap(),
            "YWJj"
        );

        let out = std::env::temp_dir().join(format!("arkdesk_crypto_out_{}.txt", std::process::id()));
        encode_file_to(
            path.to_str().unwrap(),
            "base64",
            true,
            false,
            out.to_str().unwrap(),
            &mut NoopObserver,
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "YWJj");
        let _ = std::fs::remove_file(&out);

        let _ = std::fs::remove_file(&path);
    }
}
