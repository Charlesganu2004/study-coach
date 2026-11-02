//! Deterministic development-set rubric. Gold answers never enter model prompts.
//! Phrase checks cannot establish semantic entailment or replace a human audit.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string field: {key}"))
}

pub fn normalize(value: &str) -> String {
    let characters: Vec<_> = value.to_lowercase().chars().collect();
    let mut cleaned = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let previous_digit = index > 0 && characters[index - 1].is_ascii_digit();
        let next_digit = characters.get(index + 1).is_some_and(char::is_ascii_digit);
        let numeric_punctuation =
            matches!(character, '.' | ',' | '/') && previous_digit && next_digit;
        let numeric_suffix = character == '%' && previous_digit;
        let numeric_prefix = matches!(character, '$' | '-')
            && (next_digit
                || (characters.get(index + 1) == Some(&'$')
                    && characters.get(index + 2).is_some_and(char::is_ascii_digit)));
        cleaned.push(
            if character.is_alphanumeric()
                || numeric_punctuation
                || numeric_suffix
                || numeric_prefix
            {
                character
            } else {
                ' '
            },
        );
    }
    cleaned
        .split_whitespace()
        .map(|token| {
            let (prefix, number) = if let Some(number) = token.strip_prefix("-$") {
                ("-$", number)
            } else if let Some(number) = token.strip_prefix('$') {
                ("$", number)
            } else {
                ("", token)
            };
            let percent = number.ends_with('%');
            let number = number.trim_end_matches('%').replace(',', "");
            if !number.contains('/') {
                if let Ok(number) = number.parse::<f64>() {
                    if number.is_finite() {
                        return format!("{prefix}{number}{}", if percent { "%" } else { "" });
                    }
                }
            }
            token.to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn phrase_present(answer: &str, phrase: &str) -> bool {
    let phrase = normalize(phrase);
    !phrase.is_empty() && format!(" {answer} ").contains(&format!(" {phrase} "))
}

pub fn validate(notes: &Value, cases: &Value) -> Result<Value, String> {
    let sources = notes["sources"]
        .as_array()
        .ok_or("sources must be an array")?;
    if sources.is_empty() {
        return Err("notes must contain sources".into());
    }
    let mut ids = BTreeSet::new();
    for source in sources {
        let id = text(source, "id")?;
        if id.trim().is_empty() || !ids.insert(id) {
            return Err("source IDs must be nonempty and unique".into());
        }
        for field in ["title", "text"] {
            if text(source, field)?.trim().is_empty() {
                return Err(format!("{id}: empty {field}"));
            }
        }
    }
    let cases = cases.as_array().ok_or("evaluation set must be an array")?;
    if cases.len() < 50 {
        return Err("at least 50 cases required for this graduate-ready evaluation".into());
    }
    let mut case_ids = BTreeSet::new();
    let mut categories = BTreeMap::<String, usize>::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    for case in cases {
        let id = text(case, "id")?;
        if id.trim().is_empty() || !case_ids.insert(id) {
            return Err("case IDs must be nonempty and unique".into());
        }
        text(case, "input")?; // Empty input is a deliberate edge case.
        if text(case, "expected_answer")?.trim().is_empty() {
            return Err(format!("{id}: empty expected answer"));
        }
        let status = text(case, "expected_status")?;
        if !["answer", "clarify", "abstain"].contains(&status) {
            return Err(format!("{id}: invalid expected status"));
        }
        let category = text(case, "category")?;
        if category.trim().is_empty() {
            return Err(format!("{id}: empty category"));
        }
        let concepts = case["required_concepts"]
            .as_array()
            .ok_or("concepts must be an array")?;
        if concepts.is_empty() {
            return Err(format!("{id}: at least one concept group is required"));
        }
        for group in concepts {
            let variants = group.as_array().ok_or("concept group must be an array")?;
            if variants.is_empty()
                || variants
                    .iter()
                    .any(|v| v.as_str().is_none_or(|s| normalize(s).is_empty()))
            {
                return Err(format!("{id}: concept variants must be nonempty strings"));
            }
        }
        for forbidden in case["forbidden_phrases"]
            .as_array()
            .ok_or("forbidden phrases must be an array")?
        {
            if forbidden.as_str().is_none_or(|s| normalize(s).is_empty()) {
                return Err(format!("{id}: invalid forbidden phrase"));
            }
        }
        let citations = case["expected_citations"]
            .as_array()
            .ok_or("expected citations must be an array")?;
        let mut seen = BTreeSet::new();
        for citation in citations {
            let citation = citation.as_str().ok_or("citation IDs must be strings")?;
            if !ids.contains(citation) || !seen.insert(citation) {
                return Err(format!("{id}: unknown or repeated expected citation"));
            }
        }
        if (status == "answer") == citations.is_empty() {
            return Err(format!(
                "{id}: answers need citations; clarify/abstain need none"
            ));
        }
        *categories.entry(category.to_owned()).or_default() += 1;
        *statuses.entry(status.to_owned()).or_default() += 1;
    }
    Ok(
        json!({"cases":cases.len(), "sources":sources.len(), "categories":categories, "statuses":statuses}),
    )
}

pub fn score(case: &Value, parsed: Option<&Value>, notes: &Value) -> Value {
    let Some(output) = parsed else {
        return json!({"pass":false,"schema":false,"status":false,"concepts":false,"evidence":false,
            "matched_concepts":0,"total_concepts":case["required_concepts"].as_array().map_or(0,Vec::len),
            "reasons":["response is not a JSON object"]});
    };
    let answer = output["answer"].as_str().unwrap_or("");
    let status_value = output["status"].as_str().unwrap_or("");
    let citations = output["citations"].as_array();
    let schema = output.is_object()
        && !answer.trim().is_empty()
        && ["answer", "clarify", "abstain"].contains(&status_value)
        && citations.is_some()
        && citations.is_some_and(|items| {
            items.iter().all(|item| {
                item["source_id"].as_str().is_some() && item["quote"].as_str().is_some()
            })
        });
    let status = output["status"] == case["expected_status"];
    let normalized = normalize(answer);
    let groups = case["required_concepts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let matched = groups
        .iter()
        .filter(|group| {
            group.as_array().is_some_and(|variants| {
                variants.iter().any(|variant| {
                    variant
                        .as_str()
                        .is_some_and(|phrase| phrase_present(&normalized, phrase))
                })
            })
        })
        .count();
    let forbidden = case["forbidden_phrases"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item.as_str()
                .is_some_and(|phrase| phrase_present(&normalized, phrase))
        })
    });
    let concepts = matched == groups.len() && !forbidden;
    let expected: BTreeSet<_> = case["expected_citations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut seen = BTreeSet::new();
    let mut evidence = citations.is_some();
    for citation in citations.into_iter().flatten() {
        let id = citation["source_id"].as_str().unwrap_or("");
        let quote = citation["quote"].as_str().unwrap_or("");
        let source = notes["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|source| source["id"] == id);
        evidence &= expected.contains(id)
            && seen.insert(id)
            && quote.chars().count() >= 12
            && source
                .is_some_and(|source| source["text"].as_str().is_some_and(|s| s.contains(quote)));
    }
    evidence &= seen == expected;
    let mut reasons = Vec::new();
    if !schema {
        reasons.push("invalid response schema");
    }
    if !status {
        reasons.push("wrong answer/clarify/abstain decision");
    }
    if matched < groups.len() {
        reasons.push("missing required concept");
    }
    if forbidden {
        reasons.push("forbidden phrase present");
    }
    if !evidence {
        reasons.push("missing, unrelated, duplicate, invented, or nonverbatim evidence");
    }
    json!({"pass":schema && status && concepts && evidence,"schema":schema,"status":status,
        "concepts":concepts,"evidence":evidence,"matched_concepts":matched,
        "total_concepts":groups.len(),"reasons":reasons})
}

/// Identifier for comparison/resume consistency, not a cryptographic signature.
pub fn fingerprint(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("fnv1a64:{hash:016x}")
}

pub fn summarize(records: &[Value], profiles: &Value, runs: usize, case_count: usize) -> Value {
    let profile_names: BTreeSet<_> = profiles
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|profile| profile["name"].as_str())
        .collect();
    let mut tuples = BTreeSet::new();
    let structurally_valid = records.iter().all(|record| {
        let Some(name) = record["profile"].as_str() else {
            return false;
        };
        let Some(run) = record["run"].as_u64() else {
            return false;
        };
        let Some(case_id) = record["case_id"].as_str() else {
            return false;
        };
        profile_names.contains(name)
            && (1..=runs as u64).contains(&run)
            && !case_id.is_empty()
            && record.get("error").is_some()
            && record["score"]["pass"].as_bool().is_some()
            && tuples.insert((name, run, case_id))
    });
    let first_profile = profile_names.iter().next().copied().unwrap_or("");
    let expected_cases: BTreeSet<_> = tuples
        .iter()
        .filter(|(name, run, _)| *name == first_profile && *run == 1)
        .map(|(_, _, case_id)| *case_id)
        .collect();
    let coverage_valid = !profile_names.is_empty()
        && runs > 0
        && case_count > 0
        && expected_cases.len() == case_count
        && profile_names.iter().all(|name| {
            (1..=runs as u64).all(|run| {
                let actual: BTreeSet<_> = tuples
                    .iter()
                    .filter(|(profile, record_run, _)| profile == name && *record_run == run)
                    .map(|(_, _, case_id)| *case_id)
                    .collect();
                actual == expected_cases
            })
        });
    let mut rows = Vec::new();
    for profile in profiles.as_array().into_iter().flatten() {
        let name = profile["name"].as_str().unwrap_or("");
        let selected: Vec<_> = records.iter().filter(|r| r["profile"] == name).collect();
        let mut run_scores = Vec::new();
        for run in 1..=runs {
            let count = selected
                .iter()
                .filter(|r| r["run"] == run && r["score"]["pass"] == true)
                .count();
            run_scores.push(100.0 * count as f64 / case_count as f64);
        }
        let passed = selected
            .iter()
            .filter(|r| r["score"]["pass"] == true)
            .count();
        let completed = selected.iter().filter(|r| r["error"].is_null()).count();
        let latencies: Vec<_> = selected
            .iter()
            .filter(|r| r["error"].is_null())
            .filter_map(|r| r["latency_ms"].as_f64())
            .collect();
        let average_latency = if latencies.is_empty() {
            Value::Null
        } else {
            json!(latencies.iter().sum::<f64>() / latencies.len() as f64)
        };
        let total = runs * case_count;
        rows.push(json!({"profile":name,"provider":profile["provider"],"model":profile["model"],
            "expected_requests":total,"recorded_requests":selected.len(),"successful_requests":completed,
            "api_errors":selected.len()-completed,"passed":passed,
            "score_is_final":structurally_valid && coverage_valid && selected.len()==total && completed==total,
            "score_percent":100.0*passed as f64/total as f64,
            "run_scores_percent":run_scores,
            "score_min_percent":run_scores.iter().copied().fold(f64::INFINITY,f64::min),
            "score_max_percent":run_scores.iter().copied().fold(f64::NEG_INFINITY,f64::max),
            "average_response_ms":average_latency,
            "estimated_usd_per_1000_free_tier":0.0,
            "cost_condition":"Only while requests are covered by the provider's free account limits; billing not independently verified"}));
    }
    let complete = structurally_valid
        && coverage_valid
        && rows.iter().all(|row| {
            row["recorded_requests"] == row["expected_requests"] && row["api_errors"] == 0
        });
    let chosen = if complete && !rows.is_empty() {
        rows.iter()
            .max_by(|left, right| {
                left["score_percent"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .total_cmp(&right["score_percent"].as_f64().unwrap_or(0.0))
                    .then_with(|| {
                        right["average_response_ms"]
                            .as_f64()
                            .unwrap_or(f64::INFINITY)
                            .total_cmp(
                                &left["average_response_ms"]
                                    .as_f64()
                                    .unwrap_or(f64::INFINITY),
                            )
                    })
            })
            .map(|row| row["profile"].clone())
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    json!({"complete":complete,"runs_per_model":runs,"cases_per_run":case_count,"models":rows,
        "chosen_profile":chosen,"selection_rule":"Highest mean strict pass rate, then lowest mean successful response latency; only complete error-free experiments are eligible"})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixtures() -> (Value, Value, Value) {
        let notes = json!({"sources":[{"id":"S1","title":"Testing","text":"Unit tests isolate one function. Integration tests check interactions."}]});
        let case = json!({"id":"T1","category":"typical","input":"What are unit tests?","expected_status":"answer",
            "expected_answer":"Unit tests isolate one function.","required_concepts":[["isolate","isolation"],["function","unit"]],
            "forbidden_phrases":["guarantee no bugs"],"expected_citations":["S1"]});
        let output = json!({"status":"answer","answer":"Unit tests isolate one function.",
            "citations":[{"source_id":"S1","quote":"Unit tests isolate one function."}]});
        (notes, case, output)
    }
    #[test]
    fn correct_answer_passes() {
        let (n, c, o) = fixtures();
        assert_eq!(score(&c, Some(&o), &n)["pass"], true);
    }
    #[test]
    fn invented_quote_and_id_fail() {
        let (n, c, mut o) = fixtures();
        o["citations"][0]["quote"] = json!("Testing proves correctness.");
        assert_eq!(score(&c, Some(&o), &n)["evidence"], false);
        o["citations"][0]["source_id"] = json!("S99");
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
    }
    #[test]
    fn missing_or_duplicate_citation_fails() {
        let (n, c, mut o) = fixtures();
        let duplicate = o["citations"][0].clone();
        o["citations"].as_array_mut().unwrap().push(duplicate);
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
        o["citations"] = json!([]);
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
    }
    #[test]
    fn incorrect_decision_missing_concept_and_forbidden_claim_fail() {
        let (n, c, mut o) = fixtures();
        o["status"] = json!("abstain");
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
        o["status"] = json!("answer");
        o["answer"] = json!("Tests are helpful.");
        assert_eq!(score(&c, Some(&o), &n)["concepts"], false);
        o["answer"] = json!("Unit tests isolate one function and guarantee no bugs.");
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
    }
    #[test]
    fn malformed_schema_is_failure() {
        let (n, c, _) = fixtures();
        for o in [Value::Null, json!({"status":"answer"}), json!([])] {
            assert_eq!(score(&c, Some(&o), &n)["pass"], false);
        }
        assert_eq!(score(&c, None, &n)["pass"], false);
    }
    #[test]
    fn whole_words_and_normalization() {
        assert!(phrase_present(
            &normalize("Unit-tests: ISOLATE one function!"),
            "unit tests"
        ));
        assert!(!phrase_present("dysfunction", "function"));
    }
    #[test]
    fn numeric_magnitude_sign_and_percent_are_not_lost() {
        assert!(!phrase_present(&normalize("The cost is $2,000."), "$2"));
        assert!(!phrase_present(&normalize("Precision is 0.8%."), "0.8"));
        assert!(!phrase_present(&normalize("Precision is -0.8."), "0.8"));
        assert!(!phrase_present(&normalize("The cost is -$2."), "$2"));
        assert!(phrase_present(&normalize("Precision is 0.80."), "0.8"));
        assert!(phrase_present(&normalize("The cost is $2.00."), "$2"));
    }
    #[test]
    fn abstention_needs_no_citations() {
        let (n, mut c, mut o) = fixtures();
        c["expected_status"] = json!("abstain");
        c["required_concepts"] = json!([["not in notes"]]);
        c["expected_citations"] = json!([]);
        o["status"] = json!("abstain");
        o["answer"] = json!("This is not in notes.");
        o["citations"] = json!([]);
        assert_eq!(score(&c, Some(&o), &n)["pass"], true);
        o["citations"] = json!([{"source_id":"S1","quote":"Unit tests isolate one function."}]);
        assert_eq!(score(&c, Some(&o), &n)["pass"], false);
    }
    #[test]
    fn partial_experiment_cannot_select_model() {
        let profiles = json!([{"name":"one","provider":"groq","model":"a"}]);
        let records = vec![
            json!({"profile":"one","run":1,"score":{"pass":true},"error":null,"latency_ms":10}),
        ];
        let result = summarize(&records, &profiles, 3, 50);
        assert_eq!(result["complete"], false);
        assert!(result["chosen_profile"].is_null());
    }
    #[test]
    fn selection_uses_quality_then_latency() {
        let profiles = json!([{"name":"one"},{"name":"two"}]);
        let records = vec![
            json!({"profile":"one","run":1,"case_id":"E01","score":{"pass":true},"error":null,"latency_ms":20}),
            json!({"profile":"two","run":1,"case_id":"E01","score":{"pass":true},"error":null,"latency_ms":10}),
        ];
        assert_eq!(
            summarize(&records, &profiles, 1, 1)["chosen_profile"],
            "two"
        );
    }

    #[test]
    fn wrong_runs_duplicate_records_and_mismatched_case_coverage_cannot_select() {
        let profiles = json!([{"name":"one"}]);
        let record = json!({"profile":"one","run":2,"case_id":"E01","score":{"pass":true},"error":null,"latency_ms":1});
        assert_eq!(
            summarize(std::slice::from_ref(&record), &profiles, 1, 1)["complete"],
            false
        );
        let mut correct = record.clone();
        correct["run"] = json!(1);
        assert_eq!(
            summarize(&[correct.clone(), correct], &profiles, 1, 2)["complete"],
            false
        );
        let second = json!({"profile":"one","run":1,"case_id":"E02","score":{"pass":true},"error":null,"latency_ms":1});
        assert_eq!(
            summarize(&[record, second], &profiles, 2, 1)["complete"],
            false
        );
    }
}
