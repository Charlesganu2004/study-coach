# Evaluation artifacts

The completed baseline is `milestone1-2026-11-01/`: 450 unique completed API responses across three models and three runs each. Its original JSON evidence was already committed in `3d10051220aec37c2fa1ae9337d76bcd4c2d5798`. Later Markdown edits, CSV exports, and Milestone 2 application work are separate pending changes until the owner commits and pushes them.

The evaluator creates one directory per experiment with `manifest.json`, `records.jsonl`, `summary.json`, and `MODEL_SELECTION.md`. These directories are ignored by default. Inspect them for private content before explicitly adding the exact experiment directory to your class repository. Bundled evaluation inputs contain only original synthetic material.

Retain raw records with a submitted report so reported scores, timing, and failure analyses can be checked. Do not label reference answer checks or local transport tests as Groq/NVIDIA benchmark results.

`VERIFICATION.json` independently confirms all nine groups of 50 cases. The ledger retains nine recovered NVIDIA timeouts in addition to the 450 final responses. `QUALITATIVE_AUDIT.md` explains observed model and grader failures; its observations do not change the frozen numeric grades. `METHOD_DRAFT.md` is an intermediate writing artifact superseded by the method section in `REPORT.md`.

CSV exports in the baseline directory contain 450 final response rows and a three model comparison. Earlier error outcomes remain in JSONL rather than being counted twice in the final response export.

`milestone2-2026-11-08/` holds the separate baseline and grounded prompt approaches. Each has its own manifest, raw ledger, and summary. The final comparison uses `complete=true` for both approaches; `milestone_complete` is a legacy flag for Milestone 1's specific three model, three run requirement. Do not treat that flag as the Milestone 2 completion rule. The root comparison files are written only after both approaches finish.
