use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use study_coach::{core, eval};

const HELP: &str = "Study Coach\n\
  study-coach validate\n\
  study-coach doctor\n\
  study-coach answer --model groq-120b --question \"What is recall?\"\n\
  study-coach answer --provider groq --model-id MODEL_ID --question TEXT\n\
  study-coach evaluate [--models all|ALIAS,ALIAS] [--runs 3] [--out DIRECTORY]\n\
  study-coach evaluate --resume DIRECTORY [--models all|ALIAS,ALIAS] [--runs 3]\n\
  study-coach evaluate --models groq-20b --runs 1 --limit 2 --out DIRECTORY\n\
  study-coach summarize --out DIRECTORY\n\
\nInputs come from models.json and data/ beside Cargo.toml. Credentials come from\n\
GROQ_API_KEY and NVIDIA_API_KEY, or ignored .env and .env.local files. Nonempty\n\
process values take precedence, followed by .env.local, then .env. Never pass keys\n\
as command arguments. Evaluation defaults to 50 cases, 3 models, and 3 runs.\n\
answer/evaluate accept --approach baseline|grounded (default grounded). The\n\
baseline preserves Milestone 1 instructions; grounded checks quotes and tasks.\n\
summarize is read only and uses saved provenance without making API calls.\n\
--limit selects an explicitly incomplete smoke experiment. Only free accounts\n\
and provider-covered free quotas qualify for the reported conditional $0 cost.";

#[derive(Debug)]
struct Options {
    command: String,
    model: String,
    approach: String,
    provider: Option<String>,
    model_id: Option<String>,
    question: Option<String>,
    models: String,
    runs: usize,
    out: Option<PathBuf>,
    resume: Option<PathBuf>,
    limit: Option<usize>,
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let command = arguments.first().map(String::as_str).unwrap_or("help");
    let mut options = Options {
        command: command.into(),
        model: "groq-120b".into(),
        approach: "grounded".into(),
        provider: None,
        model_id: None,
        question: None,
        models: "all".into(),
        runs: 3,
        out: None,
        resume: None,
        limit: None,
    };
    if ["help", "--help", "-h"].contains(&command) {
        options.command = "help".into();
        if arguments.len() > 1 {
            return Err("help does not accept additional arguments.".into());
        }
        return Ok(options);
    }
    let allowed: &[&str] = match command {
        "validate" | "doctor" => &[],
        "answer" => &[
            "--model",
            "--provider",
            "--model-id",
            "--question",
            "--approach",
        ],
        "evaluate" => &[
            "--models",
            "--runs",
            "--out",
            "--resume",
            "--limit",
            "--approach",
        ],
        "summarize" => &["--out"],
        _ => return Err("Unknown command; run study-coach help.".into()),
    };
    let mut seen = BTreeSet::new();
    let mut index = 1;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        if flag == "--help" || flag == "-h" {
            options.command = "help".into();
            return Ok(options);
        }
        if !allowed.contains(&flag) {
            return Err(format!(
                "Unknown option for {command}; run study-coach help."
            ));
        }
        if !seen.insert(flag) {
            return Err(format!("Duplicate option {flag}."));
        }
        let value = arguments
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("Missing value for {flag}."))?;
        match flag {
            "--model" => options.model = value.clone(),
            "--approach" => {
                if !["baseline", "grounded"].contains(&value.as_str()) {
                    return Err("--approach must be baseline or grounded.".into());
                }
                options.approach = value.clone();
            }
            "--provider" => options.provider = Some(value.clone()),
            "--model-id" => options.model_id = Some(value.clone()),
            "--question" => options.question = Some(value.clone()),
            "--models" => options.models = value.clone(),
            "--runs" => {
                options.runs = value.parse().map_err(|_| "--runs must be an integer.")?;
                if !(1..=10).contains(&options.runs) {
                    return Err("--runs must be between 1 and 10.".into());
                }
            }
            "--out" => options.out = Some(PathBuf::from(value)),
            "--resume" => options.resume = Some(PathBuf::from(value)),
            "--limit" => {
                let count: usize = value.parse().map_err(|_| "--limit must be an integer.")?;
                if count == 0 {
                    return Err("--limit must be positive.".into());
                }
                options.limit = Some(count);
            }
            _ => return Err("Unsupported option.".into()),
        }
        index += 2;
    }
    if options.command == "answer" {
        if options.question.is_none() {
            return Err("answer requires --question TEXT (empty text is allowed).".into());
        }
        if options.provider.is_some() != options.model_id.is_some() {
            return Err("--provider and --model-id must be supplied together.".into());
        }
        if options.provider.is_some() && seen.contains("--model") {
            return Err("Choose --model or the --provider/--model-id pair.".into());
        }
    }
    if options.out.is_some() && options.resume.is_some() {
        return Err("Choose --out or --resume, not both.".into());
    }
    if options.command == "summarize" && options.out.is_none() {
        return Err("summarize requires --out DIRECTORY.".into());
    }
    Ok(options)
}

