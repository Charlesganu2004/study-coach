# Project report: Source-grounded study coach

Milestone 1 evaluation and write-up are complete locally, November 1, 2026 (America/Chicago). All three models completed three 50-case runs, giving 450 unique completed API responses. The Rust summary and an independent aggregation agree on coverage, scores, ranges, and response times. Publishing the updated report and evidence to GitHub and submitting them remain separate steps.

The project is a Rust study coach that answers questions from supplied computing notes and returns source IDs with quotations. This replaces the earlier programming-error-explainer proposal. The current prototype uses hosted Groq and NVIDIA inference, rather than the brainstorm's local-only approach, and sends only the bundled original synthetic material. A graphical interface is later project work and is not a requirement in the supplied Milestone 1 instructions.

## Evaluation method

A good study-coach response answers a student's computing question using the supplied notes, provides checkable evidence, and avoids inventing missing facts. When the request is ambiguous or contradictory, it should ask for clarification; when the requested information is absent or outside the study task, it should abstain.

The primary metric is strict pass rate: the number of cases satisfying every programmed criterion divided by all 50 cases. This metric tests whether a response meets the application's output contract and reference criteria. It combines relevant concepts, the appropriate response status, and source evidence because a plausible explanation without support can mislead a student. Component checks and categories identify where failures occur. Strict pass rate is not a measurement of factual accuracy or overall teaching quality.

The deterministic Rust grader requires valid JSON with the prescribed fields and the exact expected status: `answer`, `clarify`, or `abstain`. Every required concept group must match at least one permitted phrase alternative. Matching uses normalized phrases with word boundaries and preserves numerical and unit distinctions. Forbidden phrases must be absent. Answer citations must match the complete expected source-ID set, without duplicates or extra IDs. Each quotation must contain at least 12 characters and appear in its named source. Clarification and abstention responses require no citations. No AI-as-a-judge score is used, so there is no judge prompt or claimed agreement with human ratings. Separate qualitative inspection examines response meaning and grader mistakes.

The development set contains 50 original synthetic inputs and 12 short original computing notes, with 36 expected answers, seven clarifications, and seven abstentions. Cases cover definitions, comparisons, calculations, practice questions, zero denominators, ambiguous and truncated inputs, missing course facts, false premises, conflicting constraints, and attempted prompt overrides. All models receive the entire identical corpus; retrieval is not evaluated. Only notes and user input are sent; reference answers and grading criteria stay out of prompts. These visible development cases were designed rather than sampled from student traffic. They are not an independent held-out test or a representative estimate of classroom performance.

The experiment schedules three runs per model, totaling 450 case responses across Groq GPT-OSS 20B, Groq GPT-OSS 120B, and NVIDIA Nemotron 3.5 Lightning. Prompt, corpus, examples, temperature of 0.2, top-p of 1, and generated-token allocation of 2,048 remain fixed. Provider reasoning defaults differ. Scores are summarized by their average and range across runs. Criteria remain frozen during collection, avoiding changes that favor observed outputs.

Mean response time covers successful calls, including retries within a call. It excludes request pacing, earlier standalone timeouts, and resume pauses; historical timeouts are reported separately. Estimated API charge is $0 per 1,000 requests only within applicable free quotas. Billing is not independently verified, and no paid fallback is enabled.

Inspection has identified valid paraphrases and additional relevant citations that fail, plus a quiz missing its requested question that passes. These limits require output inspection alongside scores. Future targets are at least 85% strict passes and mean response time below five seconds on unseen cases. The distinction between exact checks and broader application evaluation follows the author's public [Chapter 3–4 summaries of *AI Engineering*](https://github.com/chiphuyen/aie-book/blob/main/chapter-summaries.md).

## Model selection

