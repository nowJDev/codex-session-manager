// Codex JSONL 대화 본문에서 사용자에게 의미 있는 텍스트를 판별한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptContent {
    Text(String),
    Parts(Vec<TranscriptPart>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptPart {
    pub kind: TranscriptPartKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptPartKind {
    Text,
    OutputText,
    InputText,
}

pub fn extract_first_text(content: &TranscriptContent) -> Option<String> {
    match content {
        TranscriptContent::Text(text) => Some(text.clone()),
        TranscriptContent::Parts(parts) => parts.first().map(|part| part.text.clone()),
    }
}

pub fn extract_joined_text(content: &TranscriptContent) -> Option<String> {
    match content {
        TranscriptContent::Text(text) => Some(text.clone()),
        TranscriptContent::Parts(parts) => {
            if parts.is_empty() {
                return None;
            }
            let mut buf = String::new();
            for part in parts {
                buf.push_str(&part.text);
                buf.push('\n');
            }
            Some(buf)
        }
    }
}

pub fn is_injected_instruction_message(message: &str) -> bool {
    let trimmed = message.trim_start();
    (trimmed.starts_with("# AGENTS.md instructions") && trimmed.contains("<INSTRUCTIONS>"))
        || trimmed.starts_with("The following is the Codex agent history")
        || trimmed.starts_with("Summarize the following Codex sessions in Korean.")
        || trimmed.starts_with("Summarize this Codex session in Korean.")
        || trimmed.starts_with("<environment_context>")
        || trimmed.starts_with("<skill>")
}

pub fn extract_user_message_text(content: &TranscriptContent) -> Option<String> {
    let text = extract_first_text(content)?;
    if is_injected_instruction_message(&text) {
        None
    } else {
        Some(text)
    }
}

pub fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn compact_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn first_user_message_name(message: &str) -> Option<String> {
    let compact = compact_whitespace(message);
    if compact.is_empty() {
        None
    } else {
        Some(truncate(&compact, 24))
    }
}
