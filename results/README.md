# Evaluation artifacts

No live model results are committed yet. `baseline-status.json` records the initial external blocker; it is not a scored experiment.

The evaluator creates one directory per experiment with `manifest.json`, `records.jsonl`, `summary.json`, and `MODEL_SELECTION.md`. These directories are ignored by default. Inspect them for private content before explicitly adding the exact experiment directory to your class repository. Bundled evaluation inputs contain only original synthetic material.

Retain raw records with a submitted report so reported scores, timing, and failure analyses can be checked. Do not label reference-answer checks or local transport tests as Groq/NVIDIA benchmark results.