struct Inputs {
    config: Value,
    notes: Value,
    cases: Value,
    fingerprint: String,
    dataset_fingerprint: String,
    notes_fingerprint: String,
    scorer_fingerprint: String,
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let parsed = serde_json::from_slice(&bytes)
        .map_err(|_| format!("Invalid JSON in {}.", path.display()))?;
    Ok((bytes, parsed))
}

fn load_inputs(root: &Path, approach: &str) -> Result<Inputs, String> {
    let (config_bytes, config) = read_json(&root.join("models.json"))?;
    let (notes_bytes, notes) = read_json(&root.join("data/notes.json"))?;
    let (cases_bytes, cases) = read_json(&root.join("data/eval.json"))?;
    validate_config(&config)?;
    eval::validate(&notes, &cases)?;
    let prompt = core::prompt_fingerprint_material(approach)?;
    let mut material = Vec::new();
    for component in [
        config_bytes.as_slice(),
        notes_bytes.as_slice(),
        cases_bytes.as_slice(),
        prompt.as_bytes(),
        include_str!("core.rs").as_bytes(),
        include_str!("eval.rs").as_bytes(),
        include_str!("main.rs").as_bytes(),
    ] {
        material.extend_from_slice(&(component.len() as u64).to_le_bytes());
        material.extend_from_slice(component);
    }
    Ok(Inputs {
        config,
        notes,
        cases,
        fingerprint: eval::fingerprint(&material),
        dataset_fingerprint: eval::fingerprint(&cases_bytes),
        notes_fingerprint: eval::fingerprint(&notes_bytes),
        scorer_fingerprint: eval::fingerprint(include_str!("eval.rs").as_bytes()),
    })
}

fn validate_config(config: &Value) -> Result<(), String> {
    let temperature = config["temperature"]
        .as_f64()
        .ok_or("temperature must be numeric.")?;
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err("temperature must be between 0 and 2.".into());
    }
    for (field, low, high) in [
        ("max_tokens", 1, 4096),
        ("timeout_seconds", 1, 300),
        ("retries", 0, 3),
    ] {
        if !config[field]
            .as_u64()
            .is_some_and(|number| (low..=high).contains(&number))
        {
            return Err(format!(
                "{field} must be an integer between {low} and {high}."
            ));
        }
    }
    let profiles = config["profiles"]
        .as_array()
        .filter(|profiles| !profiles.is_empty())
        .ok_or("profiles must be a nonempty array.")?;
    let mut names = BTreeSet::new();
    for profile in profiles {
        let name = profile["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or("Every model profile needs a name.")?;
        if !names.insert(name) || name.contains(',') {
            return Err("Profile names must be unique and contain no commas.".into());
        }
        if !matches!(profile["provider"].as_str(), Some("groq" | "nvidia")) {
            return Err("Profile provider must be groq or nvidia.".into());
        }
        if !profile["model"]
            .as_str()
            .is_some_and(|model| !model.trim().is_empty() && model.len() <= 256)
        {
            return Err("Profile model must contain between 1 and 256 bytes.".into());
        }
        if !profile["min_interval_ms"]
            .as_u64()
            .is_some_and(|value| value <= 60000)
        {
            return Err("min_interval_ms must be an integer between 0 and 60000.".into());
        }
    }
    Ok(())
}

fn parse_credentials(text: &str, filename: &str) -> Result<BTreeMap<String, String>, String> {
    let mut credentials = BTreeMap::new();
    for (line_number, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("Invalid assignment at {filename} line {}.", line_number + 1))?;
        let key = key.trim();
        if !["GROQ_API_KEY", "NVIDIA_API_KEY"].contains(&key) {
            return Err(format!(
                "Unsupported key name at {filename} line {}.",
                line_number + 1
            ));
        }
        let value = value.trim();
        let value = if value.starts_with('"') || value.starts_with('\'') {
            let quote = value.chars().next().unwrap();
            let closing = value[1..].find(quote).map(|index| index + 1);
            let Some(closing) = closing else {
                return Err(format!(
                    "Unclosed quote at {filename} line {}.",
                    line_number + 1
                ));
            };
            let suffix = value[closing + 1..].trim();
            if !suffix.is_empty() && !suffix.starts_with('#') {
                return Err(format!(
                    "Invalid quoted value at {filename} line {}.",
                    line_number + 1
                ));
            }
            &value[1..closing]
        } else {
            let comment = value
                .char_indices()
                .find(|(index, character)| {
                    *character == '#'
                        && (*index == 0 || value[..*index].ends_with(char::is_whitespace))
                })
                .map(|(index, _)| index);
            &value[..comment.unwrap_or(value.len())]
        };
        if !value.trim().is_empty() {
            credentials.insert(key.to_owned(), value.trim().to_owned());
        }
    }
    Ok(credentials)
}

fn credential_file(root: &Path, filename: &str) -> Result<BTreeMap<String, String>, String> {
    let path = root.join(filename);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let metadata = fs::metadata(&path).map_err(|_| format!("Cannot inspect {filename}."))?;
    if metadata.len() > 65536 {
        return Err(format!("{filename} exceeds 64 KiB."));
    }
    let text = fs::read_to_string(path).map_err(|_| format!("Cannot read {filename} as UTF-8."))?;
    parse_credentials(&text, filename)
}

