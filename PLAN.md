# Milestone 1 plan

Build one of the student's requested projects in Rust: a source-grounded study coach using free Groq and NVIDIA hosted APIs. The existing initial proposal describes a programming error explainer in Python; this project follows the later explicit request to choose one of the three new candidates and use Rust. The earlier proposal remains available unchanged.

Done means: an installable Rust command, shared `answer` function, configurable providers/models, 50 validated evaluation examples, one-command evaluation, three distinct models evaluated three times each, measured scores/latency/cost, an evidence-based selection, and analysis of three actual failures in `REPORT.md`.

Constraints: free account access only; no paid fallback, private documents, generated benchmark results, or AI judge. Hosted inference is a change from the local-only brainstorm. Credentials must be entered locally by the student. Treat the assignment's “about 500 pages” as a likely typo for about 500 words, pending instructor clarification. The follow-up request authorizes placing this project in its own repository; create it private by default and exclude credential files.

1. Verify provider availability and audit the JSON dependency before building.
2. Independently build the API core and 50-case dataset against one schema.
3. Build and test deterministic scoring, CLI, result capture, and comparison reporting.
4. Run static checks, unit and transport tests, dataset validation, and credential preflight.
5. Run the full comparison when both API credentials are configured. Inspect actual failures and finish the report from saved results.

Verification: `cargo fmt --check`, `cargo check --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `cargo run --locked -- validate`, and the live evaluation command documented in README.

Stop only when all deliverables are verified, or name the exact external blocker. Missing credentials block live measurement and model selection, not local implementation and testing.

Progress: all five steps complete locally. All three models completed three 50-case runs. Independent aggregation confirms 450 unique final responses and nine retained timeout outcomes. Groq GPT-OSS 120B is selected at 85.33% strict passes and 0.899-second mean completed-call latency. The final table, method, iteration log, and three observed application failures are in `REPORT.md`. Updated files and result evidence still need to be committed and pushed for submission; `.env` remains local.

Follow-up: move the new, uncommitted project out of the class repository to `C:\Users\Charl\Downloads\study-coach-rust`, add an ignored blank `.env` with automatic loading, initialize an independent repository, verify the full suite and staged files, then create and populate its private GitHub repository under `Charlesganu2004`.

Follow-up progress: standalone folder, independent Git repository, blank ignored `.env`, documented credential loading, and forty passing tests are verified. Private GitHub repository created at `https://github.com/Charlesganu2004/study-coach-rust`; `origin` points there. No API credentials or live benchmark results are included in the initial project files.
