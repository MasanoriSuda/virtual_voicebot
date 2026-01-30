use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use chrono::Local;
use serde::Deserialize;
use virtual_voicebot_backend::logging;

#[derive(Debug, Deserialize, Default)]
struct TestsFile {
    #[serde(default)]
    cases: Vec<TestCaseDef>,
}

#[derive(Debug, Deserialize, Default)]
struct TestCaseDef {
    id: String,
    title: String,
    kind: String,
    cargo_target: Option<String>,
    compose_file: Option<String>,
    scenario: Option<String>,
}

#[derive(Debug, Clone)]
struct TestCase {
    id: String,
    title: String,
    kind: TestKind,
    category_dir: PathBuf,
    category_rel: String,
}

#[derive(Debug, Clone)]
enum TestKind {
    CargoTest {
        cargo_target: String,
    },
    SippCompose {
        compose_file: PathBuf,
        scenario_rel: Option<String>,
    },
}

#[derive(Debug)]
struct CaseResult {
    id: String,
    title: String,
    duration_sec: f64,
    success: bool,
    exit_code: Option<i32>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    command: String,
}

#[derive(Clone, Copy, Debug)]
enum RunMode {
    Always,
    All,
    Custom,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init();
    let mode = parse_mode(env::args().skip(1))?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_root = root.join("test");

    let discovered = discover_tests(&test_root)?;
    if discovered.is_empty() {
        log::error!("no tests.toml found under {}", test_root.display());
        std::process::exit(2);
    }

    let cases = load_cases(&discovered, &test_root, &root)?;
    if cases.is_empty() {
        log::error!("no cases found");
        std::process::exit(2);
    }
    ensure_unique_ids(&cases)?;

    let selected = select_cases(&cases, &test_root, mode)?;
    if selected.is_empty() {
        log::info!("no cases selected");
        return Ok(());
    }

    let run_id = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let grouped = group_by_category(&selected);

    let mut all_results: Vec<(String, Vec<CaseResult>)> = Vec::new();
    for (category_rel, cat_cases) in grouped {
        let category_dir = cat_cases
            .first()
            .expect("category should have cases")
            .category_dir
            .clone();
        let result_dir = category_dir.join("result").join(&run_id);
        fs::create_dir_all(&result_dir)?;

        let mut results = Vec::new();
        for case in cat_cases {
            let case_dir = result_dir.join(&case.id);
            fs::create_dir_all(&case_dir)?;
            let result = run_case(&root, &run_id, &case, &case_dir);
            results.push(result);
        }

        let junit_path = result_dir.join("junit.xml");
        let junit = build_junit(&category_rel, &results);
        fs::write(&junit_path, junit)?;

        all_results.push((category_rel, results));
    }

    log::info!("run_id={}", run_id);
    for (category_rel, _results) in &all_results {
        let category_dir = test_root.join(category_rel);
        let result_dir = category_dir.join("result").join(&run_id);
        let junit_path = result_dir.join("junit.xml");
        log::info!("result: {}", result_dir.display());
        log::info!("junit: {}", junit_path.display());
    }

    let failures = all_results
        .iter()
        .flat_map(|(_, results)| results.iter())
        .filter(|r| !r.success)
        .count();
    if failures > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn parse_mode<I>(mut args: I) -> Result<RunMode, Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        None => Ok(RunMode::Always),
        Some("always") => Ok(RunMode::Always),
        Some("all") => Ok(RunMode::All),
        Some("custom") => Ok(RunMode::Custom),
        Some(value) => Err(format!("unknown mode: {}", value).into()),
    }
}

fn discover_tests(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    visit_dirs(root, &mut found)?;
    Ok(found)
}

fn visit_dirs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
            if name == "result" || name == "artifacts" || name == "target" {
                continue;
            }
            visit_dirs(&path, found)?;
        } else if path.file_name().and_then(|v| v.to_str()) == Some("tests.toml") {
            found.push(path);
        }
    }
    Ok(())
}

