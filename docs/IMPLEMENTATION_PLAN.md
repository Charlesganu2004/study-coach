# Prototype and distribution plan

Updated November 8, 2026.

## Scope and completion checks

Build the supplied Milestone 2 prototype and finish the study coach as an installable local application. Keep the completed Milestone 1 evidence intact. The final application must let a student configure a free account, add notes, ask a question, inspect exact citations, and recover from bad input without using a terminal.

The installation must contain no developer credentials. First launch creates an application data directory and a private `.env` file. A setup screen accepts keys locally and links to the providers. Free account quotas are conditional, and the application must say so.

## Work sequence

1. Audit the new rubric, existing evaluation, and actual peer review. Preserve the 50 case baseline and CSV exports.
2. Build a Rust application service with local file storage, bounded requests, citation checks, and the same `core::answer` function as the evaluator. Verify the prototype before starting the measured comparison.
3. Compare the original prompt with an explicit grounding prompt using the selected Groq model and the same 50 cases. Also compare full context with lexical source retrieval under the same grounded prompt, giving a distinct graduate approach comparison. Save records, settings, errors, scores, source coverage, and latency. Milestone 2 uses one run of 50 cases per arm; Milestone 1 retains its required three runs per model. Report every outcome even if it does not improve.
4. Build a distributable package with a launcher, installation and removal scripts, and first launch configuration. Test installation in a temporary directory without copying the developer `.env` file.
5. Finish the browser interface last: setup, notes, study question, exact evidence, history, empty states, and actionable errors. Check desktop and phone layouts in a real browser.
6. Update the report, architecture diagram, version table, demo guide, verification evidence, and submission checklist. Prepare the alpha tag command without claiming the tag or classroom demo already exists.

## Ownership and boundaries

* The core module owns model requests and prompt selection. Both the command line evaluation and the application service call its public answer function.
* The application service owns credentials, notes, history, local session protection, and the browser API. Credentials never enter browser state, history, evaluation records, or packages.
* The interface owns presentation and sends explicit user actions to the service. It displays answer text as text rather than executable markup.
* The package owns installation and launch paths. User data lives outside the installation so an upgrade can retain it.
* Evaluation files own experiment evidence. The existing Milestone 1 result directory is historical and is never overwritten by the new experiment.

## Verification gates

Run Rust static checks before tests. Check API request limits, origin protection, source persistence, exact quotations, and credential suppression. Round trip CSV exports against JSON. Verify the package archive contents and launch an isolated installation. Exercise setup, notes, answers, history, and bad input through the actual browser.

## Work requiring the student or separate authorization

Software verification is complete locally: 58 Rust tests, 18 browser checks, a real browser model request, 150 new evaluation responses, independent CSV and aggregate checks, and the Windows package installation test. The final prompt scores are 78% baseline and 86% grounded; lexical retrieval also scores 86% with 36.03% fewer mean input tokens. The report and demo guide contain the measured evidence. The UI follows the user's later request for dimensional high motion visuals. The Windows ZIP is built; Unix installer logic is checked, with native Linux and macOS execution still unverified.

The supplied assignment requires a classroom demo, observations of classmates, and an actual peer review response. These cannot be fabricated. A missing or inaccessible review must be recorded. Commit, push, publication, and alpha tagging require explicit authorization or commands run by the owner. The full final project rubric has not been supplied, so completion against any additional final requirements remains unverified.
