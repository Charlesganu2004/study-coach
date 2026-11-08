# Measured prototype comparison

All three configurations completed the same 50 cases once, giving 150 responses with no transport errors. The strict rubric, model, corpus, and exposed generation settings were fixed. [VERIFICATION.json](VERIFICATION.json) independently aggregates every case and checks equal controlled fields. [graduate_comparison.csv](graduate_comparison.csv) exports the table.

| Configuration | Passes | Strict score | Mean model time | Mean input tokens |
|---|---:|---:|---:|---:|
| Original prompt, all notes | 39/50 | 78% | 0.924 s | 928.9 |
| Grounded prompt, all notes | 43/50 | 86% | 1.002 s | 1,076.9 |
| Grounded prompt, lexical retrieval | 43/50 | 86% | 0.933 s | 688.9 |

## Targeted failures

The new baseline reproduced the three previously inspected problems. E20 inserted `...` into a quotation, E30 stitched nonadjacent sentences, and E29 supplied a definition without a quiz question. Both grounded configurations use contiguous quotes and produce the requested question and answer. E29's numeric grade stays passing because the frozen rubric already missed that task error.

Evidence for E20: [baseline line 20](baseline/records.jsonl#L20), [grounded line 20](grounded/records.jsonl#L20), [retrieval line 20](../milestone2-retrieval-2026-11-08/records.jsonl#L20).

Evidence for E29: [baseline line 29](baseline/records.jsonl#L29), [grounded line 29](grounded/records.jsonl#L29), [retrieval line 29](../milestone2-retrieval-2026-11-08/records.jsonl#L29).

Evidence for E30: [baseline line 30](baseline/records.jsonl#L30), [grounded line 30](grounded/records.jsonl#L30), [retrieval line 30](../milestone2-retrieval-2026-11-08/records.jsonl#L30).

## Changes beyond the target cases

Grounded full context gained E09, E20, E30, E32, E45, and E50, while losing E13 and E14: six gains minus two regressions, or four additional passes. The two regressions are correct explanations of false positives and false negatives whose phrasing misses the concept matcher. Their original grades remain unchanged. The primary score measures the strict contract rather than factual accuracy.

Retrieval exchanged an E06 failure for an E09 failure, leaving the aggregate unchanged. Its E09 answer says a system always produces the same output for `a given input`, with an exact S4 quotation. This is another wording failure in the rubric. All 37 reference source IDs across 36 answer cases were retained. This source coverage does not prove that the response uses every fact correctly or generalizes to other material.

## Tradeoff and limits

Retrieval used 36.03% fewer mean input tokens and 3.88 selected sources per request. Mean selection time was 0.158 ms. It is available in the app with `--context retrieval`, but full context remains the default because there is no observed score gain and the lexical method does not handle semantic similarity. The grounded prompt is the selected prompt because it improves the measured score and fixes all three inspected task or quotation problems.

Each configuration has a single run. The 78% fresh baseline is not a replacement for Milestone 1's 85.33% three run mean. The two prompt runs were sequential and the retrieval stream overlapped part of the grounded run. Latency is observed, not a causal speed comparison. Cost is estimated at $0 per 1,000 only while covered by free quotas; account billing is unverified. The per arm `milestone_complete` flag refers to Milestone 1's three model, three run rule; the appropriate Milestone 2 coverage flag is `complete=true`.
