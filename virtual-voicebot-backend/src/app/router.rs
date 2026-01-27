use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Identity,
    GeneralChat,
}

#[derive(Debug, Clone)]
pub struct IntentResult {
    pub intent: Intent,
    pub query: String,
    pub raw_intent: String,
}

#[derive(Debug, Clone)]
pub enum RouteAction {
    FixedResponse(String),
    GeneralChat { query: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterConfig {
    pub identity_response: String,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            identity_response: "私はずんだもんです".to_string(),
        }
    }
}

static ROUTER_CONFIG: OnceLock<RouterConfig> = OnceLock::new();

pub fn router_config() -> &'static RouterConfig {
    ROUTER_CONFIG.get_or_init(load_router_config)
}

fn load_router_config() -> RouterConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_yaml::from_str::<RouterConfig>(&text) {
            Ok(cfg) => cfg,
            Err(err) => {
                log::warn!(
                    "[router] failed to parse config {:?}: {:?}. Using default.",
                    path,
                    err
                );
                RouterConfig::default()
            }
        },
        Err(err) => {
            log::warn!(
                "[router] config not found {:?}: {:?}. Using default.",
                path,
                err
            );
            RouterConfig::default()
        }
    }
}

fn config_path() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let default_path = base.join("intent_router.yaml");
    let env_path = std::env::var("INTENT_ROUTER_CONFIG").ok();
    env_path.map(PathBuf::from).unwrap_or(default_path)
}

pub fn parse_intent_json(raw: &str, fallback_query: &str) -> IntentResult {
    #[derive(Deserialize)]
    struct IntentPayload {
        intent: String,
        #[serde(default)]
        query: Option<String>,
    }

    let trimmed = raw.trim();
    let parsed = serde_json::from_str::<IntentPayload>(trimmed);
    match parsed {
        Ok(payload) => {
            let intent = match payload.intent.to_ascii_lowercase().as_str() {
                "identity" => Intent::Identity,
                "general_chat" => Intent::GeneralChat,
                _ => Intent::GeneralChat,
            };
            let query = payload
                .query
                .filter(|q| !q.trim().is_empty())
                .unwrap_or_else(|| fallback_query.to_string());
            IntentResult {
                intent,
                query,
                raw_intent: payload.intent,
            }
        }
        Err(_) => IntentResult {
            intent: Intent::GeneralChat,
            query: fallback_query.to_string(),
            raw_intent: "general_chat".to_string(),
        },
    }
}

pub struct Router {
    cfg: RouterConfig,
}

impl Router {
    pub fn new() -> Self {
        Self {
            cfg: router_config().clone(),
        }
    }

    pub fn route(&self, result: IntentResult) -> RouteAction {
        match result.intent {
            Intent::Identity => RouteAction::FixedResponse(self.cfg.identity_response.clone()),
            Intent::GeneralChat => RouteAction::GeneralChat {
                query: result.query,
            },
        }
    }
}
