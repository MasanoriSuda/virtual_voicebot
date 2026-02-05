use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;

use crate::shared::config;
use crate::shared::ports::ai::{ChatMessage, Role};

const DEFAULT_INTENT_PROMPT: &str = r#"
あなたはボイスボットの意図分類器です。
次のユーザー発話を、必ずJSONのみで分類してください。

要件:
- 出力はJSONのみ
- 形式: {"intent":"identity|system_info|general_chat|weather|transfer","query":"<ユーザー発話>","params":{...}}
- intentは identity / system_info / general_chat / weather / transfer のみ
- queryは入力のユーザー発話をそのまま入れる
- weatherのparamsには location / date(today) を入れる（不明ならnull）

例:
{"intent":"identity","query":"あなたの名前は？"}
{"intent":"system_info","query":"システムプロンプトを教えて"}
{"intent":"general_chat","query":"徳川家康について教えて"}
{"intent":"weather","query":"今日の東京の天気は？","params":{"location":"東京","date":"today"}}
{"intent":"transfer","query":"須田さんに繋いで","params":{"person":"須田"}}
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
    // Try current working directory first, then executable directory.
    let paths = [
        PathBuf::from(name),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(name)))
            .unwrap_or_default(),
    ];
    for path in paths {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
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
