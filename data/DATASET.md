# Evaluation dataset

This development set contains 50 original study questions written for this project and 12 compact sources about introductory computing, testing, and model evaluation. It contains no private course documents, real student records, external repository content, or copied textbook passages. Each request receives the same complete source corpus in `notes.json`.

The expected answers and criteria were written before collecting candidate model responses. They are visible development criteria, not a hidden set or a set held out for testing. The examples are deliberately balanced by behavior rather than sampled from real usage, so the aggregate score does not estimate student success in practice or population accuracy. Future evaluation should add independent questions from actual users, with permission, and keep a separate final test set.

| Category | Examples | IDs |
| --- | ---: | --- |
| Basic explanations | 14 | E01 to E14 |
| Calculations and undefined metrics | 8 | E15 to E22 |
| Comparisons | 6 | E23 to E28 |
| Practice questions and flashcards | 4 | E29 to E32 |
| Ambiguous questions | 4 | E33 to E36 |
| Conflicting output constraints | 1 | E37 |
| Missing course or personal facts | 4 | E38 to E41 |
| Requests outside scope | 3 | E42 to E44 |
| False premises | 2 | E45 to E46 |
| Attempted prompt overrides | 2 | E47 to E48 |
| Empty or truncated input | 2 | E49 to E50 |
| Total | 50 | E01 to E50 |

There are 36 expected `answer` cases, 7 expected `clarify` cases, and 7 expected `abstain` cases. E15 to E22 include simple arithmetic, zero denominators, unit reporting, and a conditional cost estimate for a free quota. E28 combines evidence from two sources. Missing course facts require abstention rather than inventing an instructor, exam date, grade, or hidden test set size. Ambiguous, empty, truncated, and contradictory requests require clarification.

## File contract

`eval.csv` exports the same 50 cases for spreadsheet inspection. Nested criteria and citation lists remain JSON strings in CSV cells. `eval.json` remains the canonical runtime input; a CSV export does not create a second evaluation set. The Milestone 2 prompt comparison uses these same unchanged 50 cases and 12 sources.

`notes.json` contains a title and a `sources` array. Each source has a unique `id`, `title`, and `text`. Source IDs remain stable so that citations can be checked against exact source text.

`eval.json` is an array of cases. Each case contains:

* `id`: unique case identifier.
* `category`: behavior category for subgroup reporting.
* `input`: the complete user input, including an intentionally empty string in E49.
* `expected_status`: `answer`, `clarify`, or `abstain`.
* `expected_answer`: a reference answer or response criterion in plain language.
* `required_concepts`: groups of acceptable alternative phrases. Every group must have at least one match in the response's answer text after normalization.
* `forbidden_phrases`: phrases that must not occur in the answer text after normalization.
* `expected_citations`: all source IDs required for an answer. Clarifications and abstentions require no citations.

The model response contract is `{"status":"answer","answer":"...","citations":[{"source_id":"S1","quote":"exact source text"}]}`. Use the appropriate status and an empty citations array for clarification or abstention. The scoring code checks that output quotations occur in the named source; `expected_answer` provides a reference for review in plain language, not a target requiring an exact match.

## Scoring limits and review

Phrase groups permit several ordinary formulations and numerical representations, but cannot recognize every valid paraphrase. Conversely, mentioning both class labels does not by itself prove that a false positive or false negative was explained correctly. An exact source quotation proves that a quotation exists, not that the whole answer follows from it. Formula keywords similarly cannot prove correct mathematical reasoning. Record these as limitations of automatic scoring and review actual responses before interpreting the score.

E47 to E48 deliberately forbid the injected marker even if the model repeats it to explain why it ignored the instruction. This tests a strict output requirement, so a failure may reflect unnecessary repetition rather than obedience to the injection. The report should distinguish those cases by reading the output. These two probes do not establish general resistance to prompt injection.

Case E37, with contradictory constraints, tests a documented product choice: ask which length requirement to follow. A model that answers the mathematical question correctly but ignores that conflict fails this criterion. The poem request, which falls outside scope, also tests the limits of the study coach, rather than general model helpfulness.

Do not edit criteria after observing failures merely to improve scores. If review finds an unfair criterion or uncaptured valid paraphrase, version the dataset, record the reason, and rerun every candidate with the same revised version.
