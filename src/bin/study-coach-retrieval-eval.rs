//! Controlled retrieval arm. Inference uses the same grounded core as the app;
//! reference answers enter only the scorer and coverage analysis afterward.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use study_coach::{app, core, eval, retrieval};

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|_| "Cannot read an evaluation input file.")?;
    let value =
        serde_json::from_slice(&bytes).map_err(|_| "An evaluation input is not valid JSON.")?;
    Ok((bytes, value))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(value).map_err(|_| "Cannot encode evaluation output.")?;
    fs::write(path, text + "\n").map_err(|_| "Cannot save evaluation output.".into())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn summarize(
    directory: &Path,
    latest: &BTreeMap<String, Value>,
    profile: &Value,
) -> Result<Value, String> {
    let records: Vec<_> = latest.values().cloned().collect();
    let mut summary = eval::summarize(&records, &json!([profile]), 1, 50);
    let answer_records: Vec<_> = records
        .iter()
        .filter(|record| record["expected_citations_count"].as_u64().unwrap_or(0) > 0)
        .collect();
    let covered = answer_records
        .iter()
        .filter(|record| record["all_expected_citations_retrieved"] == true)
        .count();
    let expected: u64 = answer_records
        .iter()
        .filter_map(|record| record["expected_citations_count"].as_u64())
        .sum();
    let retrieved: u64 = answer_records
        .iter()
        .filter_map(|record| record["expected_citations_retrieved"].as_u64())
        .sum();
    summary["context_mode"] = json!("lexical_top4");
    summary["approach"] = json!("grounded");
    for (field, target) in [
        ("retrieval_ms", "average_retrieval_ms"),
        ("end_to_end_latency_ms", "average_end_to_end_response_ms"),
    ] {
        let values: Vec<_> = records
            .iter()
            .filter_map(|record| record[field].as_f64())
            .collect();
        summary[target] = if values.is_empty() {
            Value::Null
        } else {
            json!(values.iter().sum::<f64>() / values.len() as f64)
        };
    }
    summary["retrieval"] = json!({"top_k":retrieval::TOP_K,"answer_cases_recorded":answer_records.len(),
        "answer_cases_with_all_expected_sources":covered,"expected_source_references":expected,
        "retrieved_expected_source_references":retrieved,
        "expected_reference_recall_percent":if expected>0 {json!(100.0*retrieved as f64/expected as f64)}else{Value::Null},
        "average_selected_sources":if records.is_empty(){Value::Null}else{json!(records.iter().map(|record|record["selected_source_ids"].as_array().unwrap().len()).sum::<usize>() as f64/records.len() as f64)}});
    summary["limitation"] = json!("One run on a visible development set; retrieval coverage is diagnosed with reference citations after selection and is not an inference input.");
    write_json(&directory.join("summary.json"), &summary)?;
    Ok(summary)
}