fn merge_credentials(
    base: BTreeMap<String, String>,
    local: BTreeMap<String, String>,
    process: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut credentials = BTreeMap::new();
    for (name, value) in base.into_iter().chain(local).chain(process) {
        if !value.trim().is_empty() {
            credentials.insert(name, value);
        }
    }
    credentials
}

fn load_local_credentials(root: &Path) -> Result<(), String> {
    let base = credential_file(root, ".env")?;
    let local = credential_file(root, ".env.local")?;
    let process = ["GROQ_API_KEY", "NVIDIA_API_KEY"]
        .into_iter()
        .filter_map(|name| env::var(name).ok().map(|value| (name.to_owned(), value)))
        .collect();
    for (name, value) in merge_credentials(base, local, process) {
        env::set_var(name, value);
    }
    Ok(())
}

fn settings(config: &Value, profile: &Value, approach: &str) -> core::Settings {
    core::Settings {
        provider: profile["provider"].as_str().unwrap().to_owned(),
        model: profile["model"].as_str().unwrap().to_owned(),
        approach: approach.into(),
        temperature: config["temperature"].as_f64().unwrap(),
        max_tokens: config["max_tokens"].as_u64().unwrap(),
        timeout_seconds: config["timeout_seconds"].as_u64().unwrap(),
        retries: config["retries"].as_u64().unwrap() as u32,
    }
}

fn selected_profiles(config: &Value, requested: &str) -> Result<Value, String> {
    let profiles = config["profiles"].as_array().unwrap();
    if requested == "all" {
        return Ok(json!(profiles));
    }
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    for name in requested.split(',') {
        if name.is_empty() || !seen.insert(name) {
            return Err("--models must contain unique nonempty profile aliases.".into());
        }
        let profile = profiles
            .iter()
            .find(|profile| profile["name"] == name)
            .ok_or("Unknown model profile; inspect models.json for available aliases.")?;
        selected.push(profile.clone());
    }
    Ok(json!(selected))
}

fn preflight(profiles: &Value) -> Result<(), String> {
    let mut missing = BTreeSet::new();
    for profile in profiles.as_array().unwrap() {
        let key = match profile["provider"].as_str() {
            Some("groq") => "GROQ_API_KEY",
            Some("nvidia") => "NVIDIA_API_KEY",
            _ => return Err("Unsupported provider.".into()),
        };
        if env::var(key)
            .ok()
            .is_none_or(|value| value.trim().is_empty())
        {
            missing.insert(key);
        }
    }
    if !missing.is_empty() {
        return Err(format!("Missing local credentials: {}. No API calls made. Configure .env, .env.local, or process environment locally; never paste keys into chat.", missing.into_iter().collect::<Vec<_>>().join(", ")));
    }
    Ok(())
}

fn guard_completion(completion: core::Completion) -> Result<core::Completion, String> {
    for key_name in ["GROQ_API_KEY", "NVIDIA_API_KEY"] {
        if let Ok(key) = env::var(key_name) {
            if !key.is_empty() && completion.text.contains(&key) {
                return Err(
                    "Provider output contained a local credential; response suppressed.".into(),
                );
            }
        }
    }
    Ok(completion)
}

fn unix_time() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "System clock precedes Unix epoch.".into())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|_| "Cannot encode JSON output.")?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("Cannot write {}: {error}", path.display()))
}

type RecordKey = (String, u64, String);

fn record_key(record: &Value) -> Result<RecordKey, String> {
    Ok((
        record["profile"]
            .as_str()
            .ok_or("Record has no profile.")?
            .to_owned(),
        record["run"]
            .as_u64()
            .filter(|run| *run > 0)
            .ok_or("Record has no valid run.")?,
        record["case_id"]
            .as_str()
            .ok_or("Record has no case ID.")?
            .to_owned(),
    ))
}

fn load_records(path: &Path) -> Result<BTreeMap<RecordKey, Value>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(path).map_err(|_| "Cannot read records.jsonl.")?;
    let mut latest = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(line).map_err(|_| {
            format!(
                "Invalid JSONL at line {}. Preserve and repair this checkpoint before resuming.",
                number + 1
            )
        })?;
        let key = record_key(&record)?;
        latest.insert(key, record);
    }
    if !text.is_empty() && !text.ends_with('\n') {
        return Err("records.jsonl lacks a final newline; preserve and repair the interrupted checkpoint before resuming.".into());
    }
    Ok(latest)
}

fn check_manifest(existing: &Value, expected: &Value) -> Result<(), String> {
    for field in [
        "fingerprint",
        "profiles",
        "runs",
        "case_count",
        "settings",
        "library_version",
        "schema_version",
        "approach",
        "system_prompt",
        "dataset_fingerprint",
        "notes_fingerprint",
        "scorer_fingerprint",
    ] {
        if existing[field] != expected[field] {
            return Err(format!("Resume refused: manifest {field} differs. Use a new output directory for changed experiments."));
        }
    }
    Ok(())
}

