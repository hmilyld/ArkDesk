//! 正文清洗：HTML → 段落 → 分句（逐行移植自 Python `z2v/processor/cleaner.py`）。

use std::sync::OnceLock;

use regex::Regex;
use scraper::{ElementRef, Html};

use super::models::{Content, ContentDraft};

/// 需要跳过的段落（跳过原因 + 说明）
#[derive(Debug, Clone)]
pub struct Skip {
    pub reason: String,
    pub detail: String,
}

impl Skip {
    pub fn new(reason: &str, detail: impl Into<String>) -> Self {
        Skip {
            reason: reason.to_string(),
            detail: detail.into(),
        }
    }
}

/// 清洗参数（来自设置）
#[derive(Debug, Clone)]
pub struct CleanOptions {
    pub min_chars: usize,
    pub max_chars: usize,
    pub min_sentence: usize,
    pub max_sentence: usize,
    pub min_block: usize,
    pub banned_keywords: Vec<String>,
}

/// 推广/无关段落正则（与 Python 版逐条对应；Rust regex 不支持 lookaround，相关逻辑另写）
const PROMO_PATTERNS: &[&str] = &[
    r"^发布于",
    r"^编辑于",
    r"^(赞同|喜欢|收藏|分享)\s*\d*$",
    r"关注.{0,12}(公众号|微信号)",
    r"(微信|weixin|wx|vx|VX)[号:：]",
    r"扫码(关注|添加)",
    r"(下载|打开)(知乎|知乎App)",
    r"^广告$",
    r"^盐选",
    r"本文收录于",
    r"以上内容(?:均)?由",
    r"更多精彩.{0,10}(请|戳)",
    r"^(编辑|责编|排版|审核)[:：]",
    r"公有领域",
    r"(?i)public\s*domain",
    r"(此作品|本作品).{0,30}(出版|逝世|版权|领域)",
    r"作者逝世",
    r"[一又]作[「【『（(]",
];

const EMOJI_CLASS: &str = concat!(
    "[",
    "\\x{1F000}-\\x{1FAFF}",
    "\\x{2600}-\\x{27BF}",
    "\\x{1F1E6}-\\x{1F1FF}",
    "\\x{2B00}-\\x{2BFF}",
    "\\x{FE0F}\\x{200D}\\x{2B50}\\x{3030}\\x{303D}\\x{3297}\\x{3299}",
    "]+"
);

const BLOCK_TAGS: &[&str] = &["p", "h1", "h2", "h3", "h4", "li", "blockquote", "pre"];
const SENT_TERMINATORS: &[char] = &['。', '！', '？', '!', '?', '…', '；', ';'];
const LONG_SPLIT: &[char] = &['，', ',', '、', '：', ':'];

fn promo_regexes() -> &'static Vec<Regex> {
    static RES: OnceLock<Vec<Regex>> = OnceLock::new();
    RES.get_or_init(|| {
        PROMO_PATTERNS
            .iter()
            .map(|p| Regex::new(p).expect("推广正则编译失败"))
            .collect()
    })
}

fn emoji_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(EMOJI_CLASS).expect("emoji 正则编译失败"))
}

fn whitespace_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("空白正则编译失败"))
}

fn is_promo(text: &str) -> bool {
    promo_regexes().iter().any(|re| re.is_match(text))
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

/// 提取元素内全部文本节点，以空格连接（对齐 BeautifulSoup `get_text(" ", strip=True)`）
fn element_text(el: &ElementRef) -> String {
    el.text()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// HTML → 段落文本列表：取块级标签的纯文本，跳过嵌套块（父块已收录）
pub fn html_to_blocks(html: &str) -> Vec<String> {
    let doc = Html::parse_fragment(html);
    let mut blocks = Vec::new();
    walk_blocks(&doc.root_element(), &mut blocks);
    blocks
}

fn walk_blocks(el: &ElementRef, out: &mut Vec<String>) {
    if BLOCK_TAGS.contains(&el.value().name()) {
        let text = element_text(el);
        if !text.is_empty() {
            out.push(text);
        }
        return; // 已收录父块，内部嵌套块不再重复
    }
    for child in el.child_elements() {
        walk_blocks(&child, out);
    }
}

/// 单行清洗：去 emoji、全角空格、压缩空白、去 CJK 间空格
pub fn clean_line(text: &str) -> String {
    let no_emoji = emoji_regex().replace_all(text, "");
    let normalized = no_emoji.replace('\u{3000}', " ");
    let collapsed = whitespace_regex().replace_all(&normalized, " ");
    remove_cjk_spaces(collapsed.trim())
}

/// 去除中文字符之间的空格（等价 Python 的 lookbehind/lookahead 正则）
fn remove_cjk_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            if let (Some(p), Some(n)) = (prev, next) {
                if is_cjk(p) && is_cjk(n) {
                    continue;
                }
            }
        }
        out.push(ch);
    }
    out
}

