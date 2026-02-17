use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Identity,
    SystemInfo,
    Weather,
    Transfer,
    GeneralChat,
}

#[derive(Debug, Clone)]
pub struct IntentResult {
    pub intent: Intent,
    pub query: String,
    pub raw_intent: String,
    pub location: Option<String>,
    pub date: Option<String>,
    pub person: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RouteAction {
    FixedResponse(String),
    GeneralChat {
        query: String,
    },
    Weather {
        query: String,
        location: String,
        date: Option<String>,
    },
    SystemInfo,
    Transfer {
        person: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RouterConfig {
    pub identity_response: String,
    pub weather_default_location: String,
    pub weather_error_response: String,
    pub system_info: Option<SystemInfoConfig>,
    pub system_info_response: Option<String>,
    pub transfer: Option<TransferConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SystemInfoConfig {
    pub default: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TransferConfig {
    pub confirm_message: String,
    pub not_found_message: String,
    pub directory: std::collections::HashMap<String, TransferEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TransferEntry {
    pub aliases: Vec<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            identity_response: "私はずんだもんです".to_string(),
            weather_default_location: "東京".to_string(),
            weather_error_response: "天気情報を取得できませんでした。".to_string(),
            system_info: Some(SystemInfoConfig::default()),
            system_info_response: None,
            transfer: Some(TransferConfig::default()),
        }
    }
}

impl Default for SystemInfoConfig {
    fn default() -> Self {
        Self {
            default: "それは無理なのだ、管理者に連絡するのだ".to_string(),
        }
    }
}

impl Default for TransferConfig {
    fn default() -> Self {
        let mut directory = std::collections::HashMap::new();
        directory.insert(
            "須田".to_string(),
            TransferEntry {
                aliases: vec![
                    "すださん".to_string(),
                    "須田さん".to_string(),
                    "すだ".to_string(),
                    "菅田".to_string(),
                    "菅田さん".to_string(),
                    "すがた".to_string(),
                    "すがたさん".to_string(),
                ],
            },
        );
        Self {
            confirm_message: "おつなぎします".to_string(),
            not_found_message: "申し訳ありません、その方の連絡先が見つかりません".to_string(),
            directory,
        }
    }
}

static ROUTER_CONFIG: OnceLock<RouterConfig> = OnceLock::new();

pub fn router_config() -> &'static RouterConfig {
    ROUTER_CONFIG.get_or_init(load_router_config)
}

pub fn system_info_response() -> String {
    if let Some(cfg) = &router_config().system_info {
        return cfg.default.clone();
    }
    router_config()
        .system_info_response
        .clone()
        .unwrap_or_else(|| SystemInfoConfig::default().default)
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
        #[serde(default)]
        params: Option<IntentParams>,
    }

    #[derive(Deserialize)]
    struct IntentParams {
        #[serde(default)]
        location: Option<String>,
        #[serde(default)]
        date: Option<String>,
        #[serde(default)]
        person: Option<String>,
    }

    let trimmed = raw.trim();
    let sanitized = sanitize_json_block(trimmed);
    let parsed = serde_json::from_str::<IntentPayload>(sanitized.as_str());
    match parsed {
        Ok(payload) => {
            let intent = match payload.intent.to_ascii_lowercase().as_str() {
                "identity" => Intent::Identity,
                "system_info" => Intent::SystemInfo,
                "weather" => Intent::Weather,
                "transfer" => Intent::Transfer,
                "general_chat" => Intent::GeneralChat,
                _ => Intent::GeneralChat,
            };
            let query = payload
                .query
                .filter(|q| !q.trim().is_empty())
                .unwrap_or_else(|| fallback_query.to_string());
            let location = payload
                .params
                .as_ref()
                .and_then(|p| p.location.clone())
                .filter(|v| !v.trim().is_empty());
            let date = payload
                .params
                .as_ref()
                .and_then(|p| p.date.clone())
                .filter(|v| !v.trim().is_empty());
            let person = payload
                .params
                .as_ref()
                .and_then(|p| p.person.clone())
                .filter(|v| !v.trim().is_empty());
            IntentResult {
                intent,
                query,
                raw_intent: payload.intent,
                location,
                date,
                person,
            }
        }
        Err(_) => IntentResult {
            intent: Intent::GeneralChat,
            query: fallback_query.to_string(),
            raw_intent: "general_chat".to_string(),
            location: None,
            date: None,
            person: None,
        },
    }
}

fn sanitize_json_block(input: &str) -> String {
    let mut s = input.trim().to_string();
    if let Some(stripped) = s.strip_prefix("```") {
        s = stripped.trim().to_string();
        if let Some(rest) = s.strip_prefix("json") {
            s = rest.trim().to_string();
        }
        if let Some(end) = s.rfind("```") {
            s = s[..end].trim().to_string();
        }
    }
    s
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
            Intent::Weather => RouteAction::Weather {
                query: result.query,
                location: result
                    .location
                    .unwrap_or_else(|| self.cfg.weather_default_location.clone()),
                date: result.date,
            },
            Intent::SystemInfo => RouteAction::SystemInfo,
            Intent::Transfer => RouteAction::Transfer {
                person: result.person.unwrap_or_default(),
            },
        }
    }

    pub fn transfer_confirm_message(&self) -> String {
        self.cfg
            .transfer
            .as_ref()
            .map(|cfg| cfg.confirm_message.clone())
            .unwrap_or_else(|| TransferConfig::default().confirm_message)
    }

    pub fn transfer_not_found_message(&self) -> String {
        self.cfg
            .transfer
            .as_ref()
            .map(|cfg| cfg.not_found_message.clone())
            .unwrap_or_else(|| TransferConfig::default().not_found_message)
    }

    /// Resolves a transfer directory key that matches the provided person identifier.
    ///
    /// The input is normalized (whitespace and full-/half-width spaces removed) and compared
    /// against directory keys and each entry's aliases. If the normalized input is empty
    /// or no match is found, `None` is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let router = Router::new();
    /// // Default config includes an entry keyed by "須田" with several aliases.
    /// assert_eq!(router.resolve_transfer_person("須田"), Some("須田".to_string()));
    /// assert_eq!(router.resolve_transfer_person("  須田  "), Some("須田".to_string()));
    /// assert_eq!(router.resolve_transfer_person("unknown"), None);
    /// ```
    pub fn resolve_transfer_person(&self, person: &str) -> Option<String> {
        let Some(cfg) = &self.cfg.transfer else {
            return None;
        };
        let target = normalize_person(person);
        if target.is_empty() {
            return None;
        }
        for (name, entry) in &cfg.directory {
            if normalize_person(name) == target {
                return Some(name.clone());
            }
            if entry
                .aliases
                .iter()
                .any(|alias| normalize_person(alias) == target)
            {
                return Some(name.clone());
            }
        }
        None
    }
}

/// Normalize a person name by trimming and removing full-width and half-width spaces.
///
/// The returned string has leading and trailing whitespace removed and all full-width
/// spaces (U+3000) and ASCII space characters removed from the remainder.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(normalize_person("  山田　太郎  "), "山田太郎");
/// assert_eq!(normalize_person("　 Alice Bob "), "AliceBob");
/// ```
fn normalize_person(input: &str) -> String {
    input.trim().replace(['　', ' '], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_person() {
        assert_eq!(normalize_person("  山田　太郎  "), "山田太郎");
        assert_eq!(normalize_person("　 Alice Bob "), "AliceBob");
        assert_eq!(normalize_person("須田"), "須田");
        assert_eq!(normalize_person("  "), "");
        assert_eq!(normalize_person(""), "");
    }
}
