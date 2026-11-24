# Study Coach walkthroughs

## Version 0.2.0

The [4K motion preview](demo/study-coach-4k-motion.mp4) shows the landing screen's vector illustration at 30 frames per second. It samples the application's own animation timeline at fixed intervals. The longer WebM below records the interactive study session and uses variable frame timing.

The [upgrade recording](demo/study-coach-upgrade.webm) follows a study session from adding a connection and importing course material through taking a test, reviewing missed answers and exporting illustrated notes. A local demonstration model supplies repeatable sample responses through the application's regular connection. [Captions](demo/study-coach-upgrade.vtt) · [Chapter transcript](demo/study-coach-upgrade-transcript.md) · [Verification](UPGRADE_VERIFICATION.md)

## Original milestone demo

## Before class

Install or extract the package and start `StartStudyCoach.cmd`. Keep its terminal open. In Connections, save a real free account key yourself and select Groq GPT OSS 120B. Confirm access with a short synthetic question. Do not show, record, or share a key. Keep the provider console and credential file off screen.

Prepare one original source in My notes: title `Workshop notes`, text `A unit test checks one function in isolation. An integration test checks whether multiple components work together.` If duplicate sources create extra supporting citations, use the bundled Test scope source instead. The development evaluation uses only the original bundled corpus, so custom demo notes are a separate example.

The [captioned backup recording](demo/study-coach-demo.mp4) is available with [subtitles](demo/demo.srt) and [an actual timing transcript](demo/TRANSCRIPT.md). It records the real application and one real Groq request. The separate UI test screenshots labelled browser presentation fixture are not model results. Practice the live walkthrough before class and check that provider access still works.

## Walkthrough

| Time | User action | What to explain |
|---|---|---|
| 0:00 to 0:30 | Open Study and expand the Test scope source | Notes and history are local. Asking sends the question and current source collection to the chosen provider. |
| 0:30 to 1:20 | Ask `How does a unit test differ from an integration test?` | This is a real request through the same Rust function used by the evaluator. Read the answer, then compare its quotation with the original source. |
| 1:20 to 1:50 | Ask `???` | The app requests a clearer question without a crash or API call. |
| 1:50 to 2:20 | Open My notes, then Past questions | Show plain text import or the prepared source and one saved answer. |
| 2:20 to 3:00 | Show the report's before and after table | Explain the one prompt change, fixed cases, measured score, and one remaining rubric limitation. |

If a provider is unavailable, show the error and explain it. Use the saved backup recording as a labelled backup. Do not present a mocked response, screenshot, reference answer, or prior response as a successful live request.

## After the demo

The student must actually give the demo in class and record observations of classmates: one thing that worked, one thing that broke, and one suggestion for each demo watched. Those observations are not yet known and cannot be filled in by the application. The source repository is public. The `alpha` tag identifies the original milestone snapshot.
