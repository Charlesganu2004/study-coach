# Study Coach recorded demo

This subtitle script accompanies the actual three minute recording of the working Study Coach window. Windows Graphics Capture and FFmpeg captured the interface, a real model request, and real note and history actions. The captions provide written narration; no human voice was captured. They are aligned to the recorded interface event times rather than the earlier planned schedule.

## Recording sequence

| Time | Action on screen | Caption focus |
|---|---|---|
| 0:00 to 0:15 | Show Study | Purpose, Rust core, local storage, and cloud processing |
| 0:15 to 0:30 | Keep Study visible | Same 50 cases, scores, and mean input token reduction |
| 0:30 to 0:45 | Open the Test scope source at 0:31.352 | Original evidence and quotation checking |
| 0:45 to 1:00 | Bring the source into view at 0:47.224 and type the comparison question | The original passage and shared answer function |
| 1:00 to 1:10 | Send at 1:00.449; response appears at 1:08.019 | Actual selected Groq model and provider response |
| 1:10 to 1:30 | Read the answer and citation; evidence view at 1:27.074 | Exact source quotation and its limits |
| 1:30 to 1:55 | Enter `???` at 1:35.544; clarification appears around 1:48 | Local handling and no additional model request |
| 1:55 to 2:10 | Open My notes at 1:59.341 and save at 2:09.488 | Adding original material to the local library |
| 2:10 to 2:15 | Show the saved source | Source persistence and stable ID |
| 2:15 to 2:30 | Open history at 2:16.368 and restore the answer at 2:28.540 | Saved explanation and evidence |
| 2:30 to 2:50 | Open Connections at 2:34.766 | Own keys, blank key fields, NVIDIA alternative, and free quota limits |
| 2:50 to 3:00 | Return to Study around 2:52.2 | Closing view and development set limitations |

Use this original source text when adding Demo notes:

```text
A unit test checks one function in isolation. An integration test checks whether multiple components work together.
```

## Subtitle format

`demo.srt` contains 36 captions, each lasting five seconds. Each caption has at most two lines and no more than 84 text characters. The final caption ends at 00:03:00,000. The timestamp separators are required SRT syntax.

Outcome captions begin after the recorded response, clarification, save, and history restoration events. Earlier captions describe or request an action rather than claiming that it has already succeeded. The raw capture was verified as 180.000 seconds, 1920 by 1080 pixels, H264, and 30 frames per second with 5,400 frames. The server history confirmed one real model answer with a reported response time of 1,384.028 ms. The punctuation only clarification was local and not persisted as a model answer. The final encoded version adds a 120 pixel subtitle footer for a 1920 by 1200 frame, keeping captions clear of interface controls. Media probing and visual frame review must verify that encoded file separately.

The measured results are a 78% original prompt score, an 86% revised prompt score, and an 86% lexical retrieval score on the same 50 cases. Retrieval uses 36.03% fewer mean input tokens than the revised prompt with full context. Each Milestone 2 configuration has one run on a visible development set. These are strict rubric scores, not factual accuracy or measured learning outcomes.

The screen recording is a backup demonstration. The student must still deliver the required classroom demo and record observations of classmates. Do not describe the recording as proof that the classroom event has already occurred.
