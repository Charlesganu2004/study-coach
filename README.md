# Study Coach

A Rust study coach answers questions from supplied course notes, quotes its evidence, and compares free hosted models with a reproducible evaluation.

## Study workspace upgrade

The illustrated workspace scales to native 4K and 5K. The book, recall cards and orbital paths move independently. Turn motion off in the top bar or use your device's reduced-motion setting. [5K workspace](docs/demo/upgrade-checks/5k-study.png)

Create practice tests with 1 to 30 questions: multiple choice, open response, fill in the blank, or a typed custom format. Tests open a dedicated screen with hidden solutions, green correct feedback, yellow partial feedback with retries, red wrong/revealed answers, and a review list based on saved attempts.

Import text, Markdown, CSV/TSV, DOCX, PPTX, XLSX, PDF, screenshots and captions into editable source previews. Windows native extraction/OCR handles Office documents, PDFs and images. Videos use a supplied transcript or embedded captions when FFmpeg is installed; automatic speech transcription is not included. Choose or type note styles and document types, then save study notes with relationship diagrams, flashcards, or original acronym/song-text memory aids.

Download editable Word documents with diagrams, save the printable document as PDF, or copy formatted content for pasting into OneNote. Connections lists bundled models and six service templates, with add/select/remove for custom OpenAI-compatible model IDs and endpoints. Provider account access and charges depend on the selected service.

See [architecture and trust boundaries](docs/ARCHITECTURE.md), [upgrade plan](docs/UPGRADE_PLAN.md), and [verification](docs/UPGRADE_VERIFICATION.md). The [updated walkthrough](docs/demo/study-coach-upgrade.webm) uses a visibly labelled local model fixture; it demonstrates the interface and actual local transport, not cloud-model quality. The original real-cloud demonstration remains in the recorded-demo section below.