fn load_cases(
    tests_files: &[PathBuf],
    test_root: &Path,
    repo_root: &Path,
) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for path in tests_files {
        let text = fs::read_to_string(path)?;
        let data: TestsFile = toml::from_str(&text)?;
        let category_dir = path.parent().unwrap_or(test_root).to_path_buf();
        let category_rel = category_dir
            .strip_prefix(test_root)
            .unwrap_or(&category_dir)
            .to_string_lossy()
            .to_string();

        for case in data.cases {
            validate_test_id(&case.id)?;
            let kind = match case.kind.as_str() {
                "cargo_test" => {
                    let cargo_target = case
                        .cargo_target
                        .ok_or_else(|| format!("cargo_target is required for {}", case.id))?;
                    TestKind::CargoTest { cargo_target }
                }
                "sipp_compose" => {
                    let compose_file = case
                        .compose_file
                        .ok_or_else(|| format!("compose_file is required for {}", case.id))?;
                    let compose_path = resolve_path(&category_dir, &compose_file);
                    let scenario_rel = match case.scenario.as_ref() {
                        Some(scenario) => {
                            let path = resolve_path(&category_dir, scenario);
                            if !path.exists() {
                                return Err(format!(
                                    "scenario not found for {}: {}",
                                    case.id,
                                    path.display()
                                )
                                .into());
                            }
                            let rel = path.strip_prefix(repo_root).map_err(|_| {
                                format!(
                                    "scenario must be under repo root for {}: {}",
                                    case.id,
                                    path.display()
                                )
                            })?;
                            Some(rel.to_string_lossy().to_string())
                        }
                        None => None,
                    };
                    TestKind::SippCompose {
                        compose_file: compose_path,
                        scenario_rel,
                    }
                }
                other => {
                    return Err(format!("unknown kind {} for {}", other, case.id).into());
                }
            };

            cases.push(TestCase {
                id: case.id,
                title: case.title,
                kind,
                category_dir: category_dir.clone(),
                category_rel: category_rel.clone(),
            });
        }
    }
    Ok(cases)
}

fn validate_test_id(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    if id.len() < 6 {
        return Err(format!("invalid test id (too short): {}", id).into());
    }
    let (prefix, digits) = id.split_at(id.len() - 5);
    if prefix.is_empty() {
        return Err(format!("invalid test id (missing prefix): {}", id).into());
    }
    if !prefix.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("invalid test id prefix: {}", id).into());
    }
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid test id digits: {}", id).into());
    }
    Ok(())
}

fn ensure_unique_ids(cases: &[TestCase]) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen: HashMap<&str, &TestCase> = HashMap::new();
    for case in cases {
        if let Some(existing) = seen.insert(&case.id, case) {
            return Err(format!(
                "duplicate test id {} in {} and {}",
                case.id, existing.category_rel, case.category_rel
            )
            .into());
        }
    }
    Ok(())
}

fn select_cases(
    cases: &[TestCase],
    test_root: &Path,
    mode: RunMode,
) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    match mode {
        RunMode::All => Ok(cases.to_vec()),
        RunMode::Always => {
            let plan_path = test_root.join("plan/always.txt");
            let ids = read_plan_ids(&plan_path)?;
            if ids.is_empty() {
                return Err(format!("always plan is empty: {}", plan_path.display()).into());
            }
            filter_by_ids(cases, &ids)
        }
        RunMode::Custom => {
            let plan_path = test_root.join("plan/custom.txt");
            let ids = read_plan_ids(&plan_path)?;
            if ids.is_empty() {
                log::info!("custom plan empty: {}", plan_path.display());
                print_case_list(cases);
                return Ok(Vec::new());
            }
            filter_by_ids(cases, &ids)
        }
    }
}

fn read_plan_ids(path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        ids.push(trimmed.to_string());
    }
    Ok(ids)
}

fn filter_by_ids(
    cases: &[TestCase],
    ids: &[String],
) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    let wanted: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    let mut selected = Vec::new();
    for case in cases {
        if wanted.contains(case.id.as_str()) {
            selected.push(case.clone());
        }
    }
    let missing: Vec<&str> = ids
        .iter()
        .map(|s| s.as_str())
        .filter(|id| !cases.iter().any(|case| case.id == *id))
        .collect();
    if !missing.is_empty() {
        return Err(format!("unknown test id(s): {}", missing.join(", ")).into());
    }
    Ok(selected)
}

fn print_case_list(cases: &[TestCase]) {
    for case in cases {
        log::info!("{} {}", case.id, case.title);
    }
}

fn group_by_category(cases: &[TestCase]) -> HashMap<String, Vec<TestCase>> {
    let mut map: HashMap<String, Vec<TestCase>> = HashMap::new();
    for case in cases {
        map.entry(case.category_rel.clone())
            .or_default()
            .push(case.clone());
    }
    map
}

fn run_case(root: &Path, run_id: &str, case: &TestCase, case_dir: &Path) -> CaseResult {
    match &case.kind {
        TestKind::CargoTest { cargo_target } => run_cargo_test(root, case, cargo_target, case_dir),
        TestKind::SippCompose {
            compose_file,
            scenario_rel,
        } => run_sipp_compose(
            run_id,
            case,
            compose_file,
            scenario_rel.as_deref(),
            case_dir,
        ),
    }
}

