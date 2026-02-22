use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use tokio::time::timeout;

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

pub async fn classify_intent(call_id: &str, text: String) -> Result<String> {
    let prompt = intent_prompt();
    let ai_cfg = config::ai_config();
    let text_len = text.chars().count();
    let messages = vec![ChatMessage {
        role: Role::User,
        content: text,
    }];

    if !ai_cfg.intent_local_server_enabled && !ai_cfg.intent_raspi_enabled {
        log::error!("[intent {call_id}] intent failed: reason=all intent stages disabled");
        anyhow::bail!("all intent stages failed");
    }

    if ai_cfg.intent_local_server_enabled {
        let endpoint_url = ai_cfg.intent_local_server_url.clone();
        let model = ai_cfg.intent_local_model.clone();
        if let Some(raw) = try_intent_stage(
            call_id,
            super::IntentStage::Local,
            text_len,
            ai_cfg.intent_local_timeout,
            || {
                super::call_ollama_for_intent_stage(
                    &messages,
                    &prompt,
                    &model,
                    &endpoint_url,
                    ai_cfg.intent_local_timeout,
                )
            },
        )
        .await
        {
            return Ok(raw);
        }
    }

    if ai_cfg.intent_raspi_enabled {
        if let Some(endpoint_url) = ai_cfg.intent_raspi_url.clone() {
            let model = ai_cfg.intent_raspi_model.clone();
            if let Some(raw) = try_intent_stage(
                call_id,
                super::IntentStage::Raspi,
                text_len,
                ai_cfg.intent_raspi_timeout,
                || {
                    super::call_ollama_for_intent_stage(
                        &messages,
                        &prompt,
                        &model,
                        &endpoint_url,
                        ai_cfg.intent_raspi_timeout,
                    )
                },
            )
            .await
            {
                return Ok(raw);
            }
        } else {
            log::warn!(
                "[intent {call_id}] intent stage failed: intent_stage=raspi reason=INTENT_RASPI_URL missing"
            );
        }
    }

    log::error!("[intent {call_id}] intent failed: reason=all intent stages failed");
    anyhow::bail!("all intent stages failed")
}

async fn try_intent_stage<F, Fut>(
    call_id: &str,
    stage: super::IntentStage,
    text_len: usize,
    stage_timeout: Duration,
    run: F,
) -> Option<String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let stage_name = stage.as_str();
    log::debug!(
        "[intent {call_id}] intent stage start: intent_stage={} timeout_ms={} text_len={}",
        stage_name,
        stage_timeout.as_millis(),
        text_len
    );

    let raw = match timeout(stage_timeout, run()).await {
        Ok(Ok(raw)) => raw,
        Ok(Err(err)) => {
            log::warn!(
                "[intent {call_id}] intent stage failed: intent_stage={} reason={}",
                stage_name,
                err
            );
            return None;
        }
        Err(_) => {
            log::warn!(
                "[intent {call_id}] intent stage failed: intent_stage={} reason=timeout timeout_ms={}",
                stage_name,
                stage_timeout.as_millis()
            );
            return None;
        }
    };

    log::info!(
        "[intent {call_id}] intent stage success: intent_stage={} raw_len={}",
        stage_name,
        raw.chars().count()
    );
    Some(raw)
}
