# Local verification receipt

Date: November 1, 2026, America/Chicago. Working directory: this project's root. Initial checks below ran before credentials were configured; the later live evaluation is documented separately in this receipt.

## Initial offline checks actually run

| Command | Observed result |
|---|---|
| `cargo fmt --check` | Exit 0 using the default installed formatter |
| `cargo +stable-x86_64-pc-windows-gnu check --offline --locked --all-targets` | Exit 0; `Finished dev profile` |
| `cargo +stable-x86_64-pc-windows-gnu clippy --offline --locked --all-targets -- -D warnings` | Exit 0; no warnings |
| `cargo +stable-x86_64-pc-windows-gnu test --offline --locked` | 23 library + 11 CLI-unit + 1 summary CLI-integration + 3 dataset + 2 dotenv CLI-integration tests passed; 0 failed |
| `scripts/coach.ps1 validate` | Release build exit 0; `RUBRIC CHECK: 50/50 reference answers pass. This is not a model result.` |
| `scripts/coach.ps1 doctor` | Exit 1; curl available; both credential variables reported false |
| `scripts/coach.ps1 evaluate --models all --runs 3` | Exit 1 before network calls; missing local credentials |
| PowerShell parser checks for both helper scripts | `PowerShell helper syntax: PASS` |

The current dataset/prompt/code fingerprint after standalone-project and `.env` support was `fnv1a64:497603742b808c6a`. This is a consistency identifier, not a cryptographic signature.

## Meaningful cases verified

- A real curl POST to a loopback mock HTTP server sends exact JSON and the dummy authorization header. Provider endpoints are never contacted by tests.
- Invalid settings, header injection, missing content, malformed JSON, and provider-specific request fields are exercised.
- Citation fabrication, duplicates, missing sources, wrong response decisions, missing concepts, and forbidden phrases fail the scorer.
- Reversed false-positive/false-negative definitions, `$2,000` in place of `$2`, and `0.8%` in place of `80%` fail dataset regressions.
- Wrong-run and duplicate coverage cannot select a model. An actual CLI integration test rejects 450 synthetic checkpoint records for runs 4–6 when the manifest requests runs 1–3, and writes no model-selection report. Those fixtures are test data, not benchmark results.
- All fifty reference answers satisfy their own criteria. This checks rubric consistency, not model performance or educational quality.
- The standard `.env` file is read without requiring `.env.local`. Tests verify nonempty process/local/base precedence, blank placeholders, UTF-8 BOM, quotes, comments, and parse errors that omit credential values. CLI tests use fake values and make no provider calls.

An independent review found the scorer and summary-gate defects above. After corrections, the reviewer reran the two targeted regressions and found no remaining blocker in those corrections. This is not a security certification or a live-provider verification.

## Environment findings

The default MSVC toolchain initially failed with `linker link.exe not found`. An existing GNU toolchain built and tested successfully. Its Clippy component was installed through the existing official rustup tool. The PowerShell wrapper chooses that existing GNU toolchain when MSVC's linker is unavailable; it never installs software.

After relocation to `C:\Users\Charl\Downloads\study-coach-rust`, one cached integration-test binary retained the old executable path and failed with `The system cannot find the path specified.` The package's build cache was cleared after verifying its target directory, then static checks and all forty tests passed from the standalone folder. No test or hook was disabled.

Dependency review is recorded separately in [DEPENDENCY_AUDIT.md](DEPENDENCY_AUDIT.md). Microsoft Defender remained enabled.

## Live evaluation after credentials were configured

- All three configured models passed a real one-case API smoke test.
- The frozen comparison completed 450 unique API responses: 50 cases, three models, three runs per model. The ledger retains nine earlier timeout outcomes as well, for 459 recorded outcomes.
- `scripts/coach.ps1 summarize --out results/milestone1-2026-11-01` exited 0 and printed `milestone_complete=true`.
- Independent aggregation checked every expected case ID in all nine model/run groups and matched all means, ranges, and latencies in the Rust summary. Its output is `results/milestone1-2026-11-01/VERIFICATION.json`.
- Groq 20B: 119/150 strict passes, 79.33% mean, 78%–80% range, 0.863-second mean completed-call latency.
- Groq 120B: 128/150 strict passes, 85.33% mean, 84%–86% range, 0.899-second mean completed-call latency. Selected model.
- NVIDIA: 125/150 strict passes, 83.33% mean, 82%–86% range, 20.854-second mean completed-call latency.
- All nine timeout outcomes were on NVIDIA requests and were recovered from checkpoints. Their failed-call durations were not saved and are excluded from completed-call latency. Eleven NVIDIA final responses were empty, all at 2,048 reported completion tokens; these remain rubric failures.
- `REPORT.md` contains three observed application failures in the selected model, with links to raw evidence. One of those failures was a rubric false pass. Strict pass rate is not factual accuracy.

## Remaining limits

Account billing/free-tier eligibility is not independently verified; estimated $0/1,000 applies only within provider-covered free quotas. Provider finish reasons and individual failed-call durations were not captured. Real classroom learning outcomes and performance on an unseen final test set remain unmeasured. A UI and material import are later development work, outside the supplied Milestone 1 requirements. Updated local report/results still need to be committed and pushed before course submission.
