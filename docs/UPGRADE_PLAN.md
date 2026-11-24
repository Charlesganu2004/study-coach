# Study Coach upgrade, November 23, 2026

The workspace combines note-grounded answers, private practice tests, a saved study library, and user-configured model connections. Existing evaluation datasets and historical results remain reproducible.

## Acceptance criteria

- Dedicated test screen: multiple choice, open response, fill in the blank, and custom format instructions; 1 to 30 questions.
- Correct answers show green, partial answers show yellow with a retry, and wrong or revealed answers show red with a source-backed solution. Creating a test never sends its solutions to the browser.
- Review history distinguishes answered-well topics from missed and partly answered topics without claiming a validated mastery score.
- Connections list existing models and supported service templates, with add, select, and remove for custom OpenAI-compatible models. Saved keys are never returned to the browser.
- Import text, Markdown, CSV/TSV, captions, DOCX, PPTX, XLSX, PDF, and screenshots; extracted content is editable before saving. Windows native OCR handles images and rendered PDF pages. Videos require embedded captions and FFmpeg, or a supplied transcript.
- Generate and reopen notes, five-card recall sets, and original text memory aids. Select or type a note style, document type, and extra instructions.
- Notes include source-supported relationship diagrams. Export editable Word documents, print/save PDF, and copy formatted content for pasting into OneNote.
- Preserve notes, history, closing/reopening, keyboard access, visible status text, motion controls, reduced-motion preferences, and responsive layouts.
- Update architecture documentation and record an accurate walkthrough after the finished app passes browser checks.

## Boundaries

The Rust service owns validation, file persistence, credential handling, quiz answers, and grading. Browser code owns interaction, diagrams, printing, and clipboard export. The import helper runs only fixed extraction routines with bounded temporary files. CLI evaluation prompts and frozen result files are not rewritten to improve historical scores. No new runtime crates are introduced.

The model may grade semantically different free-text responses; its assessment is guidance and can be wrong. Exact supported answers and multiple-choice matching use local checks that preserve numeric signs and operators. Source quotations are validated before content is shown. Cloud requests happen only for explicit study actions.

## Verification

Run Rust formatting, static compilation, Clippy, and tests. Run the real-service integration suite against a clearly labelled local model fixture, the responsive browser suite, the existing lifecycle regressions, and native document/OCR checks. Inspect actual phone and 4K screenshots, rendered Word/PDF output, and recorded video frames. Record evidence in `UPGRADE_VERIFICATION.md`.