fn run() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut output = root.join("results/milestone2-retrieval-2026-11-08");
    let mut check_only = false;
    let mut arguments = env::args().skip(1);
    let mut seen = BTreeSet::new();
    while let Some(argument) = arguments.next() {
        if !seen.insert(argument.clone()) {
            return Err("Do not repeat options.".into());
        }
        match argument.as_str() {
            "--out" => output = PathBuf::from(arguments.next().ok_or("--out needs a path.")?),
            "--check-only" => check_only = true,
            "--help" | "-h" => {
                println!("Usage: study-coach-retrieval-eval [--out PATH] [--check-only]\nRuns or resumes 50 cases with the grounded Groq120B core and lexical top four retrieval. Check-only performs no API calls.");
                return Ok(());
            }
            _ => return Err("Unknown option. Use --help.".into()),
        }
    }
    let (config_bytes, config) = read_json(&root.join("models.json"))?;
    let (notes_bytes, notes) = read_json(&root.join("data/notes.json"))?;
    let (case_bytes, cases) = read_json(&root.join("data/eval.json"))?;
    eval::validate(&notes, &cases)?;
    let cases = cases.as_array().ok_or("Cases must be an array.")?;
    if cases.len() != 50 {
        return Err("This controlled study requires the frozen 50 cases.".into());
    }
    let profile = config["profiles"]
        .as_array()
        .ok_or("Missing profiles.")?
        .iter()
        .find(|profile| profile["name"] == "groq-120b")
        .ok_or("Selected model profile is missing.")?
        .clone();
    let settings = core::Settings {
        provider: profile["provider"]
            .as_str()
            .ok_or("Invalid provider.")?
            .into(),
        model: profile["model"].as_str().ok_or("Invalid model.")?.into(),
        approach: "grounded".into(),
        temperature: config["temperature"]
            .as_f64()
            .ok_or("Invalid temperature.")?,
        max_tokens: config["max_tokens"]
            .as_u64()
            .ok_or("Invalid output allocation.")?,
        timeout_seconds: config["timeout_seconds"]
            .as_u64()
            .ok_or("Invalid timeout.")?,
        retries: config["retries"]
            .as_u64()
            .filter(|value| *value <= 5)
            .ok_or("Invalid retries.")? as u32,
    };
    if settings.provider != "groq" || settings.model != "openai/gpt-oss-120b" {
        return Err("This study requires the selected Groq 120B model.".into());
    }
    let prompt = core::prompt_fingerprint_material("grounded")?;
    let mut material = Vec::new();
    for component in [
        &config_bytes[..],
        &notes_bytes[..],
        &case_bytes[..],
        prompt.as_bytes(),
        include_str!("../core.rs").as_bytes(),
        include_str!("../eval.rs").as_bytes(),
        include_str!("../retrieval.rs").as_bytes(),
        include_str!("study-coach-retrieval-eval.rs").as_bytes(),
    ] {
        material.extend_from_slice(&(component.len() as u64).to_le_bytes());
        material.extend_from_slice(component);
    }
    let manifest = json!({"schema_version":1,"library_version":env!("CARGO_PKG_VERSION"),"fingerprint":eval::fingerprint(&material),
        "dataset_fingerprint":eval::fingerprint(&case_bytes),"notes_fingerprint":eval::fingerprint(&notes_bytes),
        "scorer_fingerprint":eval::fingerprint(include_str!("../eval.rs").as_bytes()),
        "retriever_fingerprint":eval::fingerprint(include_str!("../retrieval.rs").as_bytes()),
        "approach":"grounded","context_mode":"lexical_top4","top_k":retrieval::TOP_K,"case_count":50,"runs":1,
        "profiles":[profile],"settings":config,"system_prompt":prompt,
        "retrieval_rule":"Distinct ASCII alphanumeric terms, stop words excluded; title overlap weighted 3 and source text overlap weighted 1; stable source order for ties; top four positive matches; no matches use full corpus"});
    let mut answer_cases = 0;
    let mut covered = 0;
    for case in cases {
        let selected = retrieval::select_sources(case["input"].as_str().unwrap(), &notes)?;
        let ids: BTreeSet<_> = selected["sources"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|source| source["id"].as_str())
            .collect();
        let expected = case["expected_citations"].as_array().unwrap();
        if !expected.is_empty() {
            answer_cases += 1;
            covered += usize::from(expected.iter().all(|id| ids.contains(id.as_str().unwrap())));
        }
        let gold = json!({"status":case["expected_status"],"answer":case["expected_answer"],"citations":expected.iter().map(|id| {
            let source=notes["sources"].as_array().unwrap().iter().find(|source|source["id"]==*id).unwrap();
            json!({"source_id":id,"quote":source["text"]})}).collect::<Vec<_>>()});
        if eval::score(case, Some(&gold), &notes)["pass"] != true {
            return Err("A reference answer fails its frozen rubric.".into());
        }
    }
    println!("Offline retrieval coverage: {covered}/{answer_cases} answer cases contain all reference source IDs. Reference checks: 50/50.");
    if check_only {
        println!("Check-only: no API calls or evidence writes.");
        return Ok(());
    }
    for file in [".env.local", ".env"] {
        let path = root.join(file);
        if path.exists() {
            app::configure_process_credentials(&path)?;
        }
    }
    if !env::var("GROQ_API_KEY").is_ok_and(|value| !value.trim().is_empty()) {
        return Err("Configure GROQ_API_KEY locally before running the study.".into());
    }
    let mut latest = BTreeMap::new();
    let manifest_path = output.join("manifest.json");
    if manifest_path.exists() {
        if read_json(&manifest_path)?.1 != manifest {
            return Err("Saved retrieval provenance differs; use a new output directory.".into());
        }
    } else {
        if output.join("records.jsonl").exists() {
            return Err("Existing ledger has no manifest; do not overwrite it.".into());
        }
        fs::create_dir_all(&output).map_err(|_| "Cannot create result directory.")?;
        write_json(&manifest_path, &manifest)?;
    }
    let path = output.join("records.jsonl");
    if path.exists() {
        let file = fs::File::open(&path).map_err(|_| "Cannot open saved ledger.")?;
        for line in BufReader::new(file).lines() {
            let value: Value =
                serde_json::from_str(&line.map_err(|_| "Cannot read saved ledger.")?)
                    .map_err(|_| "Saved ledger has invalid JSON.")?;
            let id = value["case_id"]
                .as_str()
                .ok_or("Saved ledger case ID missing.")?
                .to_owned();
            if value["profile"] != "groq-120b"
                || value["run"] != 1
                || !cases.iter().any(|case| case["id"] == id)
                || value.get("error").is_none()
            {
                return Err("Saved ledger contains an unexpected case or run.".into());
            }
            latest.insert(id, value);
        }
    }
    let mut ledger = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| "Cannot append result ledger.")?;
    let interval = Duration::from_millis(
        profile["min_interval_ms"]
            .as_u64()
            .ok_or("Invalid pacing interval.")?,
    );
    let mut last_call: Option<Instant> = None;
    for case in cases {
        let id = case["id"].as_str().unwrap();
        if latest
            .get(id)
            .is_some_and(|record| record["error"].is_null())
        {
            continue;
        }
        let retrieval_started = Instant::now();
        let selected = retrieval::select_sources(case["input"].as_str().unwrap(), &notes)?;
        let retrieval_ms = retrieval_started.elapsed().as_secs_f64() * 1000.0;
        let ids: Vec<_> = selected["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source["id"].clone())
            .collect();
        let expected = case["expected_citations"].as_array().unwrap();
        let found = expected.iter().filter(|id| ids.contains(id)).count();
        if let Some(last) = last_call {
            if let Some(wait) = interval.checked_sub(last.elapsed()) {
                thread::sleep(wait);
            }
        }
        last_call = Some(Instant::now());
        let response=core::answer(case["input"].as_str().unwrap(),&selected,&settings).and_then(|completion|{
            for key in ["GROQ_API_KEY","NVIDIA_API_KEY"] { if let Ok(value)=env::var(key) { if !value.is_empty() && (completion.text.contains(&value)||completion.parsed.as_ref().is_some_and(|parsed|parsed.to_string().contains(&value))) { return Err("Provider output contained a local credential; response suppressed.".into()); } } }
            Ok(completion)
        });
        let mut record = json!({"profile":"groq-120b","provider":settings.provider,"model":settings.model,"run":1,"case_id":id,
            "category":case["category"],"input":case["input"],"expected_answer":case["expected_answer"],"approach":"grounded",
            "context_mode":"lexical_top4","selected_source_ids":ids,"expected_citations_count":expected.len(),"expected_citations_retrieved":found,
            "all_expected_citations_retrieved":if expected.is_empty(){Value::Null}else{json!(found==expected.len())},
            "recorded_unix_seconds":now(),"retrieval_ms":retrieval_ms,"end_to_end_latency_ms":null,
            "raw_response":null,"parsed":null,"latency_ms":null,"input_tokens":null,"output_tokens":null,"attempts":null,"error":null});
        let error = match response {
            Ok(completion) => {
                record["raw_response"] = json!(completion.text);
                record["parsed"] = completion.parsed.unwrap_or(Value::Null);
                record["latency_ms"] = json!(completion.latency_ms);
                record["end_to_end_latency_ms"] = json!(completion.latency_ms + retrieval_ms);
                record["input_tokens"] = json!(completion.input_tokens);
                record["output_tokens"] = json!(completion.output_tokens);
                record["attempts"] = json!(completion.attempts);
                None
            }
            Err(error) => {
                record["error"] = json!(error);
                Some(error)
            }
        };
        record["score"] = eval::score(
            case,
            record["parsed"].as_object().map(|_| &record["parsed"]),
            &notes,
        );
        writeln!(ledger, "{}", record)
            .and_then(|_| ledger.flush())
            .and_then(|_| ledger.sync_data())
            .map_err(|_| "Cannot persist checkpoint; stopping before another call.")?;
        println!(
            "retrieval {id}: {}",
            if error.is_some() {
                "API ERROR"
            } else if record["score"]["pass"] == true {
                "PASS"
            } else {
                "FAIL"
            }
        );
        latest.insert(id.to_owned(), record);
        if let Some(error) = error {
            summarize(&output, &latest, &profile)?;
            return Err(format!("Provider error: {error} Checkpoint saved. Resume unchanged after resolving free access or quota."));
        }
    }
    let summary = summarize(&output, &latest, &profile)?;
    if summary["complete"] != true {
        return Err("Retrieval study is incomplete.".into());
    }
    println!(
        "Retrieval study complete: {}/50 passes; {:.2}%; mean {:.2} ms.",
        summary["models"][0]["passed"],
        summary["models"][0]["score_percent"].as_f64().unwrap(),
        summary["models"][0]["average_response_ms"]
            .as_f64()
            .unwrap()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Retrieval evaluation: {error}");
        std::process::exit(1);
    }
}