fn run_cargo_test(root: &Path, case: &TestCase, cargo_target: &str, case_dir: &Path) -> CaseResult {
    let mut cmd = Command::new("cargo");
    cmd.arg("test").arg("-q").arg("--test").arg(cargo_target);
    cmd.current_dir(root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let command_line = format!("cargo test -q --test {}", cargo_target);
    let started = Instant::now();
    let output = cmd.output();
    let duration_sec = started.elapsed().as_secs_f64();

    let stdout_path = case_dir.join("stdout.log");
    let stderr_path = case_dir.join("stderr.log");

    match output {
        Ok(out) => {
            let _ = fs::write(&stdout_path, &out.stdout);
            let _ = fs::write(&stderr_path, &out.stderr);
            CaseResult {
                id: case.id.clone(),
                title: case.title.clone(),
                duration_sec,
                success: out.status.success(),
                exit_code: out.status.code(),
                stdout_path,
                stderr_path,
                command: command_line,
            }
        }
        Err(err) => {
            let _ = fs::write(&stderr_path, err.to_string());
            CaseResult {
                id: case.id.clone(),
                title: case.title.clone(),
                duration_sec,
                success: false,
                exit_code: None,
                stdout_path,
                stderr_path,
                command: command_line,
            }
        }
    }
}

fn run_sipp_compose(
    run_id: &str,
    case: &TestCase,
    compose_file: &Path,
    scenario_rel: Option<&str>,
    case_dir: &Path,
) -> CaseResult {
    let compose_dir = compose_file.parent().unwrap_or_else(|| Path::new("."));

    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("-f")
        .arg(compose_file)
        .arg("up")
        .arg("--build")
        .arg("--abort-on-container-exit")
        .arg("--exit-code-from")
        .arg("sipp");
    cmd.current_dir(compose_dir);
    cmd.env("RUN_ID", run_id);
    cmd.env("TEST_ID", &case.id);
    cmd.env("RESULT_DIR", case_dir);
    if let Some(rel) = scenario_rel {
        cmd.env("SIPP_SCENARIO", format!("/workspace/{}", rel));
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let command_line = match scenario_rel {
        Some(rel) => format!(
            "SIPP_SCENARIO=/workspace/{} docker compose -f {} up --build --abort-on-container-exit --exit-code-from sipp",
            rel,
            compose_file.display()
        ),
        None => format!(
            "docker compose -f {} up --build --abort-on-container-exit --exit-code-from sipp",
            compose_file.display()
        ),
    };

    let started = Instant::now();
    let output = cmd.output();
    let duration_sec = started.elapsed().as_secs_f64();

    let stdout_path = case_dir.join("stdout.log");
    let stderr_path = case_dir.join("stderr.log");

    let (success, exit_code) = match output {
        Ok(out) => {
            let _ = fs::write(&stdout_path, &out.stdout);
            let _ = fs::write(&stderr_path, &out.stderr);
            (out.status.success(), out.status.code())
        }
        Err(err) => {
            let _ = fs::write(&stderr_path, err.to_string());
            (false, None)
        }
    };

    let mut down_cmd = Command::new("docker");
    down_cmd
        .arg("compose")
        .arg("-f")
        .arg(compose_file)
        .arg("down")
        .arg("-v");
    down_cmd.current_dir(compose_dir);
    down_cmd.env("RUN_ID", run_id);
    down_cmd.env("TEST_ID", &case.id);
    if let Some(rel) = scenario_rel {
        down_cmd.env("SIPP_SCENARIO", format!("/workspace/{}", rel));
    }
    let _ = down_cmd.status();

    CaseResult {
        id: case.id.clone(),
        title: case.title.clone(),
        duration_sec,
        success,
        exit_code,
        stdout_path,
        stderr_path,
        command: command_line,
    }
}

fn build_junit(category: &str, results: &[CaseResult]) -> String {
    let failures = results.iter().filter(|r| !r.success).count();
    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(&format!(
        r#"<testsuite name="regression:{}" tests="{}" failures="{}" errors="0">"#,
        xml_escape(category),
        results.len(),
        failures
    ));
    out.push('\n');
    for result in results {
        out.push_str(&format!(
            r#"  <testcase name="{}" time="{:.3}">"#,
            xml_escape(&result.id),
            result.duration_sec
        ));
        out.push('\n');
        if !result.success {
            let message = format!("exit={:?}", result.exit_code);
            out.push_str(&format!(
                r#"    <failure message="{}">"#,
                xml_escape(&message)
            ));
            out.push_str(&xml_escape(&short_tail(&result.stderr_path, 2048)));
            out.push_str("</failure>\n");
        }
        let summary = format!(
            "title: {}\ncommand: {}\nstdout: {}\nstderr: {}\n",
            result.title,
            result.command,
            result.stdout_path.display(),
            result.stderr_path.display()
        );
        out.push_str("    <system-out>");
        out.push_str(&xml_escape(&summary));
        out.push_str("</system-out>\n");
        out.push_str("  </testcase>\n");
    }
    out.push_str("</testsuite>\n");
    out
}

fn short_tail(path: &Path, limit: usize) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    if bytes.is_empty() {
        return String::new();
    }
    let start = bytes.len().saturating_sub(limit);
    String::from_utf8_lossy(&bytes[start..]).to_string()
}

fn resolve_path(base: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
