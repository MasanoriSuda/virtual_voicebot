use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::shared::config;
use crate::shared::ports::ai::{ChatMessage, Role, WeatherQuery};

#[derive(Debug, Clone)]
struct WeatherReport {
    location: String,
    date: String,
    weather: Option<String>,
    temp_min: Option<i32>,
    temp_max: Option<i32>,
    pops: Vec<String>,
}

#[derive(Debug, Clone)]
struct CachedReport {
    report: WeatherReport,
    fetched_at: Instant,
}

const DEFAULT_WEATHER_PROMPT: &str = r#"
あなたは天気予報の要約アシスタントです。
次のJSONを読み取り、日本語で簡潔に天気を1文で答えてください。

要件:
- 1文で短く
- 気温があれば最高/最低を含める
- JSON以外の情報は使わない
"#;

const WEATHER_PROMPT_FILE_NAME: &str = "weather_prompt.local.txt";
const WEATHER_PROMPT_EXAMPLE: &str = "weather_prompt.example.txt";

static WEATHER_PROMPT_CACHE: OnceLock<String> = OnceLock::new();
static WEATHER_CACHE: OnceLock<Mutex<HashMap<String, CachedReport>>> = OnceLock::new();

pub async fn handle_weather(query: WeatherQuery) -> Result<String> {
    let report = fetch_weather_report(&query).await?;
    let summary = summarize_weather(&report).await;
    Ok(summary)
}

async fn summarize_weather(report: &WeatherReport) -> String {
    let payload = serde_json::json!({
        "location": report.location,
        "date": report.date,
        "weather": report.weather,
        "temp_min": report.temp_min,
        "temp_max": report.temp_max,
        "pops": report.pops,
    });

    let prompt = weather_prompt();
    let model = config::ai_config().ollama_model.clone();
    let messages = vec![ChatMessage {
        role: Role::User,
        content: payload.to_string(),
    }];
    match super::call_ollama_with_prompt(&messages, &prompt, &model).await {
        Ok(text) => text,
        Err(err) => {
            log::warn!("[weather] summarization failed: {err:?}");
            fallback_summary(report)
        }
    }
}

fn fallback_summary(report: &WeatherReport) -> String {
    let mut parts = vec![format!("{}の天気は", report.location)];
    if let Some(weather) = &report.weather {
        parts.push(weather.to_string());
    } else {
        parts.push("取得できませんでした".to_string());
    }
    if report.temp_min.is_some() || report.temp_max.is_some() {
        let min = report
            .temp_min
            .map(|v| format!("最低{}度", v))
            .unwrap_or_default();
        let max = report
            .temp_max
            .map(|v| format!("最高{}度", v))
            .unwrap_or_default();
        let temps = format!("{min} {max}").trim().to_string();
        if !temps.is_empty() {
            parts.push(temps);
        }
    }
    format!("{}です。", parts.join(" "))
}

/// Fetches a weather report for the given query, using a cached value when available and falling back to the remote weather API.
///
/// The function will parse the API response into a `WeatherReport` and store it in the cache before returning.
///
/// # Returns
///
/// `Ok(WeatherReport)` on success, `Err` if the request, parsing, or client construction fails.
///
/// # Examples
///
/// ```ignore
/// # use crate::weather::WeatherQuery;
/// # async fn _example() -> anyhow::Result<()> {
/// let q = WeatherQuery { location: "Tokyo".into(), date: None };
/// let report = crate::weather::fetch_weather_report(&q).await?;
/// println!("{}", report.location);
/// # Ok(())
/// # }
/// ```
async fn fetch_weather_report(query: &WeatherQuery) -> Result<WeatherReport> {
    let weather_cfg = config::weather_config();
    let area_code = location_to_area_code(query.location.as_str())
        .unwrap_or_else(|| weather_cfg.default_area_code.clone());
    let date = query.date.clone().unwrap_or_else(|| "today".to_string());
    let cache_key = format!("{area_code}:{date}");
    if let Some(cached) = load_cache(&cache_key, weather_cfg.cache_ttl).await {
        return Ok(cached);
    }

    let base = weather_cfg.api_base.trim_end_matches('/');
    let url = format!("{}/{}.json", base, area_code);
    let client = reqwest::Client::builder()
        .timeout(config::timeouts().ai_http)
        .build()?;
    let resp = client.get(&url).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("weather api error {}: {}", status, body);
    }
    let json: Value = serde_json::from_str(&body)?;
    let report = parse_weather_report(&json, query.location.as_str(), date.as_str())
        .context("parse weather report")?;
    store_cache(cache_key, report.clone()).await;
    Ok(report)
}

