# Model comparison

Results reflect saved API calls. Pending rows are not final model scores. Cost assumes requests remain covered by the account's free quotas; billing was not independently verified. Response time includes retries and backoff but excludes intentional pacing.

| Model profile | Strict pass mean | Run range | Mean response time | Estimated cost / 1,000 | Successful / expected |
| --- | ---: | ---: | ---: | ---: | ---: |
| groq-20b | 79.33% | 78.00% to 80.00% | 0.863 s | $0, conditional | 150 / 150 |
| groq-120b | 85.33% | 84.00% to 86.00% | 0.899 s | $0, conditional | 150 / 150 |
| nvidia-lightning | 83.33% | 82.00% to 86.00% | 20.854 s | $0, conditional | 150 / 150 |

Selected model profile: **groq-120b**. Selection follows highest mean strict pass rate, with lowest mean successful response time as the tie-breaker. All compared requests have the same conditional $0 estimate. Scores measure this synthetic development set, not general learning effectiveness.

## Observed failures

### E06 (run 1)

Input:

```text
What is a unit test supposed to check?
```

Expected:

```text
One function in isolation.
```

Actual output:

```text
{"status":"answer","answer":"A unit test is meant to verify that a single function works correctly on its own, comparing the function’s actual output to the expected result.","citations":[{"source_id":"S3","quote":"A unit test checks one function in isolation."},{"source_id":"S4","quote":"A check compares the actual result with that expectation."}]}
```

Scorer reasons: ["missing required concept","missing, unrelated, duplicate, invented, or nonverbatim evidence"]

Possible cause: a quote or citation set did not match the strict reference requirements. First change: inspect whether the model used valid alternative evidence before changing the common citation instructions or rubric.

### E13 (run 1)

Input:

```text
Explain a false positive using the notes' classification labels.
```

Expected:

```text
A predicted positive that is actually negative.
```

Actual output:

```text
{"status":"answer","answer":"A false positive occurs when the model predicts a positive label, but the true label is negative—this is the FP case in classification metrics.","citations":[{"source_id":"S6","quote":"FP means a predicted positive that is actually negative."}]}
```

Scorer reasons: ["missing required concept"]

Possible cause: the response omitted a required idea, used an unlisted synonym, or triggered a context-insensitive forbidden phrase. First change: manually inspect meaning and negation before deciding whether to improve the shared prompt or document and revise the rubric.

### E20 (run 1)

Input:

```text
TP=0 and FP=0. What precision value do the notes define?
```

Expected:

```text
Precision is undefined because its denominator is zero.
```

Actual output:

```text
{"status":"answer","answer":"The notes define precision as undefined because the denominator TP + FP equals zero.","citations":[{"source_id":"S6","quote":"Precision = TP / (TP + FP). ... A metric with a zero denominator is undefined."}]}
```

Scorer reasons: ["missing, unrelated, duplicate, invented, or nonverbatim evidence"]

Possible cause: a quote or citation set did not match the strict reference requirements. First change: inspect whether the model used valid alternative evidence before changing the common citation instructions or rubric.
