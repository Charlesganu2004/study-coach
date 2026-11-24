# Study Coach 0.2.0 verification

## Interface and motion

Edge rendered the study workspace at 3840×2160, 5120×2880 and 390×844. The screenshots match those pixel dimensions. Two separate illustration layers changed position between rendered frames. The operating system's reduced-motion preference stopped them. All six views were checked at phone and 4K widths. These checks measure browser output, and hardware refresh rates vary. [Resolution and motion receipt](demo/upgrade-checks/display-verification.json) · [5K workspace](demo/upgrade-checks/5k-study.png) · [Phone workspace](demo/upgrade-checks/phone-study.png)

## Practice and study materials

On Windows, Rust formatting and Clippy passed and the Rust suite reported 84 passing tests. Edge completed 49 end-to-end checks across 16 actual loopback model requests, plus 11 quiz and material UI test groups, 18 interface smoke checks, and 10 lifecycle checks covering launch, shutdown and data retention.

The browser suite starts a real Rust service with temporary app data and selects a local model fixture using the Connections screen. Quiz generation, free-text grading and material creation travel through the regular HTTP and curl transport. The receipt lists each checked interaction and provider call. [End-to-end receipt](demo/upgrade-checks/upgrade-verification.json)

Quiz checks cover answer secrecy before grading, partial-credit retries, terminal outcomes, explicit solution reveal, source selection and saved progress. Material checks cover editable notes and a relationship diagram, flashcards, memory aids, export and persistence after reopening. [Source-selection receipt](evidence/upgrade-documents/source-selection.json)

## Import and export

Windows import checks use synthetic Word documents, presentations, spreadsheets, PDFs and images. Text in screenshot and PDF fixtures was read with Windows OCR. The importer also extracted embedded subtitles from real MP4 and Matroska fixtures using FFmpeg. Users can supply a transcript when a recording lacks captions. [Video import receipt](evidence/upgrade-documents/video-caption-import.json)

The downloaded Word document contains editable paragraphs, exact source quotations and a vector diagram. Its ZIP structure, XML, drawing relationships and text were checked. Opening the document through Microsoft Word automation timed out, so native Word diagram layout and pagination remain unverified. The browser produced a PDF of the illustrated notes. The OneNote action supplies rich clipboard content or a downloadable HTML fallback for manual paste. [Sample Word document](demo/upgrade-checks/upgrade-notes.docx) · [Sample PDF](demo/upgrade-checks/upgrade-notes.pdf) · [Word rendering check](evidence/upgrade-documents/word-render-blocker.json)

## Windows distribution

The package test checks the ZIP contents, imported DLLs, per-user installation, launch on loopback, bundled notes, checksum rejection, and removal that preserves user data. The receipt includes the tested archive's SHA-256 digest. [Package receipt](evidence/upgrade-documents/windows-package.json)

## Walkthrough

The recorded session uses original sample notes and a labelled local model. Chapter timestamps, recording details and interactions are available alongside the video. [Walkthrough](demo/study-coach-upgrade.webm) · [Transcript](demo/study-coach-upgrade-transcript.md) · [Recording receipt](demo/study-coach-upgrade-verification.json)

All 24 chapter markers were located in decoded 3840×2160 frames. The interactive walkthrough uses variable frame timing and holds screens while the viewer reads them. The separate five-second motion preview renders the real illustration at 30 frames per second. [Walkthrough playback check](demo/upgrade-checks/playback.json) · [4K motion preview](demo/study-coach-4k-motion.mp4)
