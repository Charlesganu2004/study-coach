//! Synthetic checkpoint fixtures test rejection paths, never model quality.
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn summarize_refuses_records_from_wrong_runs_before_writing_selection() {
    let executable = env!("CARGO_BIN_EXE_study-coach");
    let validate = Command::new(executable).arg("validate").output().unwrap();
    assert!(validate.status.success());
    let output = String::from_utf8(validate.stdout).unwrap();
    let fingerprint = output
        .lines()
        .find_map(|line| line.strip_prefix("Dataset/prompt/code fingerprint: "))
        .unwrap();
    let config: Value = serde_json::from_str(include_str!("../models.json")).unwrap();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "study-coach-summary-test-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let manifest = json!({"schema_version":1,"library_version":env!("CARGO_PKG_VERSION"),
        "fingerprint":fingerprint,"profiles":config["profiles"],"settings":config,"runs":3,"case_count":50});
    fs::write(directory.join("manifest.json"), manifest.to_string()).unwrap();
    let mut records = String::new();
    for profile in config["profiles"].as_array().unwrap() {
        for run in 4..=6 {
            for case in 1..=50 {
                let record = json!({"profile":profile["name"],"provider":profile["provider"],"model":profile["model"],
                    "run":run,"case_id":format!("E{case:02}"),"error":null,"score":{"pass":true},"latency_ms":1});
                records.push_str(&record.to_string());
                records.push('\n');
            }
        }
    }
    fs::write(directory.join("records.jsonl"), records).unwrap();
    let result = Command::new(executable)
        .args(["summarize", "--out"])
        .arg(&directory)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unexpected run or case ID"));
    assert!(!directory.join("MODEL_SELECTION.md").exists());
    fs::remove_dir_all(directory).unwrap();
}