fn validate_records(
    records: &BTreeMap<RecordKey, Value>,
    manifest: &Value,
    cases: &[Value],
) -> Result<(), String> {
    for ((profile_name, run, case_id), record) in records {
        let profile = manifest["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["name"] == profile_name.as_str())
            .ok_or("Checkpoint record has an unexpected profile.")?;
        if *run > manifest["runs"].as_u64().unwrap()
            || !cases.iter().any(|case| case["id"] == case_id.as_str())
        {
            return Err("Checkpoint record has an unexpected run or case ID.".into());
        }
        if record["provider"] != profile["provider"] || record["model"] != profile["model"] {
            return Err("Checkpoint record provider/model differs from the manifest.".into());
        }
        if record.get("error").is_none() || !record["score"].is_object() {
            return Err("Checkpoint record is missing its error or score field.".into());
        }
    }
    Ok(())
}

fn golden_check(inputs: &Inputs) -> Result<(), String> {
    let mut passed = 0;
    for case in inputs.cases.as_array().unwrap() {
        let citations: Vec<_> = case["expected_citations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let source = inputs.notes["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|source| source["id"] == *id)
                    .unwrap();
                json!({"source_id":id,"quote":source["text"]})
            })
            .collect();
        let reference = json!({"status":case["expected_status"],"answer":case["expected_answer"],"citations":citations});
        let result = eval::score(case, Some(&reference), &inputs.notes);
        if result["pass"] != true {
            return Err(format!(
                "RUBRIC CHECK failed for {}: {}",
                case["id"], result["reasons"]
            ));
        }
        passed += 1;
    }
    println!(
        "RUBRIC CHECK: {passed}/{} reference answers pass. This is not a model result.",
        inputs.cases.as_array().unwrap().len()
    );
    Ok(())
}

fn markdown_block(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}text\n{text}\n{fence}\n")
}

fn suggestion(reasons: &Value) -> &'static str {
    let reasons = reasons.to_string();
    if reasons.contains("schema") || reasons.contains("JSON") {
        "Possible cause: formatting instructions were not followed or the completion was truncated. First change: inspect the raw completion and token budget, then test a common JSON output constraint supported by all compared models."
    } else if reasons.contains("decision") {
        "Possible cause: the boundary between answering, clarifying, and abstaining was unclear. First change: inspect whether the rubric decision is defensible, then make that boundary explicit in the shared prompt and rerun every model."
    } else if reasons.contains("evidence") {
        "Possible cause: a quote or citation set did not match the strict reference requirements. First change: inspect whether the model used valid alternative evidence before changing the common citation instructions or rubric."
    } else {
        "Possible cause: the response omitted a required idea, used an unlisted synonym, or triggered a context-insensitive forbidden phrase. First change: manually inspect meaning and negation before deciding whether to improve the shared prompt or document and revise the rubric."
    }
}

fn selection_markdown(summary: &Value, records: &[Value]) -> String {
    let mut text = String::from("# Model comparison\n\nResults reflect saved API calls. Pending rows are not final model scores. Cost assumes requests remain covered by the account's free quotas; billing was not independently verified. Response time includes retries and backoff but excludes intentional pacing.\n\n| Model profile | Strict pass mean | Run range | Mean response time | Estimated cost / 1,000 | Successful / expected |\n| --- | ---: | ---: | ---: | ---: | ---: |\n");
    for row in summary["models"].as_array().unwrap() {
        let latency = row["average_response_ms"]
            .as_f64()
            .map_or("not measured".into(), |value| {
                format!("{:.3} s", value / 1000.0)
            });
        let (score, range) = if row["score_is_final"] == true {
            (
                format!("{:.2}%", row["score_percent"].as_f64().unwrap()),
                format!(
                    "{:.2}% to {:.2}%",
                    row["score_min_percent"].as_f64().unwrap(),
                    row["score_max_percent"].as_f64().unwrap()
                ),
            )
        } else {
            ("pending".into(), "pending".into())
        };
        text.push_str(&format!(
            "| {} | {} | {} | {} | $0, conditional | {} / {} |\n",
            row["profile"].as_str().unwrap(),
            score,
            range,
            latency,
            row["successful_requests"],
            row["expected_requests"]
        ));
    }
    if summary["milestone_complete"] != true {
        text.push_str("\nNo milestone model selected. This experiment is incomplete, contains API failures, or uses smoke settings. A milestone comparison needs at least 50 cases, three distinct models, and three complete runs per model.\n");
        return text;
    }
    let chosen = summary["chosen_profile"].as_str().unwrap();
    text.push_str(&format!("\nSelected model profile: **{chosen}**. Selection follows highest mean strict pass rate, with lowest mean successful response time as the tie-breaker. All compared requests have the same conditional $0 estimate. Scores measure this synthetic development set, not general learning effectiveness.\n\n## Observed failures\n"));
    let mut seen = BTreeSet::new();
    let failures: Vec<_> = records
        .iter()
        .filter(|record| {
            record["profile"] == chosen
                && record["score"]["pass"] != true
                && record["error"].is_null()
        })
        .filter(|record| seen.insert(record["case_id"].as_str().unwrap().to_owned()))
        .take(3)
        .collect();
    if failures.len() < 3 {
        text.push_str(&format!("\nThis experiment contains {} distinct failed cases for the selected model; three failures cannot be supplied without inventing results. Add new held-out hard cases in a separately versioned experiment if more failure analysis is needed.\n", failures.len()));
    }
    for record in failures {
        text.push_str(&format!("\n### {} (run {})\n\nInput:\n\n{}\nExpected:\n\n{}\nActual output:\n\n{}\nScorer reasons: {}\n\n{}\n", record["case_id"].as_str().unwrap(),record["run"],
            markdown_block(record["input"].as_str().unwrap()),markdown_block(record["expected_answer"].as_str().unwrap()),
            markdown_block(record["raw_response"].as_str().unwrap_or("")),record["score"]["reasons"],suggestion(&record["score"]["reasons"])));
    }
    text
}

