use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;

use crate::config;
use crate::ports::ai::{ChatMessage, Role};

const DEFAULT_INTENT_PROMPT: &str = r#"
あなたはボイスボットの意図分類器です。
次のユーザー発話を、必ずJSONのみで分類してください。

要件:
- 出力はJSONのみ
- 形式: {"intent":"identity|general_chat","query":"<ユーザー発話>"}
- intentは identity または general_chat のみ
- queryは入力のユーザー発話をそのまま入れる

例:
{"intent":"identity","query":"あなたの名前は？"}
{"intent":"general_chat","query":"徳川家康について教えて"}
"#;

const INTENT_PROMPT_FILE_NAME: &str = "intent_prompt.local.txt";
const INTENT_PROMPT_EXAMPLE: &str = "intent_prompt.example.txt";

static INTENT_PROMPT_CACHE: OnceLock<String> = OnceLock::new();

pub fn init_intent_prompt() {
    let _ = intent_prompt();
}

pub fn intent_prompt() -> String {
    INTENT_PROMPT_CACHE
        .get_or_init(|| {
            read_prompt_file()
                .or_else(read_example_file)
                .unwrap_or_else(|| DEFAULT_INTENT_PROMPT.trim().to_string())
        })
        .clone()
}

fn read_prompt_file() -> Option<String> {
    read_prompt_from(INTENT_PROMPT_FILE_NAME)
}

fn read_example_file() -> Option<String> {
    read_prompt_from(INTENT_PROMPT_EXAMPLE)
}

fn read_prompt_from(name: &str) -> Option<String> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let path = base.join(name);
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

pub async fn classify_intent(text: String) -> Result<String> {
    let prompt = intent_prompt();
    let model = config::ai_config().ollama_intent_model.clone();
    let messages = vec![ChatMessage {
        role: Role::User,
        content: text,
    }];
    super::call_ollama_with_prompt(&messages, &prompt, &model).await
}
