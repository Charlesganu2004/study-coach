# Local verification receipt

Date: November 1, 2026, America/Chicago. Working directory: this project's root. Initial checks below ran before credentials were configured; the later live evaluation is documented separately in this receipt.

## Initial offline checks actually run

| Command | Observed result |
|---|---|
| `cargo fmt --check` | Exit 0 using the default installed formatter |
| `cargo +stable-x86_64-pc-windows-gnu check --offline --locked --all-targets` | Exit 0; `Finished dev profile` |
| `cargo +stable-x86_64-pc-windows-gnu clippy --offline --locked --all-targets -- -D warnings` | Exit 0; no warnings |
| `cargo +stable-x86_64-pc-windows-gnu test --offline --locked` | 23 library + 11 CLI unit + 1 summary CLI integration + 3 dataset + 2 dotenv CLI integration tests passed; 0 failed |
| `scripts/coach.ps1 validate` | Release build exit 0; `RUBRIC CHECK: 50/50 reference answers pass. This is not a model result.` |
| `scripts/coach.ps1 doctor` | Exit 1; curl available; both credential variables reported false |
| `scripts/coach.ps1 evaluate --models all --runs 3` | Exit 1 before network calls; missing local credentials |
| PowerShell parser checks for both helper scripts | `PowerShell helper syntax: PASS` |

The current dataset/prompt/code fingerprint after standalone project and `.env` support was `fnv1a64:497603742b808c6a`. This is a consistency identifier, not a cryptographic signature.

## Meaningful cases verified

* A real curl POST to a loopback mock HTTP server sends exact JSON and the dummy authorization header. Provider endpoints are never contacted by tests.
* Invalid settings, header injection, missing content, malformed JSON, and request fields specific to each provider are exercised.
* Citation fabrication, duplicates, missing sources, wrong response decisions, missing concepts, and forbidden phrases fail the scorer.
* Reversed false positive and false negative definitions, `$2,000` in place of `$2`, and `0.8%` in place of `80%` fail dataset regressions.
* Results from the wrong runs or duplicate coverage cannot be used to select a model. An actual CLI integration test rejects 450 synthetic checkpoint records for runs 4 to 6 when the manifest requests runs 1 to 3, and writes no model selection report. Those fixtures are test data, not benchmark results.
* All fifty reference answers satisfy their own criteria. This checks rubric consistency, not model performance or educational quality.
* The standard `.env` file is read without requiring `.env.local`. Tests verify nonempty process/local/base precedence, blank placeholders, UTF-8 BOM, quotes, comments, and parse errors that omit credential values. CLI tests use fake values and make no provider calls.

An independent review found the scorer and summary gate defects above. After corrections, the reviewer reran the two targeted regressions and found no remaining blocker in those corrections. This is not a security certification or verification against live providers.

## Environment findings

The default MSVC toolchain initially failed with `linker link.exe not found`. An existing GNU toolchain built and tested successfully. Its Clippy component was installed through the existing official rustup tool. The PowerShell wrapper chooses that existing GNU toolchain when MSVC's linker is unavailable; it never installs software.

After relocation to `C:\Users\Charl\Downloads\study-coach-rust`, one cached binary for an integration test retained the old executable path and failed with `The system cannot find the path specified.` The package's build cache was cleared after verifying its target directory, then static checks and all forty tests passed from the standalone folder. No test or hook was disabled.

Dependency review is recorded separately in [DEPENDENCY_AUDIT.md](DEPENDENCY_AUDIT.md). Microsoft Defender remained enabled.

## Live evaluation after credentials were configured

* All three configured models passed a real API smoke test with one case.
* The frozen comparison completed 450 unique API responses: 50 cases, three models, three runs per model. The ledger retains nine earlier timeout outcomes as well, for 459 recorded outcomes.
* `scripts/coach.ps1 summarize --out results/milestone1-2026-11-01` exited 0 and printed `milestone_complete=true`.
* Independent aggregation checked every expected case ID in all nine model/run groups and matched all means, ranges, and latencies in the Rust summary. Its output is `results/milestone1-2026-11-01/VERIFICATION.json`.
* Groq 20B: 119/150 strict passes, 79.33% mean, 78% to 80% range, mean latency of 0.863 seconds for completed calls.
* Groq 120B: 128/150 strict passes, 85.33% mean, 84% to 86% range, mean latency of 0.899 seconds for completed calls. Selected model.
* NVIDIA: 125/150 strict passes, 83.33% mean, 82% to 86% range, mean latency of 20.854 seconds for completed calls.
* All nine timeout outcomes were on NVIDIA requests and were recovered from checkpoints. Their durations were not saved and are excluded from latency for completed calls. Eleven NVIDIA final responses were empty, all at 2,048 reported completion tokens; these remain rubric failures.
* `REPORT.md` contains three observed application failures in the selected model, with links to raw evidence. One of those failures was a rubric false pass. Strict pass rate is not factual accuracy.

## Remaining limits

These are the limits of the November 1 study. Account billing and free tier eligibility are not independently verified; estimated $0/1,000 applies only within free quotas covered by the provider. Provider finish reasons and individual durations of failed calls were not captured. Real classroom learning outcomes and performance on an unseen final test set remain unmeasured. The original JSON evidence was committed in `3d10051220aec37c2fa1ae9337d76bcd4c2d5798`. The later prototype and packaging checks follow.

## November 8 prototype and distribution checks