fn summarize_to(
    directory: &Path,
    manifest: &Value,
    latest: &BTreeMap<RecordKey, Value>,
) -> Result<Value, String> {
    let records: Vec<_> = latest.values().cloned().collect();
    let runs = manifest["runs"]
        .as_u64()
        .ok_or("Manifest has no valid run count.")? as usize;
    let case_count = manifest["case_count"]
        .as_u64()
        .ok_or("Manifest has no valid case count.")? as usize;
    if runs == 0
        || case_count == 0
        || manifest["profiles"]
            .as_array()
            .is_none_or(|profiles| profiles.is_empty())
    {
        return Err("Manifest has empty experiment dimensions.".into());
    }
    let mut summary = eval::summarize(&records, &manifest["profiles"], runs, case_count);
    let distinct_models: BTreeSet<_> = manifest["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|profile| profile["model"].as_str().unwrap_or(""))
        .collect();
    let milestone_complete =
        summary["complete"] == true && runs >= 3 && case_count >= 50 && distinct_models.len() >= 3;
    summary["milestone_complete"] = json!(milestone_complete);
    summary["experiment_fingerprint"] = manifest["fingerprint"].clone();
    if !milestone_complete {
        summary["chosen_profile"] = Value::Null;
    }
    write_json(&directory.join("summary.json"), &summary)?;
    fs::write(
        directory.join("MODEL_SELECTION.md"),
        selection_markdown(&summary, &records),
    )
    .map_err(|_| "Cannot write MODEL_SELECTION.md.")?;
    for row in summary["models"].as_array().unwrap() {
        let score = if row["score_is_final"] == true {
            format!("{:.2}% strict pass", row["score_percent"].as_f64().unwrap())
        } else {
            format!(
                "score pending ({} scheduled cases passed so far)",
                row["passed"]
            )
        };
        println!(
            "{}: {}; {}/{} successful requests; {} API errors.",
            row["profile"].as_str().unwrap(),
            score,
            row["successful_requests"],
            row["expected_requests"],
            row["api_errors"]
        );
    }
    println!(
        "milestone_complete={milestone_complete}; results: {}",
        directory.display()
    );
    Ok(summary)
}

fn evaluate(root: &Path, inputs: &Inputs, options: &Options) -> Result<(), String> {
    let profiles = selected_profiles(&inputs.config, &options.models)?;
    preflight(&profiles)?;
    let all_cases = inputs.cases.as_array().unwrap();
    let count = options.limit.unwrap_or(all_cases.len());
    if count > all_cases.len() {
        return Err(format!("--limit must be at most {}.", all_cases.len()));
    }
    let cases = &all_cases[..count];
    let timestamp = unix_time()?;
    let manifest = json!({"schema_version":2,"library_version":env!("CARGO_PKG_VERSION"),
        "fingerprint":inputs.fingerprint,"profiles":profiles,"runs":options.runs,"case_count":count,
        "settings":inputs.config,"created_unix_seconds":timestamp,
        "approach":options.approach,"system_prompt":core::prompt_fingerprint_material(&options.approach)?,
        "dataset_fingerprint":inputs.dataset_fingerprint,"notes_fingerprint":inputs.notes_fingerprint,
        "scorer_fingerprint":inputs.scorer_fingerprint});
    let directory = if let Some(directory) = &options.resume {
        let (_, existing) = read_json(&directory.join("manifest.json"))?;
        check_manifest(&existing, &manifest)?;
        directory.clone()
    } else {
        let directory = options.out.clone().unwrap_or_else(|| {
            root.join("results")
                .join(format!("run-{timestamp}-{}", process::id()))
        });
        if directory.exists() {
            return Err(
                "Output path already exists. Choose a new --out directory or use --resume.".into(),
            );
        }
        if let Some(parent) = directory
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|_| "Cannot create output parent directory.")?;
        }
        fs::create_dir(&directory).map_err(|_| "Cannot create new output directory.")?;
        write_json(&directory.join("manifest.json"), &manifest)?;
        directory
    };
    println!("Experiment directory: {}", directory.display());
    let path = directory.join("records.jsonl");
    let mut latest = load_records(&path)?;
    validate_records(&latest, &manifest, cases)?;
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| "Cannot open records.jsonl for append.")?;
    let mut last_calls: BTreeMap<(String, String), Instant> = BTreeMap::new();
    for run in 1..=options.runs {
        for case in cases {
            for profile in profiles.as_array().unwrap() {
                let name = profile["name"].as_str().unwrap();
                let case_id = case["id"].as_str().unwrap();
                let key = (name.to_owned(), run as u64, case_id.to_owned());
                if latest
                    .get(&key)
                    .is_some_and(|record| record["error"].is_null())
                {
                    continue;
                }
                let config = settings(&inputs.config, profile, &options.approach);
                let call_key = (config.provider.clone(), config.model.clone());
                let interval =
                    std::time::Duration::from_millis(profile["min_interval_ms"].as_u64().unwrap());
                if let Some(last) = last_calls.get(&call_key) {
                    if let Some(remaining) = interval.checked_sub(last.elapsed()) {
                        thread::sleep(remaining);
                    }
                }
                last_calls.insert(call_key, Instant::now());
                let result = core::answer(case["input"].as_str().unwrap(), &inputs.notes, &config)
                    .and_then(guard_completion);
                let mut record = json!({"profile":name,"provider":profile["provider"],"model":profile["model"],"approach":options.approach,
                    "run":run,"case_id":case_id,"category":case["category"],"input":case["input"],
                    "expected_answer":case["expected_answer"],"recorded_unix_seconds":unix_time()?,
                    "raw_response":null,"parsed":null,"latency_ms":null,"input_tokens":null,"output_tokens":null,
                    "attempts":null,"error":null});
                let failure = match result {
                    Ok(completion) => {
                        record["raw_response"] = json!(completion.text);
                        record["parsed"] = completion.parsed.unwrap_or(Value::Null);
                        record["latency_ms"] = json!(completion.latency_ms);
                        record["input_tokens"] = json!(completion.input_tokens);
                        record["output_tokens"] = json!(completion.output_tokens);
                        record["attempts"] = json!(completion.attempts);
                        record["score"] = eval::score(case, Some(&record["parsed"]), &inputs.notes);
                        None
                    }
                    Err(error) => {
                        record["error"] = json!(error);
                        record["score"] = eval::score(case, None, &inputs.notes);
                        Some(error)
                    }
                };
                let line =
                    serde_json::to_string(&record).map_err(|_| "Cannot encode result record.")?;
                writeln!(output, "{line}")
                    .and_then(|_| output.flush())
                    .and_then(|_| output.sync_data())
                    .map_err(|_| {
                        "Cannot persist evaluation checkpoint; stopping before further API calls."
                    })?;
                println!(
                    "{name} run {run} {case_id}: {}",
                    if failure.is_some() {
                        "API ERROR"
                    } else if record["score"]["pass"] == true {
                        "PASS"
                    } else {
                        "FAIL"
                    }
                );
                latest.insert(key, record);
                if let Some(error) = failure {
                    summarize_to(&directory, &manifest, &latest)?;
                    return Err(format!("Evaluation stopped after provider error: {error} Checkpoint saved; fix access or quota and use --resume with identical options."));
                }
            }
        }
    }
    let summary = summarize_to(&directory, &manifest, &latest)?;
    if summary["complete"] != true {
        return Err("Evaluation remains incomplete; inspect summary.json.".into());
    }
    Ok(())
}

