use serde_json::{json, Value};
use study_coach::eval::{score, validate};

fn dataset() -> (Value, Value) {
    (
        serde_json::from_str(include_str!("../data/notes.json")).unwrap(),
        serde_json::from_str(include_str!("../data/eval.json")).unwrap(),
    )
}

#[test]
fn all_fifty_gold_answers_satisfy_their_rubric() {
    let (notes, cases) = dataset();
    let metadata = validate(&notes, &cases).unwrap();
    assert_eq!(metadata["cases"], 50);
    assert_eq!(metadata["sources"], 12);
    for case in cases.as_array().unwrap() {
        let citations: Vec<_> = case["expected_citations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let source = notes["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|source| source["id"] == *id)
                    .unwrap();
                json!({"source_id":id,"quote":source["text"]})
            })
            .collect();
        let answer = json!({"status":case["expected_status"],"answer":case["expected_answer"],"citations":citations});
        let result = score(case, Some(&answer), &notes);
        assert_eq!(result["pass"], true, "{}: {result}", case["id"]);
    }
}

#[test]
fn incomplete_dataset_and_unknown_citations_are_rejected() {
    let (notes, mut cases) = dataset();
    cases[0]["expected_citations"] = json!(["invented"]);
    assert!(validate(&notes, &cases).unwrap_err().contains("unknown"));
    cases.as_array_mut().unwrap().truncate(49);
    assert!(validate(&notes, &cases).unwrap_err().contains("50"));
}

#[test]
fn reversed_classification_definitions_and_wrong_numbers_fail() {
    let (notes, cases) = dataset();
    for (id, wrong_answer) in [
        (
            "E13",
            "A false positive is an actual positive predicted negative.",
        ),
        (
            "E14",
            "A false negative is a predicted positive that is actually negative.",
        ),
        ("E15", "Precision is 0.8%."),
        ("E19", "The estimated cost is $2,000."),
    ] {
        let case = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let citations: Vec<_> = case["expected_citations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                let source = notes["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|source| source["id"] == *id)
                    .unwrap();
                json!({"source_id":id,"quote":source["text"]})
            })
            .collect();
        let output = json!({"status":"answer","answer":wrong_answer,"citations":citations});
        assert_eq!(
            score(case, Some(&output), &notes)["pass"],
            false,
            "{id} accepted: {wrong_answer}"
        );
    }
}