async fn load_cache(cache_key: &str, ttl: Duration) -> Option<WeatherReport> {
    let cache = WEATHER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cache_guard = cache.lock().await;
    cache_guard.get(cache_key).and_then(|cached| {
        if cached.fetched_at.elapsed() <= ttl {
            Some(cached.report.clone())
        } else {
            None
        }
    })
}

async fn store_cache(cache_key: String, report: WeatherReport) {
    let cache = WEATHER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache_guard = cache.lock().await;
    cache_guard.insert(
        cache_key,
        CachedReport {
            report,
            fetched_at: Instant::now(),
        },
    );
}

fn parse_weather_report(value: &Value, location: &str, date: &str) -> Result<WeatherReport> {
    let list = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("weather api response is not array"))?;
    let first = list
        .first()
        .ok_or_else(|| anyhow::anyhow!("weather api response empty"))?;
    let time_series = first
        .get("timeSeries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("weather api missing timeSeries"))?;

    let mut weather_text = None;
    let mut pops = Vec::new();
    let mut temps_min = None;
    let mut temps_max = None;
    let mut area_name = None;

    for series in time_series {
        let areas = match series.get("areas").and_then(|v| v.as_array()) {
            Some(areas) => areas,
            None => continue,
        };
        let area = select_area(areas, location).or_else(|| areas.first());
        let Some(area) = area else { continue };
        if area_name.is_none() {
            area_name = area
                .get("area")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
        }
        if weather_text.is_none() {
            weather_text = area
                .get("weathers")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
        }
        if pops.is_empty() {
            pops = area
                .get("pops")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
        }
        if temps_min.is_none() && temps_max.is_none() {
            let temps_min_arr = area.get("tempsMin").and_then(|v| v.as_array());
            let temps_max_arr = area.get("tempsMax").and_then(|v| v.as_array());
            if temps_min_arr.is_some() || temps_max_arr.is_some() {
                temps_min = parse_first_temp(temps_min_arr);
                temps_max = parse_first_temp(temps_max_arr);
            } else if let Some(temps) = area.get("temps").and_then(|v| v.as_array()) {
                let parsed = temps
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|v| v.parse::<i32>().ok())
                    .collect::<Vec<_>>();
                if let (Some(min), Some(max)) = (parsed.iter().min(), parsed.iter().max()) {
                    temps_min = Some(*min);
                    temps_max = Some(*max);
                }
            }
        }
    }

    Ok(WeatherReport {
        location: area_name.unwrap_or_else(|| location.to_string()),
        date: date.to_string(),
        weather: weather_text,
        temp_min: temps_min,
        temp_max: temps_max,
        pops,
    })
}

fn parse_first_temp(arr: Option<&Vec<Value>>) -> Option<i32> {
    arr.and_then(|v| v.first())
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<i32>().ok())
}

fn select_area<'a>(areas: &'a [Value], location: &str) -> Option<&'a Value> {
    let loc = normalize_location(location);
    if loc.is_empty() {
        return None;
    }
    for area in areas {
        let name = area
            .get("area")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let name_norm = normalize_location(name);
        if name_norm.contains(&loc) || loc.contains(&name_norm) {
            return Some(area);
        }
    }
    None
}

fn normalize_location(input: &str) -> String {
    input
        .replace([' ', '　'], "")
        .replace("都", "")
        .replace("府", "")
        .replace("県", "")
}

