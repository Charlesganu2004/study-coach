# Local verification receipt

Date: November 1, 2026, America/Chicago. Working directory: this project's root. No live Groq or NVIDIA requests were made.

## Checks actually run

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

## Still unverified

Provider authentication, actual free-account eligibility, model quality, provider response compatibility, real latency, usage/billing, three repeated model comparisons, final selection, and three observed chosen-model failures. The exact external blocker is absent `GROQ_API_KEY` and `NVIDIA_API_KEY`; keys must be entered locally by the student. `REPORT.md` explicitly marks these results as not measured.
