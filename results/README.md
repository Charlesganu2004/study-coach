# Evaluation artifacts

The completed local baseline is `milestone1-2026-11-01/`: 450 unique completed API responses across three models and three runs each. `baseline-status.json` gives its status; `summary.json` and the retained raw records provide the measured evidence. These latest artifacts still need to be committed and pushed before submission.

The evaluator creates one directory per experiment with `manifest.json`, `records.jsonl`, `summary.json`, and `MODEL_SELECTION.md`. These directories are ignored by default. Inspect them for private content before explicitly adding the exact experiment directory to your class repository. Bundled evaluation inputs contain only original synthetic material.

Retain raw records with a submitted report so reported scores, timing, and failure analyses can be checked. Do not label reference-answer checks or local transport tests as Groq/NVIDIA benchmark results.

`VERIFICATION.json` independently confirms all nine 50-case groups. The ledger retains nine recovered NVIDIA timeouts in addition to the 450 final responses. `QUALITATIVE_AUDIT.md` explains observed model and grader failures; its observations do not change the frozen numeric grades. `METHOD_DRAFT.md` is an intermediate writing artifact superseded by the method section in `REPORT.md`.
