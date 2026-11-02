# Project report: Source-grounded study coach

Milestone 1 status on November 1, 2026 (America/Chicago): Rust implementation and local checks available; live model comparison, final selection, and three observed failure analyses await locally configured Groq and NVIDIA credentials. The work below distinguishes a completed evaluation design from results not yet measured.

Scope follows the latest request to choose a study-coach candidate and code it in Rust. It supersedes the programming-error-explainer direction in the earlier proposal. Free hosted APIs also replace the brainstorm's local-only inference. Only original synthetic notes are included. The assignment says “about 500 pages” for evaluation method; this report interprets that as approximately 500 words, subject to instructor clarification.

## Evaluation method

Good performance means a student receives a useful answer supported by the supplied notes, or a clear clarification or abstention when those notes cannot answer the question. Unsupported confidence is a failure even when a response sounds fluent.

The primary metric is strict case pass rate: passing cases divided by all scheduled cases. A case passes only when its JSON schema is valid, its answer/clarify/abstain decision matches the reference, every required concept appears, no forbidden phrase appears, and its citations satisfy the evidence check. Concept groups include alternative phrases to reduce sensitivity to ordinary wording differences. Matching ignores case and ordinary punctuation, preserves numeric magnitude, signs and percentages, and respects word boundaries. Exact-match whole-answer scoring would penalize harmless paraphrases, so it is not used.

The evidence check requires the expected source IDs and a quotation of at least twelve characters copied exactly from each source. Unknown, repeated, irrelevant-to-the-rubric, missing, or invented citations fail. Clarifications and abstentions require no citations. This is deterministic code scoring, with no AI judge; therefore no judge prompt or judge-versus-human agreement result is claimed. The executable implementation and adversarial scorer tests are in `src/eval.rs`. Reference answers are separately checked against their own criteria to catch impossible rubrics before spending API quota.

The development set contains fifty original examples based on twelve short, original introductory computing notes. It contains thirty-six answerable questions, seven clarification cases, and seven abstentions. Cases cover definitions, comparisons, calculations, a quiz, misleading premises, missing course details, ambiguous references, contradictory instructions, prompt injection, whitespace-only input, and a truncated question. `data/DATASET.md` records provenance and category counts. References and scoring criteria are never sent to the model. All models receive the same full note collection, prompt, question, temperature, top-p, and output allocation; a small fixed corpus avoids retrieval differences during model selection.

Every model is scheduled for three complete runs. The script records each response, score components, response latency in milliseconds, retry count, available token usage, and sanitized API errors. It reports mean strict pass rate and its minimum-to-maximum range across runs. Mean latency includes retry waits for successful calls and excludes deliberate interrequest pacing. Infrastructure errors remain visible and prevent selection; they are not silently discarded. Estimated API cost is zero per thousand requests only while an account's free quota covers those requests.

These choices follow the emphasis on application-specific criteria, data, and evaluation pipelines in Huyen's publicly available summaries of Chapters 3–4, rather than claiming access to the complete chapters. The set is intentionally inspectable, but it is not a representative user sample or an unseen final test. Phrase matching can miss valid paraphrases and accept keyword stuffing; authentic quotations do not establish semantic entailment. Human review of sampled outputs and a separate unseen final set remain necessary. The final target is at least 85% strict pass rate, no fabricated source IDs, and mean successful latency below five seconds, measured separately from quota pacing.

Reference: Chip Huyen, *AI Engineering*, [author's chapter summaries](https://github.com/chiphuyen/aie-book/blob/main/chapter-summaries.md).

## Model selection

The three configured candidates have documented free-access options and available weights. Their endpoints and model identifiers are settings, not hard-coded choices inside the study workflow. [Groq limits](https://console.groq.com/docs/rate-limits), [NVIDIA endpoint](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b), [GPT-OSS weights and license](https://openai.com/index/introducing-gpt-oss/), [NVIDIA model card](https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16).

| Provider / model | Runs completed | Mean score / range | Average response time | Estimated cost / 1,000 requests |
|---|---:|---|---|---|
| Groq `openai/gpt-oss-20b` | 0 of 3 | Not measured | Not measured | $0 within free quota; billing unverified |
| Groq `openai/gpt-oss-120b` | 0 of 3 | Not measured | Not measured | $0 within free quota; billing unverified |
| NVIDIA `nvidia/nemotron-3.5-lightning-30b-a3b` | 0 of 3 | Not measured | Not measured | $0 within free quota; billing unverified |

No model has been selected. A profile default in the CLI is a convenience, not a measured recommendation. The predeclared rule selects the highest mean strict pass rate, using lowest mean successful response latency to break ties. API errors or incomplete runs prevent selection. Free-tier cost ties do not justify choosing a model without quality evidence. Provider reasoning defaults and queue conditions may affect results despite identical exposed common settings.

Run `scripts/coach.ps1 evaluate --models all --runs 3` on Windows, or `cargo run --locked --release -- evaluate --models all --runs 3` elsewhere. The result directory records raw evidence and generates `MODEL_SELECTION.md`; incorporate its completed table here after reviewing actual responses.

## Iteration log

| Date | What changed | Score | What was learned |
|---|---|---|---|
| 2026-11-01 | Baseline: fixed source-grounded prompt, 50-case rubric, three configurable free model profiles; no prompt tuning against provider outputs | Not measured; 0 live runs | Credentials are required before measuring quality, latency, or selecting a model. Local integrity checks cannot substitute for this baseline. |

Three actual chosen-model failures are **not known** because no live comparison has completed. No examples below are represented as observed model mistakes. Once measurements exist, inspect three distinct failed cases in `MODEL_SELECTION.md`, quoting each input, expected behavior, actual output, and failed rubric components. Then state a plausible failure cause and the first specific change to test. If fewer than three distinct cases fail, report that count honestly and ask the instructor how to handle the requirement.

The planned first investigations are unsupported answers on missing-information cases, failure to clarify ambiguity, and inaccurate quotations. These are hypotheses for analysis, not observed results or substitute failure writeups. Any prompt or scoring change starts a new frozen experiment so the three-model comparison remains fair.