fn location_to_area_code(location: &str) -> Option<String> {
    let loc = normalize_location(location);
    if loc.chars().all(|c| c.is_ascii_digit()) && loc.len() == 6 {
        return Some(loc);
    }
    match loc.as_str() {
        "東京" => Some("130000".to_string()),
        "大阪" => Some("270000".to_string()),
        "名古屋" | "愛知" => Some("230000".to_string()),
        "札幌" | "北海道" => Some("016000".to_string()),
        "福岡" => Some("400000".to_string()),
        "仙台" | "宮城" => Some("040000".to_string()),
        "広島" => Some("340000".to_string()),
        _ => None,
    }
}

fn weather_prompt() -> String {
    WEATHER_PROMPT_CACHE
        .get_or_init(|| {
            read_prompt_file()
                .or_else(read_example_file)
                .unwrap_or_else(|| DEFAULT_WEATHER_PROMPT.trim().to_string())
        })
        .clone()
}

fn read_prompt_file() -> Option<String> {
    read_prompt_from(WEATHER_PROMPT_FILE_NAME)
}

fn read_example_file() -> Option<String> {
    read_prompt_from(WEATHER_PROMPT_EXAMPLE)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_location_removes_spaces_and_suffixes() {
        assert_eq!(normalize_location("東京都"), "東京");
        assert_eq!(normalize_location(" 大阪府 "), "大阪");
        assert_eq!(normalize_location("愛知県"), "愛知");
        assert_eq!(normalize_location("　札幌　"), "札幌");
    }

    #[test]
    fn location_to_area_code_maps_known_locations_and_codes() {
        assert_eq!(location_to_area_code("東京"), Some("130000".to_string()));
        assert_eq!(location_to_area_code("大阪府"), Some("270000".to_string()));
        assert_eq!(location_to_area_code("130000"), Some("130000".to_string()));
        assert_eq!(location_to_area_code("Unknown"), None);
    }

    #[test]
    fn select_area_matches_by_normalized_name() {
        let areas = vec![
            json!({"area": {"name": "大阪府"}}),
            json!({"area": {"name": "東京都"}}),
        ];
        let selected = select_area(&areas, "東京").expect("expected matched area");
        let name = selected
            .get("area")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(name, Some("東京都"));
    }

    #[test]
    fn parse_weather_report_extracts_selected_area_values() {
        let value = json!([
            {
                "timeSeries": [
                    {
                        "areas": [
                            {
                                "area": {"name": "大阪府"},
                                "weathers": ["雨"],
                                "pops": ["20", "40"],
                                "tempsMin": ["10"],
                                "tempsMax": ["18"]
                            },
                            {
                                "area": {"name": "東京都"},
                                "weathers": ["晴れ"],
                                "pops": ["10", "20"],
                                "tempsMin": ["12"],
                                "tempsMax": ["22"]
                            }
                        ]
                    }
                ]
            }
        ]);

        let report = parse_weather_report(&value, "東京", "today").expect("parse should succeed");
        assert_eq!(report.location, "東京都");
        assert_eq!(report.date, "today");
        assert_eq!(report.weather.as_deref(), Some("晴れ"));
        assert_eq!(report.temp_min, Some(12));
        assert_eq!(report.temp_max, Some(22));
        assert_eq!(report.pops, vec!["10".to_string(), "20".to_string()]);
    }

    #[test]
    fn parse_weather_report_falls_back_to_temps_range() {
        let value = json!([
            {
                "timeSeries": [
                    {
                        "areas": [
                            {
                                "area": {"name": "名古屋市"},
                                "weathers": ["くもり"],
                                "temps": ["17", "5", "9"]
                            }
                        ]
                    }
                ]
            }
        ]);

        let report = parse_weather_report(&value, "名古屋", "today").expect("parse should succeed");
        assert_eq!(report.location, "名古屋市");
        assert_eq!(report.temp_min, Some(5));
        assert_eq!(report.temp_max, Some(17));
    }

    #[test]
    fn parse_weather_report_returns_error_for_non_array_response() {
        let value = json!({"timeSeries": []});
        assert!(parse_weather_report(&value, "東京", "today").is_err());
    }
}
