# Milestone 2 approach comparison

This study compares two prompt strategies for the same study coach. It changes the system instructions, while retaining the selected model, source collection, inputs, scoring code, and generation settings. The comparison is separate from the historical Milestone 1 model selection.

## Approaches

* `baseline` sends the original Milestone 1 system prompt without alteration. It asks for a supported answer, clarification, or abstention in a JSON object with source quotations.
* `grounded` appends a verification instruction to that same prompt. It asks the model to complete the requested learning task and copy each quotation as one contiguous source passage, preserving intervening words and punctuation. It forbids ellipses and stitched quotations. This targets E20 and E30 quotation failures and E29's missing quiz question from the Milestone 1 qualitative audit.

Both approaches call `core::answer`. The application and evaluator use that same function. No retrieval engine, external search, additional agent, answer rewriting, or grading change is introduced by this experiment. Both send all 12 supplied sources.

## Controlled inputs

| Setting | Value |
| --- | --- |
| Provider | Groq |
| Model | `openai/gpt-oss-120b` |
| Dataset | Original `data/eval.json`, 50 cases |
| Sources | Original `data/notes.json`, 12 sources |
| Temperature | 0.2 |
| Top p | 1.0 |
| Output allocation | 2048 tokens |
| Timeout | 120 seconds |
| Retries | 2 |
| Minimum interval | 25000 ms |
| Runs per approach | 1 by default, configurable with `-Runs` |

The scorer is the unchanged strict contract check from Milestone 1. A pass requires the expected decision, required concepts, no prohibited claim, the exact expected citation set, and a valid source quotation under the existing checks. The reference answers satisfy all 50 rubrics. This reference check is separate from model performance.

The experiment manifest saves the exact system prompt, chosen approach, generation configuration, case count, run count, and a fingerprint of the data, prompt, and relevant source code. Each response saves its raw text, parsed output, rubric decisions, token usage when returned, latency, and any provider error. Errors remain in the checkpoint history. A resumed run retries unsuccessful tuples without discarding earlier outcomes.

## Run and inspect

```powershell
.\scripts\compare-approaches.ps1
```

The command validates the dataset, evaluates `baseline`, then evaluates `grounded`. Existing checkpoints are resumed only when their experiment settings and fingerprint match. `comparison.json` and `comparison.csv` are written only after both approaches have complete results. Each approach keeps its own `manifest.json`, `records.jsonl`, and `summary.json`.

For this study, `summary.json` uses `complete` to identify a complete approach run. The separate `milestone_complete` flag belongs to Milestone 1's requirement for three models and three runs, so it is false for a complete Milestone 2 approach run. Its automatically generated `MODEL_SELECTION.md` is not a Milestone 2 completion judgment. The overall Milestone 2 evidence is the two complete approach runs and their `comparison.json`.

## Interpretation limits

A single run per approach satisfies the two approach comparison in Milestone 2, but it does not estimate output variability or establish statistical significance. Milestone 1 separately retains three runs per model. The approaches run sequentially, so provider load or a service change may affect latency. The original rubric can reject correct paraphrases and accept an incomplete learning task, as disclosed in the Milestone 1 qualitative audit. Scores measure that strict contract, not an independent estimate of factual accuracy or learning benefit.

Estimated API charge is $0 per 1,000 requests only while the provider covers those requests under the account's free limits. This is not an unlimited free service claim, and account billing has not been independently verified. The runner stops and saves progress if access, quota, or transport fails. It does not change billing settings or buy credits.
