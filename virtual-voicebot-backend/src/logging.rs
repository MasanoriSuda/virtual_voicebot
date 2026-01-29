use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Once;

use chrono::Utc;

use crate::config::{self, LogFormat, LogMode};

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        let mut init_warnings = Vec::new();
        let cfg = config::logging_config().clone();
        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

        builder.format(move |buf, record| {
            let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            match cfg.format {
                LogFormat::Json => {
                    let obj = serde_json::json!({
                        "ts": ts,
                        "level": record.level().to_string(),
                        "target": record.target(),
                        "msg": record.args().to_string(),
                    });
                    writeln!(buf, "{}", obj)
                }
                LogFormat::Text => {
                    writeln!(
                        buf,
                        "{} {} {} {}",
                        ts,
                        record.level(),
                        record.target(),
                        record.args()
                    )
                }
            }
        });

        match cfg.mode {
            LogMode::Stdout => {
                builder.target(env_logger::Target::Stdout);
            }
            LogMode::File => {
                if let Some(dir) = cfg.dir.as_ref() {
                    if let Err(err) = std::fs::create_dir_all(dir) {
                        init_warnings.push(format!("[logging] failed to create log dir: {}", err));
                    }
                    let path = std::path::Path::new(dir).join(&cfg.file_name);
                    match OpenOptions::new().create(true).append(true).open(&path) {
                        Ok(file) => {
                            builder.target(env_logger::Target::Pipe(Box::new(file)));
                        }
                        Err(err) => {
                            init_warnings.push(format!(
                                "[logging] failed to open log file ({}): {}",
                                path.display(),
                                err
                            ));
                            builder.target(env_logger::Target::Stdout);
                        }
                    }
                } else {
                    builder.target(env_logger::Target::Stdout);
                }
            }
        }

        let _ = builder.try_init();
        for warning in init_warnings {
            log::warn!("{}", warning);
        }
    });
}