fn run(options: Options) -> Result<(), String> {
    if options.command == "help" {
        println!("{HELP}");
        return Ok(());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let inputs = load_inputs(&root, &options.approach)?;
    if options.command == "summarize" {
        let directory = options.out.as_ref().unwrap();
        let (_, manifest) = read_json(&directory.join("manifest.json"))?;
        let runs = manifest["runs"]
            .as_u64()
            .filter(|runs| (1..=10).contains(runs))
            .ok_or("Saved experiment has an invalid run count.")?;
        let count = manifest["case_count"]
            .as_u64()
            .filter(|count| *count > 0 && *count <= inputs.cases.as_array().unwrap().len() as u64)
            .ok_or("Saved experiment has an invalid case count.")? as usize;
        let names = manifest["profiles"]
            .as_array()
            .filter(|profiles| !profiles.is_empty())
            .ok_or("Saved experiment has no model profiles.")?
            .iter()
            .map(|profile| profile["name"].as_str().ok_or("Saved profile has no name."))
            .collect::<Result<Vec<_>, _>>()?
            .join(",");
        let profiles = selected_profiles(&inputs.config, &names)?;
        if manifest["profiles"] != profiles {
            return Err("Saved experiment profiles differ from configured models.".into());
        }
        if !matches!(manifest["schema_version"].as_u64(), Some(1 | 2)) {
            return Err("Saved experiment schema is unsupported.".into());
        }
        let latest = load_records(&directory.join("records.jsonl"))?;
        validate_records(
            &latest,
            &manifest,
            &inputs.cases.as_array().unwrap()[..count],
        )?;
        let records: Vec<_> = latest.values().cloned().collect();
        let mut summary = eval::summarize(&records, &manifest["profiles"], runs as usize, count);
        summary["experiment_fingerprint"] = manifest["fingerprint"].clone();
        summary["approach"] = manifest
            .get("approach")
            .cloned()
            .unwrap_or(json!("baseline"));
        println!("Read-only summary from saved provenance; current prompt/code fingerprint was not compared. No files were written.");
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        return if summary["complete"] == true {
            Ok(())
        } else {
            Err("Experiment is incomplete; no milestone selection made.".into())
        };
    }
    match options.command.as_str() {
        "validate" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&eval::validate(&inputs.notes, &inputs.cases)?)
                    .unwrap()
            );
            golden_check(&inputs)?;
            println!("Dataset/prompt/code fingerprint: {}", inputs.fingerprint);
        }
        "doctor" => {
            load_local_credentials(&root)?;
            println!("Credential precedence: nonempty process environment, .env.local, then .env.");
            println!(
                "Dataset: {} cases; {} sources.",
                inputs.cases.as_array().unwrap().len(),
                inputs.notes["sources"].as_array().unwrap().len()
            );
            let executable = if cfg!(windows) { "curl.exe" } else { "curl" };
            let curl = Command::new(executable)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            println!("curl available: {curl}");
            for key in ["GROQ_API_KEY", "NVIDIA_API_KEY"] {
                println!(
                    "{key} configured: {}",
                    env::var(key).is_ok_and(|value| !value.trim().is_empty())
                );
            }
            if !curl {
                return Err("curl is required for HTTPS requests.".into());
            }
            preflight(&inputs.config["profiles"])?;
            println!("Local prerequisites present. Provider authentication/free account status still needs a live request.");
        }
        "answer" => {
            load_local_credentials(&root)?;
            let profile = if let (Some(provider), Some(model_id)) =
                (&options.provider, &options.model_id)
            {
                if !["groq", "nvidia"].contains(&provider.as_str()) {
                    return Err("--provider must be groq or nvidia.".into());
                }
                json!({"name":"custom","provider":provider,"model":model_id,"min_interval_ms":0})
            } else {
                selected_profiles(&inputs.config, &options.model)?[0].clone()
            };
            preflight(&json!([profile]))?;
            let completion = core::answer(
                options.question.as_deref().unwrap(),
                &inputs.notes,
                &settings(&inputs.config, &profile, &options.approach),
            )
            .and_then(guard_completion)?;
            println!("{}",serde_json::to_string_pretty(&json!({"model":profile["model"],"provider":profile["provider"],"output":completion.parsed,"raw_response":completion.text,"latency_ms":completion.latency_ms,"input_tokens":completion.input_tokens,"output_tokens":completion.output_tokens,"attempts":completion.attempts})).unwrap());
        }
        "evaluate" => {
            load_local_credentials(&root)?;
            evaluate(&root, &inputs, &options)?;
        }
        _ => return Err("Unknown command.".into()),
    }
    Ok(())
}