/// 分句：按终止符切分、合并短句、长句按标点二次切分（移植 split_sentences）
pub fn split_sentences(text: &str, min_sent: usize, max_sent: usize) -> Vec<String> {
    let mut raw: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if SENT_TERMINATORS.contains(&ch) {
            raw.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        raw.push(cur);
    }
    let raw: Vec<String> = raw.into_iter().filter(|s| !s.trim().is_empty()).collect();

    // 合并到最小长度
    let mut merged: Vec<String> = Vec::new();
    let mut buf = String::new();
    for part in raw {
        buf.push_str(&part);
        if buf.trim().chars().count() >= min_sent {
            merged.push(buf.trim().to_string());
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        if let Some(last) = merged.last_mut() {
            last.push_str(buf.trim());
        } else {
            merged.push(buf.trim().to_string());
        }
    }

    // 长句按标点二次切分
    let mut out: Vec<String> = Vec::new();
    for s in merged {
        let mut cur: Vec<char> = s.chars().collect();
        while cur.len() > max_sent {
            let cut = cur
                .iter()
                .take(max_sent)
                .rposition(|c| LONG_SPLIT.contains(c))
                .map(|i| i + 1)
                .unwrap_or(max_sent);
            let head: String = cur[..cut].iter().collect::<String>().trim().to_string();
            if !head.is_empty() {
                out.push(head);
            }
            cur = cur[cut..].to_vec();
            while cur.first().is_some_and(|c| c.is_whitespace()) {
                cur.remove(0);
            }
        }
        if !cur.is_empty() {
            let tail: String = cur.into_iter().collect();
            if !tail.trim().is_empty() {
                out.push(tail.trim().to_string());
            }
        }
    }
    out
}

fn check_sensitive(paragraphs: &[String], banned: &[String]) -> Result<(), Skip> {
    let joined = paragraphs.concat();
    let joined_lower = joined.to_lowercase();
    for kw in banned {
        if !kw.is_empty() && joined_lower.contains(&kw.to_lowercase()) {
            return Err(Skip::new("skipped_sensitive", format!("命中关键词: {kw}")));
        }
    }
    Ok(())
}

/// 草稿 → 正文（含长度校验与敏感词检查）
pub fn draft_to_content(draft: &ContentDraft, opts: &CleanOptions) -> Result<Content, Skip> {
    let mut paragraphs: Vec<String> = Vec::new();
    for block in html_to_blocks(&draft.html) {
        let line = clean_line(&block);
        if line.is_empty() || line.chars().count() < opts.min_block {
            continue;
        }
        if is_promo(&line) {
            continue;
        }
        paragraphs.push(line);
    }
    if paragraphs.is_empty() {
        return Err(Skip::new("skipped_empty", "清洗后无有效正文"));
    }

    let total: usize = paragraphs.iter().map(|p| p.chars().count()).sum();
    if total < opts.min_chars {
        return Err(Skip::new(
            "skipped_short",
            format!("正文仅 {total} 字 (<{})", opts.min_chars),
        ));
    }
    if total > opts.max_chars {
        return Err(Skip::new(
            "skipped_long",
            format!("正文 {total} 字 (>{})", opts.max_chars),
        ));
    }
    check_sensitive(&paragraphs, &opts.banned_keywords)?;

    let sentence_groups = paragraphs
        .iter()
        .map(|p| split_sentences(p, opts.min_sentence, opts.max_sentence))
        .collect();

    Ok(Content {
        title: draft.title.clone(),
        author: draft.author.clone(),
        voteups: draft.voteups,
        source_url: draft.source_url.clone(),
        source_type: draft.source_type.clone(),
        ref_id: draft.ref_id.clone(),
        paragraphs,
        sentence_groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_merges_sentences() {
        let text = "你好。世界！这是很短的一句";
        let out = split_sentences(text, 8, 42);
        assert!(!out.is_empty());
        assert!(out.iter().all(|s| !s.trim().is_empty()));
    }

    #[test]
    fn removes_cjk_spaces() {
        assert_eq!(clean_line("你 好  世 界"), "你好世界");
        assert_eq!(clean_line("hello   world"), "hello world");
    }

    #[test]
    fn extracts_blocks_skipping_nested() {
        let html = "<div><p>第一段</p><blockquote><p>引用</p></blockquote></div>";
        let blocks = html_to_blocks(html);
        assert_eq!(blocks, vec!["第一段", "引用"]);
    }

    #[test]
    fn rejects_short_content() {
        let draft = ContentDraft {
            title: "t".into(),
            html: "<p>太短</p>".into(),
            author: "a".into(),
            voteups: 0,
            source_url: "u".into(),
            source_type: "classics".into(),
            ref_id: "r".into(),
        };
        let opts = CleanOptions {
            min_chars: 200,
            max_chars: 8000,
            min_sentence: 8,
            max_sentence: 42,
            min_block: 4,
            banned_keywords: vec![],
        };
        let err = draft_to_content(&draft, &opts).unwrap_err();
        assert_eq!(err.reason, "skipped_empty");
    }
}
