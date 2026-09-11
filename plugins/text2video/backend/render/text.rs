//! 排版辅助：中文避头尾换行与署名文案（移植自 Python `z2v/renderer/textutil.py`）。

use super::fonts::FontKit;

/// 不可出现在行首的标点
const NO_START: &str = "，。？！；：、）」』】》”’!?%,.:;)]}…—";
/// 不可出现在行尾的开引号
const NO_END: &str = "（「『【《“’([{";

/// 按最大宽度换行（逐字累加 advance，复刻 Python 的标点规则）
pub fn wrap_text(text: &str, fonts: &FontKit, size: f32, bold: bool, max_w: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0.0f32;

    for ch in text.chars() {
        if ch == ' ' && cur.is_empty() {
            continue;
        }
        let adv = fonts.advance(ch, size, bold);
        if cur_w + adv <= max_w {
            cur.push(ch);
            cur_w += adv;
            continue;
        }
        if NO_START.contains(ch) && !cur.is_empty() {
            cur.push(ch);
            lines.push(std::mem::take(&mut cur));
            cur_w = 0.0;
            continue;
        }
        if !cur.is_empty() && cur.chars().last().is_some_and(|c| NO_END.contains(c)) {
            let opener = cur.chars().last().unwrap_or(' ');
            let head: String = cur.chars().take(cur.chars().count() - 1).collect();
            lines.push(head);
            cur.clear();
            cur.push(opener);
            cur.push(ch);
            cur_w = fonts.advance(opener, size, bold) + adv;
            continue;
        }
        lines.push(std::mem::take(&mut cur));
        cur_w = adv;
        cur.push(ch);
    }
    if !cur.trim().is_empty() {
        lines.push(cur);
    }
    lines.into_iter().filter(|l| !l.trim().is_empty()).collect()
}

/// 来源角标与版权说明
pub fn source_brand(source_type: &str) -> (&'static str, &'static str) {
    match source_type {
        "ai" => ("AI 创作", "内容由 AI 生成 · 仅供参考"),
        _ => ("原创", "内容由用户提供"),
    }
}

/// 作者署名行
pub fn credit_line(source_type: &str, author: &str) -> String {
    let (_, credit) = source_brand(source_type);
    format!("作者 @{author} · {credit}")
}
