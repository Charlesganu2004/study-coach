# Source-grounded study coach

A Rust study coach answers questions from supplied course notes, quotes its evidence, and compares free hosted models with a reproducible evaluation.

This is a standalone project with its own Git repository. Local folder: `C:\Users\Charl\Downloads\study-coach-rust`. The class repository remains separate.

GitHub repository: [Charlesganu2004/study-coach-rust](https://github.com/Charlesganu2004/study-coach-rust) (private).

Milestone 1 evaluation is complete locally: 50 examples, three runs per model, 450 unique completed API responses, measured scores/latency, conditional free-tier cost reporting, and saved evidence. **Selected model: Groq `openai/gpt-oss-120b`**, with 85.33% strict passes and 0.899-second mean completed-call latency. See [REPORT.md](REPORT.md) for all results, failure analyses, limitations, and remaining submission steps.

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

`validate` checks the 50-case dataset and confirms the reference answers satisfy their rubrics. This is a dataset integrity check, **not a model evaluation**. Offline checks need no API keys. Initial compilation may download the locked crates if they are not already cached.

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

Fresh clones can create `.env` with the two entries shown above, or run `scripts/configure-keys.ps1` for hidden local prompts. The app loads `.env` automatically; no additional dependency is needed. Nonempty process environment values take precedence over `.env.local`, which takes precedence over `.env`. Blank values do not mask populated values from a lower-priority file. Both credential files are ignored by Git. Never paste credentials into chat, model inputs, or result files.

## Ask one question

```powershell
.\scripts\coach.ps1 answer --model groq-120b --question "How does a unit test differ from an integration test?"
```

Portable equivalent:

```sh
cargo run --locked --release -- answer --model groq-120b --question "How does a unit test differ from an integration test?"
```

The reusable library function is `study_coach::core::answer(input, notes, settings)`. It takes one user question plus shared course/configuration context, calls the selected model, and returns response text, parsed JSON when valid, latency, attempts, and token usage when supplied. The CLI and evaluator call this same function. A future interface can use it too.

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

Allowed statuses are `answer`, `clarify`, and `abstain`. Clarification and abstention require an empty citation array. The application sends the same complete small note collection on every request; this milestone does not implement retrieval or document ingestion.

## Run the full evaluation in one command

```powershell
.\scripts\coach.ps1 evaluate --models all --runs 3
```

Portable equivalent:

```sh
cargo run --locked --release -- evaluate --models all --runs 3
```

This schedules **450 requests: 50 cases × 3 models × 3 runs**. Each result is flushed to disk. Provider rate limits can make this take hours or require waiting until a daily quota resets. Groq currently lists 8,000 tokens/minute and 200,000 tokens/day for each selected model; account limits can differ. NVIDIA limits depend on account/model. [Groq rate limits](https://console.groq.com/docs/rate-limits), [NVIDIA availability](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b).

The evaluator stops on an API error, preserving progress. After resolving the error or waiting for quota, resume with the exact result directory printed by the command:

```powershell
.\scripts\coach.ps1 evaluate --models all --runs 3 --resume results/YOUR_RUN_DIRECTORY
```

Resume verifies the experiment fingerprint and retains prior responses. It retries failed API calls and skips saved completed calls. Changed prompts, scoring code, data, or model configuration require a new experiment. Optional `--limit 2 --runs 1` is a smoke test and cannot complete the milestone. `--out results/my-run` names a new experiment directory explicitly.

Switch profiles without editing code:

```powershell
.\scripts\coach.ps1 evaluate --models groq-120b --runs 3
.\scripts\coach.ps1 answer --provider nvidia --model-id nvidia/nemotron-3.5-lightning-30b-a3b --question "What does open-weight mean?"
```

Available aliases and shared sampling settings live in [models.json](models.json). Common settings: temperature 0.2, top-p 1.0, maximum output allocation 2,048 tokens, nonstreaming responses, and the same prompt and notes. Provider APIs use different token-limit field names. Internal reasoning defaults may differ; Groq's hidden reasoning is excluded from displayed text but still may consume tokens. No tools, web search, or paid fallback are enabled.

## Read completed results

Each experiment directory contains `manifest.json`, `records.jsonl`, `summary.json`, and `MODEL_SELECTION.md`. The manifest identifies the code/configuration/data snapshot with a noncryptographic consistency fingerprint. Raw responses, criteria failures, latencies, usage, and API errors remain inspectable. Keep results you submit in Git explicitly because generated result directories are ignored by default:

```sh
git add -f results/YOUR_RUN_DIRECTORY
```

The completed baseline is [results/milestone1-2026-11-01](results/milestone1-2026-11-01/summary.json). `REPORT.md` contains its final table and three inspected application failures, including a task-completion error missed by the rubric. `VERIFICATION.json` independently checks all 450 unique results. Nine earlier NVIDIA timeout outcomes remain in the ledger; completed-call latency excludes those earlier waits. The generator recommends a profile only after all expected responses are present with no outstanding API errors. Highest strict pass rate wins; mean response latency breaks ties.

The reported $0/1,000 requests is conditional on requests being covered by free account quotas. It is not an unlimited-use price or independently verified billing record. CPU/network/electricity costs are excluded. The selected models are open-weight, even though this application uses cloud inference. GPT-OSS uses Apache 2.0; NVIDIA's model card supplies its weight license. [OpenAI release](https://openai.com/index/introducing-gpt-oss/), [NVIDIA model card](https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16).

## Tests

```sh
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

On this machine prepend `+stable-x86_64-pc-windows-gnu` after `cargo` for check, clippy, and test. Tests cover real curl transport to a local mock server, credential-safe request construction, invalid responses, strict scoring, reference-answer integrity, and incomplete experiment handling. They make no calls to Groq or NVIDIA.

## Scope and limitations

The latest project choice replaces the earlier error-explainer direction in the separate class proposal. Hosted APIs send the supplied notes and question to the selected provider. This is **not offline or private local inference**; bundled notes and cases are original synthetic material.

The automatic rubric detects required phrases and verifies exact source quotations. It cannot prove that a quotation supports the answer or catch every contradiction. Review real outputs before making learning-quality claims. This is a development set; reserve unseen examples for final evaluation. See [dataset documentation](data/DATASET.md).
