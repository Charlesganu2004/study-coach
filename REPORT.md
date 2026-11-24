# Project report: Source grounded study coach

Updated November 8, 2026 (America/Chicago). Milestone 1 completed three models, three runs per model, and 50 cases per run: 450 unique completed API responses. Its original evidence was committed in `3d10051220aec37c2fa1ae9337d76bcd4c2d5798`. This report adds the working prototype, distribution, architecture, and a separate controlled prompt comparison for Milestone 2. The final project rubric beyond the supplied milestone sheets has not been provided.

## 1. Problem and audience

The study coach helps students ask questions about notes, compare concepts, and generate practice material with quotations they can check. It should clarify ambiguous requests and abstain when information is absent. This replaces the earlier programming error explainer direction in the separate class proposal.

Notes and history are stored locally. Asking sends the question and complete current source collection to Groq or NVIDIA. This changes the brainstorm's local inference direction: the application does not provide offline AI or guarantee private cloud processing. Evaluation uses only original synthetic computing material, not private course documents.

## 2. Approach

The technique is prompt engineering with a complete small source collection in context. A fixed schema distinguishes `answer`, `clarify`, and `abstain`; answers include source IDs and exact passages. This fits the 12 source evaluation corpus without introducing a retrieval stage that might omit a relevant passage. Users can paste notes and import `.txt` or `.md` files. The application does not extract PDFs or train a model.

The model alternatives were measured: Groq 120B achieved 85.33% mean strict passes at 0.899 seconds, versus 79.33% at 0.863 seconds for Groq 20B and 83.33% at 20.854 seconds for NVIDIA. These development results support the 120B default. Milestone 2 also measured lexical retrieval against full context. Finetuning and local inference were considered but not evaluated: finetuning needs training data beyond these visible examples, while local inference requires hardware and installation testing. Their omission is a design tradeoff, not a measured defeat.

The Milestone 2 change adds explicit task completion and contiguous quotation instructions to the original prompt. Both approaches use the same core function, selected model, cases, notes, rubric, temperature, top p, output allocation, and pacing. The original instructions remain available as `baseline`; the revision is `grounded`. The actual prototype was tested through the real API before the comparison began. No rubric criterion was loosened to improve the score.

For the graduate comparison, the grounded prompt was held fixed while context changed from all sources to up to four positively matching sources. Shared title terms have weight three and body terms weight one; original order breaks ties, with all notes used when no terms overlap. Selection never reads reference answers or grading criteria. The installed application can use the same selector with `--context retrieval`. This is lexical retrieval augmented generation without embeddings or a vector database.

Codex assisted with scaffolding, implementation, tests, and review. Review corrected a UI byte limit mismatch, accidental persistence of inherited keys, and a second instance data loss bug. [Development evidence](docs/AI_DEVELOPMENT.md) contains three actual conversation excerpts and browser screenshots. Presentation fixtures are distinguished from real provider calls. No manual authorship or manufactured commit history is claimed.

## 3. System architecture

[The whole application diagram and component table](docs/ARCHITECTURE.md) show the browser, Rust API, local files, credentials, shared core, cloud endpoints, output guard, evaluator, and saved evidence. The browser sends a question to the loopback service, which checks the session and input, calls `study_coach::core::answer`, verifies output, and saves an accepted response locally. The evaluator calls the same function and retains raw failures for scoring.

The executable embeds HTML, CSS, and JavaScript. It checks Host, Origin, and a random local session token, with bounded HTTP parsing and a restrictive Content Security Policy. Sources, settings, and the last 50 accepted answers are JSON files. Credentials stay in the app data `.env`; API state exposes configured booleans only. Fresh Windows directories receive restricted ACLs; existing directories retain their owner's policy. Unix files use restrictive permissions. This is not an encrypted vault or a hosted account system.

The output guard rejects malformed schema, leaked configured keys, missing or duplicate source IDs, and quotations absent from the named source. Empty or punctuation only questions receive a local clarification. An operating system data folder lock prevents concurrent cached writers. Quota errors, timeouts, missing keys, and unavailable models return errors without a paid fallback. Exact quotation checks establish provenance, not semantic entailment.