| Check actually run | Observed result |
|---|---|
| `cargo fmt --check` | Exit 0 |
| `cargo +stable-x86_64-pc-windows-gnu check --offline --locked --all-targets` | Exit 0 |
| `cargo +stable-x86_64-pc-windows-gnu clippy --offline --locked --all-targets -- -D warnings` | Exit 0; no warnings |
| `cargo +stable-x86_64-pc-windows-gnu test --offline --locked --all-targets` | 40 library, 11 CLI unit, 2 CLI integration, 3 dataset, and 2 dotenv tests passed: 58 total, 0 failed |
| `node --check web/app.js` and both browser test scripts | Exit 0 |
| `node tests/ui-smoke.mjs` against the release executable | `PASS: 18 browser checks`; labelled presentation fixtures, no provider request |
| `node tests/live-ui.mjs PATH_TO_LOCAL_CREDENTIAL_FILE` | Real Groq 120B answer, 964.49 ms, exact quotation verified, history restored with no second request, no browser exceptions |
| `scripts/package.ps1` | Windows release archive built successfully |
| `scripts/test-package.ps1 -ArchivePath dist/StudyCoach-0.1.0-x86_64-pc-windows-gnu.zip` | `Package checks PASS` |
| `scripts/test-unix-installer.sh` under Git Bash | `Unix installer logic PASS`; native operating system binaries not tested |
| `scripts/commit-milestone.ps1 -CheckOnly` | Account, safety, and all three complete experiment checks passed; HEAD and index unchanged |

The final interface uses 3D transforms, orbital animation, pointer parallax, and a high density canvas. The browser test verifies changing transforms, system reduced motion, a persistent manual motion control, and no horizontal overflow at 390 pixels. It also checks plain text safety, Markdown import, source removal, persistence, key readback protection, history, bad input, and quota error recovery. Primary text contrast is 16.19:1; muted text on the question panel is 7.03:1; button text is 11.12:1; source tags are 8.26:1; input boundaries are 4.48:1. These are measured palette pairs, not a claim of a full accessibility certification.

The live test uses the explicitly selected credential file inside the Rust process. Keys are not typed into the browser, copied into the generated app credential file, or printed. The original credential file is checked unchanged. See [live receipt](evidence/ui/live-verification.json) and [browser receipt](evidence/ui/ui-verification.json). The separate [baseline receipt](evidence/prototype-baseline-2026-11-08.json) proves the prototype ran before the measured comparison.

Review reproduced a second instance overwrite bug. A retained operating system file lock now prevents a second process from loading the same data folder. A regression and independent real process check confirm refusal, release after termination, and source preservation after restart. See [concurrency receipt](evidence/concurrency-verification.json).

The final Windows archive SHA256 is `0f9ecbdd1f80daa59dff183d033fb66f28b506fba7566d5ffdd3f5b08f3c2b69`. [The package receipt](PACKAGE_VERIFICATION.json) verifies its six expected files, absence of credentials, system DLL imports only, installation without Rust, blank first launch keys, checksum tamper rejection, and removal that preserves user data. The ZIP is a local artifact, not a published release. Native Linux and macOS execution remains unverified.

## November 8 measured comparisons and CSV checks

The prompt comparison completed 100 responses: original prompt 39/50 strict passes, grounded prompt 43/50. The graduate retrieval arm completed another 50 with 43/50 strict passes. All 150 responses have the original case coverage and no API errors. The independent [verification file](../results/milestone2-2026-11-08/VERIFICATION.json) matches every aggregate and checks equal source, dataset, scorer, and generation settings. Retrieval retains 37/37 expected source references across all 36 answer cases. Reported latency is observed under sequential and overlapping streams, not a causal speed claim.

CSV round trip output: `PASS: CSV round trip: 50 cases, 450 unique final responses, 3 model summaries; text, score and timing checked.` The three configuration graduate CSV was also read back and checked against independently aggregated scores and token means. Historical data, scoring code, model settings, and Milestone 1 JSON evidence remain unchanged.

The source and saved evidence are published through the checked Git workflow, with the submission snapshot identified by `alpha`. The actual classroom demo and classmate observations remain separate. The incoming partner review is absent at the inspected commit; its author response cannot be fabricated. No account access change, paid fallback, or automatic GitHub release is part of this workflow.

## Recorded demo and publication checks

At the user's request, a dedicated Study Coach window was recorded for exactly 180 seconds. The [final MP4](demo/study-coach-demo.mp4) is H264, 1920 by 1200, 30 frames per second, and 5,400 frames. A 120 pixel caption bar below the original 1920 by 1080 recording contains 36 burned subtitle cues. [SRT](demo/demo.srt), [ASS](demo/demo.ass), and [the actual transcript](demo/TRANSCRIPT.md) are included. No microphone or system audio was recorded; captions provide written narration.

The recorded model request returned a real Groq 120B answer in 1,384.028 ms. The walkthrough shows its exact S3 quotation, a local clarification for `???`, source creation, restoration from history, and connection fields with no displayed key values. The complete final file decoded without errors. Independent visual review checked nine frames, including the complete answer and quotation, clarification, saved history, and blank key fields. [The video receipt](demo/VIDEO_VERIFICATION.json) records its hash and measurements. This is a backup recording, not a replacement for the student's classroom presentation.

The encoder was used locally from an isolated temporary review folder after its published checksum matched and a Defender scan completed. [The media tool review](demo/MEDIA_TOOL_REVIEW.json) identifies the provider, archive checksum, source reference, and limitations. It is not an application dependency or a redistributed binary. Git paging was disabled in the commit helper after the owner encountered an `(END)` pager before committing.