fn main() {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if let Err(error) = parse_options(&arguments).and_then(run) {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn dotenv_parser_supports_bom_quotes_comments_and_blank_placeholders() {
        let values = parse_credentials("\u{feff}# Local keys\r\nGROQ_API_KEY=\"fake-groq\" # comment\r\nNVIDIA_API_KEY='fake-nvidia'\n", ".env").unwrap();
        assert_eq!(values["GROQ_API_KEY"], "fake-groq");
        assert_eq!(values["NVIDIA_API_KEY"], "fake-nvidia");
        assert!(
            parse_credentials("GROQ_API_KEY= \nNVIDIA_API_KEY=''\n", ".env")
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            parse_credentials("GROQ_API_KEY=fake # ignored\n", ".env").unwrap()["GROQ_API_KEY"],
            "fake"
        );
    }

    #[test]
    fn dotenv_errors_include_location_without_credential_values() {
        for input in [
            "GROQ_API_KEY=\"fake-sensitive",
            "GROQ_API_KEY=\"fake-sensitive\" trailing",
            "OTHER_KEY=fake-sensitive",
            "fake-sensitive",
        ] {
            let error = parse_credentials(input, ".env.local").unwrap_err();
            assert!(error.contains(".env.local line 1"));
            assert!(!error.contains("fake-sensitive"));
        }
    }

    #[test]
    fn dotenv_precedence_keeps_process_then_local_then_base_nonempty_values() {
        let base = parse_credentials(
            "GROQ_API_KEY=base-groq\nNVIDIA_API_KEY=base-nvidia\n",
            ".env",
        )
        .unwrap();
        let local =
            parse_credentials("GROQ_API_KEY=local-groq\nNVIDIA_API_KEY=\n", ".env.local").unwrap();
        let process = BTreeMap::from([
            ("GROQ_API_KEY".to_owned(), "process-groq".to_owned()),
            ("NVIDIA_API_KEY".to_owned(), " ".to_owned()),
        ]);
        let selected = merge_credentials(base.clone(), local.clone(), process);
        assert_eq!(selected["GROQ_API_KEY"], "process-groq");
        assert_eq!(selected["NVIDIA_API_KEY"], "base-nvidia");
        assert_eq!(
            merge_credentials(base, local, BTreeMap::new())["GROQ_API_KEY"],
            "local-groq"
        );
    }

    #[test]
    fn standard_dotenv_file_is_read_without_requiring_dotenv_local() {
        let directory = env::temp_dir().join(format!(
            "study-coach-dotenv-{}-{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        fs::write(
            directory.join(".env"),
            "GROQ_API_KEY=fake-file-groq\nNVIDIA_API_KEY=fake-file-nvidia\n",
        )
        .unwrap();
        let base = credential_file(&directory, ".env").unwrap();
        let local = credential_file(&directory, ".env.local").unwrap();
        assert_eq!(
            merge_credentials(base, local, BTreeMap::new())["GROQ_API_KEY"],
            "fake-file-groq"
        );
        fs::remove_file(directory.join(".env")).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn defaults_cover_graduate_experiment() {
        let options = parse_options(&args(&["evaluate"])).unwrap();
        assert_eq!(options.models, "all");
        assert_eq!(options.runs, 3);
        assert_eq!(options.approach, "grounded");
        assert_eq!(options.model, "groq-120b");
        assert!(options.limit.is_none());
    }

    #[test]
    fn rejects_missing_unknown_duplicate_and_out_of_range_flags() {
        for arguments in [
            vec!["answer", "--question"],
            vec!["evaluate", "--runs", "0"],
            vec!["evaluate", "--runs", "11"],
            vec!["evaluate", "--runs", "1", "--runs", "2"],
            vec!["validate", "--runs", "3"],
            vec!["evaluate", "--out", "a", "--resume", "b"],
            vec!["answer", "--provider", "groq", "--question", "x"],
            vec!["summarize"],
            vec!["evaluate", "--limit", "0"],
            vec!["evaluate", "--approach", "unknown"],
        ] {
            assert!(
                parse_options(&args(&arguments)).is_err(),
                "accepted {arguments:?}"
            );
        }
    }

    #[test]
    fn empty_question_and_custom_model_are_explicitly_supported() {
        let options = parse_options(&args(&[
            "answer",
            "--provider",
            "groq",
            "--model-id",
            "custom",
            "--question",
            "",
        ]))
        .unwrap();
        assert_eq!(options.question.as_deref(), Some(""));
        assert_eq!(options.model_id.as_deref(), Some("custom"));
    }

    #[test]
    fn resume_rejects_changed_settings_or_fingerprint() {
        let manifest = json!({"fingerprint":"one","profiles":[],"runs":3,"case_count":50,"settings":{"temperature":0.2},"library_version":"0.1.0","schema_version":1});
        assert!(check_manifest(&manifest, &manifest).is_ok());
        let mut changed = manifest.clone();
        changed["fingerprint"] = json!("two");
        assert!(check_manifest(&manifest, &changed).is_err());
        changed = manifest.clone();
        changed["runs"] = json!(1);
        assert!(check_manifest(&manifest, &changed).is_err());
        changed = manifest.clone();
        changed["approach"] = json!("grounded");
        assert!(check_manifest(&manifest, &changed).is_err());
        changed = manifest.clone();
        changed["system_prompt"] = json!("different instructions");
        assert!(check_manifest(&manifest, &changed).is_err());
    }

    #[test]
    fn model_aliases_must_be_unique_and_known() {
        let config = json!({"profiles":[{"name":"a"},{"name":"b"}]});
        assert_eq!(selected_profiles(&config, "b,a").unwrap()[0]["name"], "b");
        assert!(selected_profiles(&config, "a,a").is_err());
        assert!(selected_profiles(&config, "missing").is_err());
        assert!(selected_profiles(&config, "a,").is_err());
    }

    #[test]
    fn markdown_fence_cannot_be_closed_by_response_text() {
        let text = "```\nUntrusted response\n```";
        assert!(markdown_block(text).starts_with("````text\n"));
    }

    #[test]
    fn checkpoint_latest_record_wins_and_history_is_untouched() {
        let path = env::temp_dir().join(format!(
            "study-coach-records-{}-{}.jsonl",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = json!({"profile":"p","run":1,"case_id":"E01","error":"quota"});
        let second = json!({"profile":"p","run":1,"case_id":"E01","error":null});
        let history = format!("{first}\n{second}\n");
        fs::write(&path, &history).unwrap();
        let records = load_records(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records.values().next().unwrap()["error"].is_null());
        assert_eq!(fs::read_to_string(&path).unwrap(), history);
        fs::remove_file(path).unwrap();
    }
}