The package contains the compiled service, launchers, installers, removal scripts, and checksums. Program files and study data are separate; removal preserves notes and keys. Checksums detect corruption but do not authenticate an unsigned publisher. Windows installation is tested. Unix installer logic has a fixture check; native Linux and macOS execution remains unverified.

The final interface uses a dark dimensional scene, a rotating CSS 3D cube, orbital motion, pointer parallax, and a high density canvas point field. Motion can be turned off and follows the operating system's reduced motion setting. These effects use no external UI library or remote assets. Browser checks measure actual transforms and persistence of the motion setting; the live browser receipt records a real model answer through this interface.

## 4. Evaluation and iteration

### Evaluation method

A good study coach response answers a student's computing question using the supplied notes, provides checkable evidence, and avoids inventing missing facts. When the request is ambiguous or contradictory, it should ask for clarification; when the requested information is absent or outside the study task, it should abstain.

The primary metric is strict pass rate: the number of cases satisfying every programmed criterion divided by all 50 cases. This metric tests whether a response meets the application's output contract and reference criteria. It combines relevant concepts, the appropriate response status, and source evidence because a plausible explanation without support can mislead a student. Component checks and categories identify where failures occur. Strict pass rate is not a measurement of factual accuracy or overall teaching quality.

The deterministic Rust grader requires valid JSON with the prescribed fields and the exact expected status: `answer`, `clarify`, or `abstain`. Every required concept group must match at least one permitted phrase alternative. Matching uses normalized phrases with word boundaries and preserves numerical and unit distinctions. Forbidden phrases must be absent. Answer citations must match the complete expected set of source IDs, without duplicates or extra IDs. Each quotation must contain at least 12 characters and appear in its named source. Clarification and abstention responses require no citations. No AI judge score is used, so there is no judge prompt or claimed agreement with human ratings. Separate qualitative inspection examines response meaning and grader mistakes.

The development set contains 50 original synthetic inputs and 12 short original computing notes, with 36 expected answers, seven clarifications, and seven abstentions. Cases cover definitions, comparisons, calculations, practice questions, zero denominators, ambiguous and truncated inputs, missing course facts, false premises, conflicting constraints, and attempted prompt overrides. All Milestone 1 models receive the entire identical corpus; the separate Milestone 2 retrieval comparison is described below. Only notes and user input are sent; reference answers and grading criteria stay out of prompts. These visible development cases were designed rather than sampled from student traffic. They are not an independent final test set or a representative estimate of classroom performance.

The experiment schedules three runs per model, totaling 450 case responses across Groq GPT OSS 20B, Groq GPT OSS 120B, and NVIDIA Nemotron 3.5 Lightning. Prompt, corpus, examples, temperature of 0.2, top p of 1, and an allocation of 2,048 generated tokens remain fixed. Provider reasoning defaults differ. Scores are summarized by their average and range across runs. Criteria remain frozen during collection, avoiding changes that favor observed outputs.

Mean response time covers successful calls, including retries within a call. It excludes request pacing, earlier standalone timeouts, and resume pauses; historical timeouts are reported separately. Estimated API charge is $0 per 1,000 requests only within applicable free quotas. Billing is not independently verified, and no paid fallback is enabled.

Inspection has identified valid paraphrases and additional relevant citations that fail, plus a quiz missing its requested question that passes. These limits require output inspection alongside scores. Future targets are at least 85% strict passes and mean response time below five seconds on unseen cases. The distinction between exact checks and broader application evaluation follows the author's public [Summaries of Chapters 3 and 4 of *AI Engineering*](https://github.com/chiphuyen/aie-book/blob/main/chapter-summaries.md).

### Model selection

