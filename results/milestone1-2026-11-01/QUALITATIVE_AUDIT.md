# Qualitative output inspection

An AI assistant helped inspect the saved outputs. These observations are not a separate scored human validation or a metric from an AI judge. The reported benchmark score remains the deterministic rubric score. No prompts, examples, parameters, or scoring code changed during the experiment.

## Groq GPT OSS 120B: diverse cases from the first run

| Case | Saved line in records.jsonl | Code grade | What inspection establishes |
|---|---:|---|---|
| E13: false positive | 38 | Fail | The answer correctly says a positive prediction has a negative true label. It fails a narrow required phrase group. This is a rubric false failure. |
| E15: precision | 44 | Pass | `8 / (8 + 2) = 0.8` is correct; S6 supplies the formula. |
| E28: controlled comparison | 83 | Pass | The answer names the fixed prompt/examples/temperature/output limit, average/range across repeated runs, and inclusion of retry waits in latency. It cites both S11 and S8. |
| E29: quiz generation | 86 | Pass | Actual answer is only `A unit test checks one function in isolation.` It contains no generated question. This failure to complete the task is missed by the rubric. |
| E38: instructor name | 114 | Pass | The model abstains because the supplied notes do not name the instructor. |
| E43: romantic poem | 129 | Pass | The model abstains because the task is outside the documented study coach scope. This checks scope compliance, not general creativity. |

## Citation failures and false failures

* Groq 120B E06, line 17: correct unit test explanation, with exact, relevant quotations from S3 and S4. The scorer rejects S4 because its expected source set contains only S3, and rejects the valid paraphrase `on its own`. Do not label this a factual model error.
* Groq 120B E20, line 59: correct answer that precision is undefined, but quotation inserts an ellipsis instead of copying a contiguous exact source excerpt. The failure to meet the citation contract is real; the arithmetic answer is correct.
* Groq 120B E26, line 77: correct explanation and relevant S6/S7 quotations, but the gold set permits only S7. This is another false failure caused by the permitted source set.
* Groq 120B E30, line 89: correct precision exercise and worked 0.75 answer, but quotation skips the intervening FN sentence from S6. This violates the exact quotation contract.

## Other observed model issues

Groq 20B E14, line 40, adds an assertion about there being three possible binary classification outcomes. The supplied source defines TP, FP, and FN but does not support that exhaustive assertion. Its recorded rubric failure concerns phrasing rather than this unsupported addition, illustrating another limitation of the concept checklist.

NVIDIA E13/E27/E29/E30, lines 39/81/87/90, return empty final text with 2,048 reported completion tokens. Exhausting the common generation allocation is a plausible explanation. The adapter did not save `finish_reason`, so that causal interpretation is not confirmed. These are unavailable final answers, not evidence of incorrect arithmetic or definitions.

The final ledger contains nine NVIDIA timeout outcomes, all recovered without repeating completed calls. Keep those original error records when counting interruptions; the summary using the latest completed response for each case has zero outstanding errors after recovery. Mean completed call latency does not include an earlier standalone timeout or the pause before resume. Individual failed call durations were not recorded.

Final aggregation also finds 11 empty NVIDIA final responses, all with 2,048 reported completion tokens. These are rubric failures within the 450 completed responses, separate from the nine timeout outcomes. `VERIFICATION.json` records these counts.

## Changes to test next

Add an explicit check that the quiz task is complete; revise synonym and permitted supporting source criteria with independent examples; preserve exact quotation validation while improving the instruction to copy contiguous excerpts; and capture provider finish reasons. Version any revised rubric and apply it to every compared model. Do not retroactively relabel these frozen scores as factual accuracy.