The application source is public. [Study Coach on GitHub](https://github.com/Charlesganu2004/study-coach)

Milestone 1 evaluation is complete locally: 50 examples, three runs per model, 450 unique completed API responses, measured scores/latency, conditional cost reporting within free quotas, and saved evidence. **Selected model: Groq `openai/gpt-oss-120b`**, with 85.33% strict passes and mean latency of 0.899 seconds for completed calls. See [REPORT.md](REPORT.md) for all results, failure analyses, limitations, and remaining submission steps.

## Install the application

Download and extract the [Windows ZIP](https://github.com/Charlesganu2004/study-coach/releases/download/v0.2.0/StudyCoach-0.2.0-x86_64-pc-windows-gnu.zip). Open the extracted folder and double-click `StartStudyCoach.cmd`. It starts the included service and opens Study Coach in your browser. For a Start menu shortcut, run `.\install.ps1` inside that folder. [Installation and removal guide](docs/INSTALL.md)

Use the X labelled **Close Study Coach** at the top right to stop the app. If an answer is running, closing waits for it to finish. Saved notes, history, settings, and keys remain available when you reopen the launcher. If the browser keeps the closed page open, close that tab with `Ctrl+W`. The X remains visible when browser controls are hidden.

First launch creates a separate application data folder and a blank `.env` file. Open **Connections**, choose a model, paste your own key, and save. Keys are never included in the package or returned to the browser. The local credential file is not an encrypted secret vault. Existing project keys are not automatically copied into an installation.

Use **My notes** to import your course material, check the extracted text, and save it. In **Practice test**, select the sources, question format and length for a test. In **Study materials**, choose the sources, output type and study style. Generation and free-text grading can send the relevant material to your selected model. Account quotas apply to hosted models.

Developers can start the same interface from source:

```sh
cargo run --locked --bin study-coach-app
```

On this Windows machine, use the installed GNU toolchain:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-x86_64-pc-windows-gnu run --locked --bin study-coach-app
```

The interface includes animated 3D geometry, pointer parallax, and a high density canvas background. Use the top bar's motion control to pause effects; your device's reduced motion preference always takes priority. Notes and answers remain readable without animation. Developers can explicitly reuse a local key file with `study-coach-app --credentials-file PATH`; it is read without copying it into app data.

## Install and check

Install Rust 1.89 or newer from [rustup's official site](https://rustup.rs/) and ensure `curl` is installed. Windows includes `curl.exe`. The project uses one pinned direct dependency, `serde_json`; [audit details](docs/DEPENDENCY_AUDIT.md) cover its locked graph. Run commands inside this folder.

On this Windows machine, the existing GNU toolchain works. The PowerShell wrapper finds Cargo and selects that installed toolchain when MSVC's `link.exe` is missing:

```powershell
.\scripts\coach.ps1 validate
```

On systems with a working default Rust toolchain:

```sh
cargo build --locked --release
cargo run --locked --release -- validate
```

`validate` checks the dataset of 50 cases and confirms the reference answers satisfy their rubrics. This is a dataset integrity check, **not a model evaluation**. Offline checks need no API keys. Initial compilation may download the locked crates if they are not already cached.

## Configure free access

Create keys yourself in [Groq Console](https://console.groq.com/keys) and [NVIDIA Build](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b). Use Groq's Free Plan and NVIDIA's Free Endpoint access. The program cannot inspect account billing settings. It never switches to a paid provider or alternative model automatically.

Windows users can enter keys through hidden local prompts:

```powershell
.\scripts\configure-keys.ps1
.\scripts\coach.ps1 doctor
```

Open the existing `.env` file in this project folder and enter your keys after the equals signs:

```dotenv
GROQ_API_KEY=
NVIDIA_API_KEY=
```

Fresh clones can create `.env` with the two entries shown above, or run `scripts/configure-keys.ps1` for hidden local prompts. The app loads `.env` automatically; no additional dependency is needed. Nonempty process environment values take precedence over `.env.local`, which takes precedence over `.env`. Blank values do not mask populated values from a file with lower priority. Both credential files are ignored by Git. Never paste credentials into chat, model inputs, or result files.

## Ask one question

```powershell
.\scripts\coach.ps1 answer --model groq-120b --question "How does a unit test differ from an integration test?"
```

Portable equivalent:

```sh
cargo run --locked --release -- answer --model groq-120b --question "How does a unit test differ from an integration test?"
```

The reusable library function is `study_coach::core::answer(input, notes, settings)`. It takes one user question plus shared course/configuration context, calls the selected model, and returns response text, parsed JSON when valid, latency, attempts, and token usage when supplied. The CLI, evaluator, and browser application service call this same function. The application checks schema, secret leakage, source IDs, and exact quotations before showing a model answer.

Model output has this schema:

```json
{
  "status": "answer",
  "answer": "A unit test checks a function in isolation.",
  "citations": [
    {"source_id": "S3", "quote": "A unit test checks one function in isolation."}
  ]
}
```

Allowed statuses are `answer`, `clarify`, and `abstain`. Clarification and abstention require an empty citation array. The application sends its current complete note collection by default; `--context retrieval` enables the lexical selector measured in Milestone 2. The evaluation uses the fixed bundled collection. Rich source import is described above; embeddings are not implemented.

## Run the full evaluation in one command

```powershell
.\scripts\coach.ps1 evaluate --models all --runs 3 --approach baseline
```

Portable equivalent:

```sh
cargo run --locked --release -- evaluate --models all --runs 3 --approach baseline
```

This schedules **450 requests: 50 cases × 3 models × 3 runs**. Each result is flushed to disk. Provider rate limits can make this take hours or require waiting until a daily quota resets. Groq currently lists 8,000 tokens/minute and 200,000 tokens/day for each selected model; account limits can differ. NVIDIA limits depend on account/model. [Groq rate limits](https://console.groq.com/docs/rate-limits), [NVIDIA availability](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b).

The evaluator stops on an API error, preserving progress. After resolving the error or waiting for quota, resume with the exact result directory printed by the command:

```powershell
.\scripts\coach.ps1 evaluate --models all --runs 3 --approach baseline --resume results/YOUR_RUN_DIRECTORY
```

Resume verifies the experiment fingerprint and retains prior responses. It retries failed API calls and skips saved completed calls. Changed prompts, scoring code, data, or model configuration require a new experiment. Optional `--limit 2 --runs 1` is a smoke test and cannot complete the milestone. `--out results/my-run` names a new experiment directory explicitly.

Switch profiles without editing code:

```powershell
.\scripts\coach.ps1 evaluate --models groq-120b --runs 3
.\scripts\coach.ps1 answer --provider nvidia --model-id nvidia/nemotron-3.5-lightning-30b-a3b --question "What does open-weight mean?"
```

Available aliases and shared sampling settings live in [models.json](models.json). Common settings: temperature 0.2, top p 1.0, maximum output allocation 2,048 tokens, nonstreaming responses, and the same prompt and notes. Provider APIs use different names for token limit fields. Internal reasoning defaults may differ; Groq's hidden reasoning is excluded from displayed text but still may consume tokens. No tools, web search, or paid fallback are enabled.

## Compare the prototype approaches

```powershell
.\scripts\compare-approaches.ps1
```

This runs the selected Groq 120B model over the same 50 cases with each prompt strategy: `baseline` preserves the original instructions, while `grounded` adds explicit task completion and contiguous quotation checks. It keeps the rubric and exposed generation settings fixed and saves resumable evidence in `results/milestone2-2026-11-08`. The default is one run per approach, or 100 requests; `-Runs 3` enables repeated runs in a new output directory. The application defaults to `grounded`. Use `--approach baseline` or `--approach grounded` to switch the CLI without editing code.

The graduate comparison also tests full context against deterministic lexical retrieval while holding the grounded prompt fixed:

```powershell
.\scripts\compare-retrieval.ps1
```

This third arm uses the same 50 cases and shared model function, scores against the original complete notes, and records source coverage and retrieval time. Its source selection never reads reference answers. The application can use the same selector with `study-coach-app --context retrieval`; the default remains full context. Final scores determine the documented tradeoff rather than a claim that retrieval must win.

Run the comparison again with the same settings to resume after an API error. It writes its final `comparison.json` and `comparison.csv` only when both approaches have complete coverage. An improvement is an experimental question, not a guaranteed result. [The report](REPORT.md) gives the measured outcome and limitations.

## Read completed results

Each experiment directory contains `manifest.json`, `records.jsonl`, `summary.json`, and `MODEL_SELECTION.md`. The manifest identifies the code/configuration/data snapshot with a noncryptographic consistency fingerprint. Raw responses, criteria failures, latencies, usage, and API errors remain inspectable. Keep results you submit in Git explicitly because generated result directories are ignored by default:

```sh
git add -f results/YOUR_RUN_DIRECTORY
```

The completed baseline is [results/milestone1-2026-11-01](results/milestone1-2026-11-01/summary.json). `REPORT.md` contains its final table and three inspected application failures, including a task completion error missed by the rubric. `VERIFICATION.json` independently checks all 450 unique results. Nine earlier NVIDIA timeout outcomes remain in the ledger; latency for completed calls excludes those earlier waits. The generator recommends a profile only after all expected responses are present with no outstanding API errors. Highest strict pass rate wins; mean response latency breaks ties. `summarize` reads historical manifests without rewriting the saved evidence.

The canonical dataset is [data/eval.json](data/eval.json). Spreadsheet copies are [data/eval.csv](data/eval.csv), [450 final response rows](results/milestone1-2026-11-01/evaluation_results.csv), and [the three model comparison](results/milestone1-2026-11-01/model_comparison.csv). The evaluator reads JSON; the assignment explicitly allows either JSON or CSV.

The reported $0/1,000 requests is conditional on requests being covered by free account quotas. It is not a price for unlimited use or an independently verified billing record. CPU/network/electricity costs are excluded. The selected models provide open weights, even though this application uses cloud inference. GPT OSS uses Apache 2.0; NVIDIA's model card supplies its weight license. [OpenAI release](https://openai.com/index/introducing-gpt-oss/), [NVIDIA model card](https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16).

## Tests

```sh
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

On this machine prepend `+stable-x86_64-pc-windows-gnu` after `cargo` for check, clippy, and test. Tests cover real curl transport to a local mock server, request construction that protects credentials, invalid responses, strict scoring, integrity of reference answers, and incomplete experiment handling. They make no calls to Groq or NVIDIA.

Browser checks are real Playwright code in [tests/ui-smoke.mjs](tests/ui-smoke.mjs). They require Node, an available official Playwright installation, and a browser. Build `study-coach-app` first, then run `node tests/ui-smoke.mjs`. Windows checks use installed Microsoft Edge. Its answer and quota response are labelled fixtures. Run `node tests/lifecycle-ui.mjs` for close and reopen checks against two real local app processes, including saved data, unsaved edits, and failed shutdown recovery. This test uses labelled response and browser policy fixtures and makes no provider requests. Both scripts accept `STUDY_COACH_APP` to select a built executable. The optional `node tests/live-ui.mjs PATH_TO_LOCAL_CREDENTIAL_FILE` makes one real request through the UI, verifies its quotations, and checks history. [Verification details](docs/VERIFICATION.md) distinguish browser checks and real requests from model measurements.

## Demo and submission

[Watch the three minute screen recording](docs/demo/study-coach-demo.mp4), with visible captions and [downloadable subtitles](docs/demo/demo.srt). It shows a real model request through the application, exact evidence, bad input, source creation, history, and local key setup without displaying keys. [The transcript](docs/demo/TRANSCRIPT.md) documents actual timings. [The architecture](docs/ARCHITECTURE.md) shows the whole application.

Use [the alpha snapshot](https://github.com/Charlesganu2004/study-coach/tree/alpha) for the milestone submission. [Peer review status](docs/PEER_REVIEW.md) links Charles’s two outgoing reviews and his submitted response to James. The backup video does not replace the student's classroom demo or observations of classmates.

The commit helper checks the Git account and stored commit identity, excludes credentials, and refuses incomplete comparison evidence. Review its changes before running it:

```powershell
.\scripts\commit-milestone.ps1 -CheckOnly
.\scripts\commit-milestone.ps1 -TagAlpha -Push
```

The second command commits, creates the `alpha` tag without overwriting an existing tag, and pushes. The helper never rewrites history or moves an existing tag. The local package is not an automatically published GitHub release.

## Scope and limitations

Hosted models receive the notes and prompt needed for each request. A local OpenAI-compatible service can process requests on your own machine. Bundled notes and evaluation cases use original synthetic material.

The automatic rubric detects required phrases and verifies exact source quotations. It cannot prove that a quotation supports the answer or catch every contradiction. Review real outputs before making claims about learning quality. This is a development set; reserve unseen examples for final evaluation. See [dataset documentation](data/DATASET.md).