The three configured candidates have documented options for free access and available weights. Provider and model choices are settings; each provider adapter uses a fixed HTTPS endpoint. [Groq limits](https://console.groq.com/docs/rate-limits), [NVIDIA endpoint](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b), [GPT OSS weights and license](https://openai.com/index/introducing-gpt-oss/), [NVIDIA model card](https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16).

| Provider / model | Runs completed | Mean strict score / run range | Average successful response time | Estimated cost / 1,000 requests |
|---|---:|---|---|---|
| Groq `openai/gpt-oss-20b` | 3 of 3 | 79.33% / 78.00% to 80.00% | 0.863 s | $0 within free quota; billing unverified |
| Groq `openai/gpt-oss-120b` | 3 of 3 | 85.33% / 84.00% to 86.00% | 0.899 s | $0 within free quota; billing unverified |
| NVIDIA `nvidia/nemotron-3.5-lightning-30b-a3b` | 3 of 3 | 83.33% / 82.00% to 86.00% | 20.854 s | $0 within free quota; billing unverified |

The individual run scores were 80%, 80%, and 78% for Groq 20B; 86%, 86%, and 84% for Groq 120B; and 86%, 82%, and 82% for NVIDIA. These correspond to 119, 128, and 125 strict passes out of 150 responses respectively.

**Selected model: Groq `openai/gpt-oss-120b`, using profile `groq-120b`.** It achieved the highest mean strict pass rate, six percentage points above 20B and two above NVIDIA, while averaging less than one second per completed call. The 20B model was slightly faster, but its score was lower. NVIDIA had a lower score, much longer latency for completed calls, and nine requests that timed out requiring recovery. All three have the same conditional cost estimate within free quotas. This selection follows the predeclared rule of highest mean strict pass rate, using response time to break ties, and also fits the application's latency goal. The 85.33% result applies to this development set; the future target still needs validation on unseen cases.

Provider reasoning defaults and queue conditions affect these endpoint measurements. Conclusions apply to the tested APIs and settings. The score is compliance with this rubric, not factual accuracy or a general model ranking. Use the selected model with `scripts/coach.ps1 answer --model groq-120b --question "What is precision?"`.

Raw evidence is saved in [records.jsonl](results/milestone1-2026-11-01/records.jsonl); the frozen configuration and consistency fingerprint are in [manifest.json](results/milestone1-2026-11-01/manifest.json). The [Rust summary](results/milestone1-2026-11-01/summary.json) reports `milestone_complete=true`, and the [independent check](results/milestone1-2026-11-01/VERIFICATION.json) confirms all nine groups of 50 cases. The ledger contains 459 call outcomes: 450 final API responses and nine earlier timeout errors, all on NVIDIA requests and all recovered. The summary's zero outstanding API errors does not mean the run was uninterrupted. Durations of calls that timed out were not saved; their earlier waits and resume pauses are excluded from the means for completed calls.

NVIDIA returned 11 empty final answers, each with 2,048 reported completion tokens. Exhausting the common output allocation is a plausible explanation, but the adapter did not capture `finish_reason`, so the cause is not confirmed. Empty final answers count as rubric failures. They remain distinct from transport timeouts.

A fresh baseline experiment uses `scripts/coach.ps1 evaluate --models all --runs 3 --approach baseline`. `scripts/coach.ps1 summarize --out results/milestone1-2026-11-01` reads historical provenance without overwriting evidence or claiming the current code matches the original fingerprint.

### Iteration log

| Date | What changed | Score | What was learned |
|---|---|---|---|
| 2026-11-01 | Baseline: unchanged prompt grounded in sources and rubric for 50 cases across three models; three repetitions per model complete | Means: 20B 79.33%; 120B 85.33%; NVIDIA 83.33% | Groq GPT OSS 120B provides the strongest measured balance of score and latency. Strict grades include real failures to meet the output contract and limitations of the rubric. |
| 2026-11-01 | Recovered interrupted NVIDIA calls through checkpoint resume; no generation settings changed | Same frozen experiment; not a new quality iteration | Saved responses do not need to be repeated. Recovered timeouts must be reported separately from the final summary of the latest responses. |
| 2026-11-01 | Inspected diverse saved outputs without changing their code grades | Scores unchanged | A valid paraphrase can fail, an extra relevant citation can fail, and an incomplete quiz can pass. The current metric is not factual accuracy. |
| 2026-11-08 | Fresh original prompt baseline; one run of the same 50 cases | 39/50, 78% | Original quotation and quiz failures recurred. This is a fresh baseline, separate from the historical three run mean. |
| 2026-11-08 | Added explicit task completion and contiguous quotation instructions | 43/50, 86% | Six cases gained passes and two regressed: a net gain of eight percentage points. |
| 2026-11-08 | Kept the grounded prompt and changed full context to lexical retrieval | 43/50, 86% | Same aggregate score with 36.03% fewer input tokens and all expected references retained. |
| 2026-11-08 | Added output guards, instance locking, installation, and dimensional UI | No new model quality score claimed | Application checks are separate from the controlled model study. |

### Milestone 2 improvement and graduate comparison

The new study completed 150 unique responses with no API errors: the same 50 inputs once under each configuration. [Independent aggregation](results/milestone2-2026-11-08/VERIFICATION.json) checks coverage, scores, timings, and equal dataset, source, scorer, and generation settings. [The CSV table](results/milestone2-2026-11-08/graduate_comparison.csv) exports all three configurations.

| Configuration | Strict passes | Mean model response time | Mean input tokens | Estimated cost per 1,000 requests |
|---|---:|---:|---:|---|
| Original prompt, full context | 39/50, 78% | 0.924 s | 928.9 | $0 only within free quota; billing unverified |
| Grounded prompt, full context | 43/50, 86% | 1.002 s | 1,076.9 | $0 only within free quota; billing unverified |
| Grounded prompt, lexical retrieval | 43/50, 86% | 0.933 s | 688.9 | $0 only within free quota; billing unverified |

The prompt change improved strict score by eight percentage points. E09, E20, E30, E32, E45, and E50 changed from fail to pass; E13 and E14 changed from pass to fail. Inspection shows that E13 and E14 correctly explain false positives and false negatives but use wording missed by the frozen concept checks. This demonstrates a rubric limit, not a measured loss of factual knowledge.

Baseline E20 again inserted an ellipsis in a quotation; grounded E20 copied a contiguous passage. Baseline E30 omitted the intervening FN sentence; grounded E30 used an exact precision sentence. Baseline E29 returned only a definition while passing the rubric; both grounded configurations supplied a question and answer. E29 therefore improved without changing its numeric grade. [The analysis](results/milestone2-2026-11-08/ANALYSIS.md) links these saved responses.

With the grounded prompt unchanged, retrieval matched the full context score while reducing mean input tokens by 36.03%. It selected 3.88 sources per request on average and retained all 37 expected references across the 36 answer cases. Reference coverage is computed after selection, not used in ranking. Mean retrieval time was 0.158 ms; mean retrieval plus model response time was 0.933 s. E06 gained a strict pass while E09 lost one; E09 is correct but its phrase `a given input` misses expected wording.

The installed application keeps the grounded prompt and full context as defaults. Retrieval reduces context but gives no aggregate score gain in this run. Its ASCII overlap cannot establish relevance for unseen synonyms or other languages. Keeping all sources avoids that omission risk while the library is small. This is a conservative design choice informed by the tie, not proof that full context is superior. `--context retrieval` uses the same selector and core function as the retrieval evaluator.

Each configuration has one run on a visible development set, so output ranges and statistical significance are unmeasured here. Baseline then grounded collection was sequential; retrieval overlapped part of grounded collection. Provider variation and concurrent load can affect latency, so timing differences cannot be attributed causally to the prompt or retrieval. Milestone 1's three runs per model remain separate. No unseen final test or learning outcome claim is made.

### Three observed application failures in the chosen model

These examples come from the first run of the chosen Groq GPT OSS 120B model. The rubric's original grades are preserved; E29 is a task failure that the code incorrectly marked as passing.

1. **E20, undefined precision.** Input: `TP=0 and FP=0. What precision value do the notes define?` The answer correctly says precision is undefined, but its S6 quotation contains `Precision = TP / (TP + FP). ... A metric with a zero denominator is undefined.` That combined string is not a contiguous source quotation. This is a failure to meet the citation contract, not an arithmetic error. The likely cause is normal summarization behavior being applied inside a field that requires exact copying. First change: validate quotations in the path that shows answers to the user and request a contiguous exact excerpt when validation fails, while retaining the original attempt for evaluation. Evidence: [records.jsonl, line 59](results/milestone1-2026-11-01/records.jsonl#L59).

2. **E29, incomplete quiz.** Input asks for one question about unit tests and its answer. Actual answer: `A unit test checks one function in isolation.` The definition is correct, but no question is generated. The code still passes it because it checks concepts and citations without checking completion of both requested parts. This is a genuine task failure and a rubric false pass. First change: give quiz responses separate question and answer fields and validate both, using independent examples before another controlled comparison. Evidence: [records.jsonl, line 86](results/milestone1-2026-11-01/records.jsonl#L86).

3. **E30, stitched quotation.** Input asks for a precision practice question with TP=3 and FP=1 and a worked answer. The model supplies the exercise and correctly computes `3 / (3 + 1) = 0.75`, but its S6 quote joins the TP, FP, and precision sentences while silently omitting the intervening FN sentence. That quotation fails the check for an exact substring. The likely cause is selecting relevant facts and presenting them as one continuous quote. First change: explicitly require a single contiguous passage and add this response with nonadjacent sentences as a regression case for quotation validation. Evidence: [records.jsonl, line 89](results/milestone1-2026-11-01/records.jsonl#L89).

The [qualitative audit](results/milestone1-2026-11-01/QUALITATIVE_AUDIT.md) also records false failures on valid wording and extra supporting sources. Any revised criteria or prompt must be versioned and applied consistently to every model; the present scores will not be rewritten as estimates of factual accuracy.

## 5. Limits and remaining research

The cases form a visible development set, not an unseen final test or evidence of improved learning. Phrase checks can reject valid paraphrases and extra citations while accepting an incomplete quiz. Future work should version task completion checks, assess semantic support, and use an independent final set with actual users' permission.

Sources are sent in full by default, with a 64 source and 180 KiB library limit. The optional lexical retrieval mode is described in Section 4. Plain text import does not extract PDF slides, images, audio, or Office files. This is a local single user application. Credentials are not encrypted; malware running as the same account is outside its protection. Provider terms, availability, quotas, and billing eligibility remain the user's responsibility. Two attack examples do not establish general prompt injection resistance.

The supplied Milestone 2 sheet names sections 2 to 4 but does not give all six final section titles. Sections 1, 5, and 6 here organize the remaining material; alignment with a separate final rubric is unverified.

## 6. Reproduction, demo, and submission status

The README covers installation, key setup, one command evaluation, approach comparison, and tests. The dataset exists as [JSON](data/eval.json) and [CSV](data/eval.csv), with CSV exports of all 450 final Milestone 1 responses and the three model comparison. JSON remains the runtime input. The method follows Huyen's public chapter summaries; the full book chapters were not supplied. The assignment's unusual `about 500 pages` wording was interpreted as approximately 500 words for the evaluation method, pending instructor clarification.

[The recorded three minute walkthrough](docs/demo/study-coach-demo.mp4) shows the actual application, one real Groq answer with a checked quotation, local handling of `???`, source creation, saved history, and connection setup. It includes burned captions and [an SRT subtitle file](docs/demo/demo.srt); [the transcript](docs/demo/TRANSCRIPT.md) records timings and limitations. This is a backup recording, not a claim that the classroom demo has been delivered. The student must still present and note classmates' demos.

The submission snapshot is identified by [the alpha tag](https://github.com/Charlesganu2004/study-coach/tree/alpha). The commit helper checks account identity, complete evidence, and stored author information, and refuses to overwrite an existing tag. It disables Git paging so the PowerShell workflow cannot pause at an `(END)` screen.

[Milestone 1 peer review status](docs/PEER_REVIEW.md) links Charles’s reviews of James and Luis and his submitted author response to James. Future held-out tests and repeated comparisons discussed in that response are separate from the recorded milestone measurements.

The repository contains source code, CSV exports, measured result evidence, packaging scripts, verification receipts, and the captioned demo. The Windows ZIP remains a local distributable rather than a published GitHub release. Classroom delivery, classmate observations, and the incoming author response remain separate requirements.

### November 9 application lifecycle update

Version 0.1.1 adds an X labelled **Close Study Coach** at the top right. The control remains visible while scrolling, stops the local service, releases its data folder lock, and preserves saved notes, history, settings, and keys for reopening. Closing waits for a current answer to finish. Unsaved notes or connection edits require confirmation; a browser that refuses tab closure shows a closed page with reopening instructions.

The [lifecycle verification](docs/LIFECYCLE.md) records 59 passing Rust tests, 10 lifecycle browser checks, the existing 18 presentation checks, and the verified new Windows package. Its error and provider response fixtures are explicitly labelled. No new provider requests or evaluation measurements were made. The original `alpha` submission snapshot remains unchanged; this application fix is a separate update.
