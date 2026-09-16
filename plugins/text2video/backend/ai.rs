//! AI 写作：调用 OpenAI 兼容的 `/chat/completions`，要求模型返回
//! `{"title","body"}`，并做容错解析。配置来自全局设置（系统设置 → AI）。

use std::time::Duration;

use serde_json::json;

use crate::error::{code, AppError};

use super::models::{AiArticle, AiConfig, AiGenerateRequest};

const SYSTEM_PROMPT: &str = "你是中文写作者。严格只输出一个 JSON 对象，格式为 \
{\"title\":\"文章标题\",\"body\":\"正文\"}，不要输出任何解释、Markdown 代码块标记或多余文字。\
正文只用自然段落，段落之间用空行分隔，不使用 Markdown 符号、序号列表或小标题。";

fn build_user_prompt(req: &AiGenerateRequest) -> String {
    if req.mode == "free" {
        return req.prompt.trim().to_string();
    }
    let topic = req.topic.trim();
    let style = if req.style.trim().is_empty() {
        "自然流畅".to_string()
    } else {
        req.style.trim().to_string()
    };
    let word_count = if req.word_count == 0 {
        900
    } else {
        req.word_count
    };
    format!(
        "请围绕主题「{topic}」写一篇文章，风格：{style}；全文约 {word_count} 字；\
开头三句要能抓住读者，结尾给出一个可执行的小建议。标题请自行拟定。"
    )
}

pub async fn generate(config: &AiConfig, req: &AiGenerateRequest) -> Result<AiArticle, AppError> {
    if config.base_url.trim().is_empty()
        || config.api_key.trim().is_empty()
        || config.model.trim().is_empty()
    {
        return Err(AppError::invalid_input(
            "请先在「系统设置 → AI」中配置接口地址、API Key 与模型",
        ));
    }
    let user = build_user_prompt(req);
    if user.is_empty() {
        return Err(AppError::invalid_input("提示词不能为空"));
    }

    let url = format!(
        "{}/chat/completions",
        config.base_url.trim().trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|err| {
            AppError::custom(code::HTTP_ERROR, format!("HTTP 客户端初始化失败: {err}"))
        })?;
    let body = json!({
        "model": config.model,
        "temperature": 0.85,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user },
        ],
    });

    let mut last_err = String::new();
    for attempt in 0..2 {
        match client
            .post(&url)
            .bearer_auth(&config.api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    last_err = format!("HTTP {}: {}", status.as_u16(), truncate(&text, 300));
                } else {
                    match parse_article(&text) {
                        Ok(article) => return Ok(article),
                        Err(err) => last_err = err,
                    }
                }
            }
            Err(err) => last_err = err.to_string(),
        }
        if attempt == 0 {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
    Err(AppError::custom(
        code::HTTP_ERROR,
        format!("AI 调用失败: {last_err}"),
    ))
}

fn parse_article(response_text: &str) -> Result<AiArticle, String> {
    let root: serde_json::Value =
        serde_json::from_str(response_text).map_err(|err| format!("响应解析失败: {err}"))?;
    let content = root
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "响应缺少 choices[0].message.content".to_string())?;
    parse_content(content)
}

fn parse_content(content: &str) -> Result<AiArticle, String> {
    let trimmed = content.trim();
    // 先按整体 JSON 解析；失败再尝试截取其中的 JSON 对象。
    // 只要解析成 JSON，就按 JSON 语义处理（正文为空视为失败），
    // 避免把原始 JSON 文本当作正文写入视频。
    let parsed = serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .or_else(|| extract_json_object(trimmed).and_then(|s| serde_json::from_str(s).ok()));

    if let Some(value) = parsed {
        let title = value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let body = value
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if body.is_empty() {
            return Err("AI 返回的 JSON 缺少正文".to_string());
        }
        let title = if title.is_empty() {
            first_line(&body)
        } else {
            title
        };
        return Ok(AiArticle { title, body });
    }

    // 非 JSON：首行作标题，其余作正文
    let mut lines = trimmed.lines();
    let title = lines
        .next()
        .unwrap_or("未命名")
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_string();
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    if body.is_empty() {
        return Err("AI 返回内容为空或无法解析".to_string());
    }
    Ok(AiArticle { title, body })
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("未命名")
        .trim()
        .chars()
        .take(40)
        .collect()
}

fn truncate(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        text.to_string()
    } else {
        chars[..max].iter().collect()
    }
}
