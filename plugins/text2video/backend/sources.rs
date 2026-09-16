//! 输入来源：手动 / AI 文本 → 构建草稿（ContentDraft）。

use super::models::{ContentDraft, ManualInput};

/// 由手动 / AI 文本构建草稿（正文按空行分段）。
///
/// `seq` 为该批次内的序号：`ref_id` 与目录名都依赖它，避免同一毫秒内
/// 批量构建时 `ref_id` 相同导致历史记录被覆盖、目录互相覆盖。
pub fn manual_draft(input: &ManualInput, seq: usize) -> ContentDraft {
    let paragraphs = split_paragraphs(&input.content);
    let html = paragraphs
        .iter()
        .map(|p| format!("<p>{}</p>", escape_html(p)))
        .collect::<String>();
    let ref_id = format!(
        "manual:{}:{seq}",
        chrono::Local::now().format("%Y%m%d%H%M%S%3f")
    );
    let author = if input.author.trim().is_empty() {
        "佚名".to_string()
    } else {
        input.author.trim().to_string()
    };
    let source_type = if input.source.trim().is_empty() {
        "manual".to_string()
    } else {
        input.source.trim().to_string()
    };
    ContentDraft {
        title: input.title.trim().to_string(),
        html,
        author,
        voteups: 0,
        source_url: String::new(),
        source_type,
        ref_id,
    }
}

/// 正文按空行分段（段落内的换行并入同段，交由清洗阶段压缩空白）
fn split_paragraphs(content: &str) -> Vec<String> {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut buf = String::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            if !buf.trim().is_empty() {
                paragraphs.push(std::mem::take(&mut buf).trim().to_string());
            }
            buf.clear();
        } else {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
        }
    }
    if !buf.trim().is_empty() {
        paragraphs.push(buf.trim().to_string());
    }
    paragraphs
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str, author: &str, source: &str) -> ManualInput {
        ManualInput {
            title: title.to_string(),
            author: author.to_string(),
            content: "第一段。\n\n第二段。".to_string(),
            source: source.to_string(),
            draft_id: None,
        }
    }

    /// 同一毫秒内批量构建时，seq 保证 ref_id 唯一（历史记录不被覆盖）
    #[test]
    fn manual_ref_ids_unique_per_seq() {
        let a = manual_draft(&input("标题", "", "manual"), 0);
        let b = manual_draft(&input("标题", "", "manual"), 1);
        assert_ne!(a.ref_id, b.ref_id);
        assert!(a.ref_id.ends_with(":0"));
        assert!(b.ref_id.ends_with(":1"));
    }

    /// 标题去除首尾空白；作者为空回退「佚名」；来源为空回退 manual
    #[test]
    fn trims_title_and_defaults() {
        let draft = manual_draft(&input("  标题  ", "", ""), 0);
        assert_eq!(draft.title, "标题");
        assert_eq!(draft.author, "佚名");
        assert_eq!(draft.source_type, "manual");
        assert!(draft.html.contains("<p>第一段。</p>"));
    }
}
