/**
 * 加解密工具插件内共享常量与类型（仅被组件 import，不参与插件自动注册）。
 */

export type ByteFormat = 'utf8' | 'hex' | 'base64';

interface CryptoOption {
  value: string;
  label: string;
  /** 不安全/仅兼容，界面标注 */
  legacy?: boolean;
}

export const BYTE_FORMATS: CryptoOption[] = [
  { value: 'utf8', label: '文本 (UTF-8)' },
  { value: 'hex', label: 'Hex' },
  { value: 'base64', label: 'Base64' },
];

/** 「输入/输出内容格式」的短标签（用于提示文案）。 */
export const BYTE_LABELS: Record<string, string> = {
  utf8: '文本 (UTF-8)',
  hex: 'Hex',
  base64: 'Base64',
};

export const HASH_ALGORITHMS: CryptoOption[] = [
  { value: 'md5', label: 'MD5', legacy: true },
  { value: 'sha1', label: 'SHA-1', legacy: true },
  { value: 'sha224', label: 'SHA-224' },
  { value: 'sha256', label: 'SHA-256' },
  { value: 'sha384', label: 'SHA-384' },
  { value: 'sha512', label: 'SHA-512' },
  { value: 'sha3-256', label: 'SHA3-256' },
  { value: 'sha3-512', label: 'SHA3-512' },
  { value: 'sm3', label: 'SM3（国密）' },
  { value: 'crc32', label: 'CRC32' },
];

export const HMAC_ALGORITHMS: CryptoOption[] = HASH_ALGORITHMS.filter(
  (item) => item.value !== 'crc32'
);

export const DIGEST_OUTPUTS: CryptoOption[] = [
  { value: 'hex', label: 'Hex' },
  { value: 'base64', label: 'Base64' },
];

export const ENCODE_SCHEMES: CryptoOption[] = [
  { value: 'base64', label: 'Base64' },
  { value: 'base64url', label: 'Base64 URL-safe' },
  { value: 'base32', label: 'Base32' },
  { value: 'hex', label: 'Hex' },
  { value: 'url', label: 'URL 编码' },
];

export const TEXT_ENCODINGS: CryptoOption[] = [
  { value: 'utf-8', label: 'UTF-8' },
  { value: 'gbk', label: 'GBK' },
  { value: 'gb18030', label: 'GB18030' },
  { value: 'big5', label: 'Big5' },
  { value: 'shift_jis', label: 'Shift_JIS' },
  { value: 'euc-jp', label: 'EUC-JP' },
  { value: 'iso-8859-1', label: 'ISO-8859-1' },
  { value: 'utf-16le', label: 'UTF-16LE' },
  { value: 'utf-16be', label: 'UTF-16BE' },
];

export const SYMMETRIC_ALGORITHMS: CryptoOption[] = [
  { value: 'aes-128', label: 'AES-128' },
  { value: 'aes-192', label: 'AES-192' },
  { value: 'aes-256', label: 'AES-256' },
  { value: 'sm4', label: 'SM4（国密）' },
  { value: 'des', label: 'DES', legacy: true },
  { value: '3des', label: '3DES', legacy: true },
];

export const SYMMETRIC_MODES: CryptoOption[] = [
  { value: 'cbc', label: 'CBC' },
  { value: 'ecb', label: 'ECB' },
  { value: 'ctr', label: 'CTR' },
  { value: 'gcm', label: 'GCM（仅 AES）' },
];

export const KEY_DERIVATIONS: CryptoOption[] = [
  { value: 'evp', label: 'OpenSSL EVP (MD5)' },
  { value: 'pbkdf2', label: 'PBKDF2-HMAC-SHA256' },
];

/** 复制文本到剪贴板（调用方负责 toast）。 */
export async function copyToClipboard(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}