The three configured candidates have documented free-access options and available weights. Provider and model choices are settings; each provider adapter uses a fixed HTTPS endpoint. [Groq limits](https://console.groq.com/docs/rate-limits), [NVIDIA endpoint](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b), [GPT-OSS weights and license](https://openai.com/index/introducing-gpt-oss/), [NVIDIA model card](https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16).

| Provider / model | Runs completed | Mean strict score / run range | Average successful response time | Estimated cost / 1,000 requests |
|---|---:|---|---|---|
| Groq `openai/gpt-oss-20b` | 3 of 3 | 79.33% / 78.00%–80.00% | 0.863 s | $0 within free quota; billing unverified |
| Groq `openai/gpt-oss-120b` | 3 of 3 | 85.33% / 84.00%–86.00% | 0.899 s | $0 within free quota; billing unverified |
| NVIDIA `nvidia/nemotron-3.5-lightning-30b-a3b` | 3 of 3 | 83.33% / 82.00%–86.00% | 20.854 s | $0 within free quota; billing unverified |

The individual run scores were 80%, 80%, and 78% for Groq 20B; 86%, 86%, and 84% for Groq 120B; and 86%, 82%, and 82% for NVIDIA. These correspond to 119, 128, and 125 strict passes out of 150 responses respectively.

**Selected model: Groq `openai/gpt-oss-120b`, using profile `groq-120b`.** It achieved the highest mean strict pass rate, six percentage points above 20B and two above NVIDIA, while averaging less than one second per completed call. The 20B model was slightly faster, but its score was lower. NVIDIA had a lower score, much longer completed-call latency, and nine timed-out requests requiring recovery. All three have the same conditional free-quota cost estimate. This selection follows the predeclared rule of highest mean strict pass rate, with response time as tie-breaker, and also fits the application's latency goal. The 85.33% result applies to this development set; the future target still needs validation on unseen cases.

Provider reasoning defaults and queue conditions affect these endpoint measurements. Conclusions apply to the tested APIs and settings. The score is compliance with this rubric, not factual accuracy or a general model ranking. Use the selected model with `scripts/coach.ps1 answer --model groq-120b --question "What is precision?"`.

Raw evidence is saved in [records.jsonl](results/milestone1-2026-11-01/records.jsonl); the frozen configuration and consistency fingerprint are in [manifest.json](results/milestone1-2026-11-01/manifest.json). The [Rust summary](results/milestone1-2026-11-01/summary.json) reports `milestone_complete=true`, and the [independent check](results/milestone1-2026-11-01/VERIFICATION.json) confirms all nine 50-case groups. The ledger contains 459 call outcomes: 450 final API responses and nine earlier timeout errors, all on NVIDIA requests and all recovered. The summary's zero outstanding API errors does not mean the run was uninterrupted. Timed-out call durations were not saved; their earlier waits and resume pauses are excluded from the completed-call means.

NVIDIA returned 11 empty final answers, each with 2,048 reported completion tokens. Exhausting the common output allocation is a plausible explanation, but the adapter did not capture `finish_reason`, so the cause is not confirmed. Empty final answers count as rubric failures. They remain distinct from transport timeouts.

A fresh experiment uses `scripts/coach.ps1 evaluate --models all --runs 3`. The saved completed experiment can be checked with `scripts/coach.ps1 summarize --out results/milestone1-2026-11-01` while using the same frozen code, data, and configuration.

## Iteration log

| Date | What changed | Score | What was learned |
|---|---|---|---|
| 2026-11-01 | Baseline: unchanged source-grounded prompt and 50-case rubric across three models; three repetitions per model complete | Means: 20B 79.33%; 120B 85.33%; NVIDIA 83.33% | Groq GPT-OSS 120B provides the strongest measured score/latency trade-off. Strict grades include real output-contract failures and limitations of the rubric. |
| 2026-11-01 | Recovered interrupted NVIDIA calls through checkpoint resume; no generation settings changed | Same frozen experiment; not a new quality iteration | Saved responses do not need to be repeated. Recovered timeouts must be reported separately from the final latest-response summary. |
| 2026-11-01 | Inspected diverse saved outputs without changing their code grades | Scores unchanged | A valid paraphrase can fail, an extra relevant citation can fail, and an incomplete quiz can pass. The current metric is not factual accuracy. |

### Three observed application failures in the chosen model

These examples come from the first run of the chosen Groq GPT-OSS 120B model. The rubric's original grades are preserved; E29 is a task failure that the code incorrectly marked as passing.

1. **E20, undefined precision.** Input: `TP=0 and FP=0. What precision value do the notes define?` The answer correctly says precision is undefined, but its S6 quotation contains `Precision = TP / (TP + FP). ... A metric with a zero denominator is undefined.` That combined string is not a contiguous source quotation. This is a citation-contract failure, not an arithmetic error. The likely cause is normal summarization behavior being applied inside a field that requires exact copying. First change: validate quotations in the user-facing answer path and request a contiguous exact excerpt when validation fails, while retaining the original attempt for evaluation. Evidence: [records.jsonl, line 59](results/milestone1-2026-11-01/records.jsonl#L59).

2. **E29, incomplete quiz.** Input asks for one unit-test question and its answer. Actual answer: `A unit test checks one function in isolation.` The definition is correct, but no question is generated. The code still passes it because it checks concepts and citations without checking completion of both requested parts. This is a genuine task failure and a rubric false pass. First change: give quiz responses separate question and answer fields and validate both, using independent examples before another controlled comparison. Evidence: [records.jsonl, line 86](results/milestone1-2026-11-01/records.jsonl#L86).

3. **E30, stitched quotation.** Input asks for a precision practice question with TP=3 and FP=1 and a worked answer. The model supplies the exercise and correctly computes `3 / (3 + 1) = 0.75`, but its S6 quote joins the TP, FP, and precision sentences while silently omitting the intervening FN sentence. That quotation fails the exact-substring check. The likely cause is selecting relevant facts and presenting them as one continuous quote. First change: explicitly require a single contiguous passage and add this nonadjacent-sentence response as a regression case for quotation validation. Evidence: [records.jsonl, line 89](results/milestone1-2026-11-01/records.jsonl#L89).

The [qualitative audit](results/milestone1-2026-11-01/QUALITATIVE_AUDIT.md) also records false failures on valid wording and extra supporting sources. Any revised criteria or prompt must be versioned and applied consistently to every model; the present scores will not be rewritten as factual-accuracy estimates.

### What remains for Milestone 1 submission

- Commit and push the updated report, saved result evidence, README changes, and example-file removal. Keep `.env` local and excluded from Git.
- Submit the repository through the course process and confirm the instructor can access the private repository.

Completed locally: repository and README setup; configurable Rust core function; 50-case dataset; one-command evaluation; three models with three runs each; score/range, latency, and cost table; evidence-based model selection; baseline iteration log; and three observed failure analyses. No graphical UI is required by the supplied Milestone 1 instructions.

### Later project work

A usable interface and study-material import are later development work; this milestone does not require them. Other planned improvements are stronger task-completion grading, fairer citation/paraphrase criteria, and an unseen final evaluation set. Later milestone instructions are needed to determine the exact required UI and remaining final-project features.
