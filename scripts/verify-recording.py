"""Verify UHD frames and align walkthrough chapters to the visible progress bar."""

import json
import pathlib
import statistics
import subprocess
import tempfile


def timecode(seconds):
    milliseconds = round(seconds * 1000)
    hours, remainder = divmod(milliseconds, 3_600_000)
    minutes, remainder = divmod(remainder, 60_000)
    seconds, milliseconds = divmod(remainder, 1000)
    return f"{hours:02d}:{minutes:02d}:{seconds:02d}.{milliseconds:03d}"


def main():
    directory = pathlib.Path(__file__).resolve().parents[1] / "docs" / "demo"
    stem = "study-coach-upgrade"
    video = directory / (stem + ".webm")
    receipt_path = directory / (stem + "-verification.json")
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    chapters = receipt["chapters"]
    count = len(chapters)
    with tempfile.TemporaryDirectory(dir=video.parent) as temporary:
        indexed = pathlib.Path(temporary) / video.name
        frames = json.loads(subprocess.check_output([
            "ffprobe", "-v", "error", "-threads", "2", "-show_frames", "-show_entries",
            "frame=width,height", "-of", "json", str(video)]))["frames"]
        assert frames and all((f["width"], f["height"]) == (3840,2160) for f in frames)
        subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-i", str(video),
                        "-map", "0:v:0", "-c", "copy", str(indexed)], check=True)
        metadata = json.loads(subprocess.check_output([
            "ffprobe", "-v", "error", "-show_entries",
            "stream=codec_type,codec_name,width,height:format=duration:packet=pts_time",
            "-of", "json", str(indexed)]))
        stream, = metadata["streams"]
        assert (stream["codec_type"], stream["width"], stream["height"]) == ("video", 3840, 2160)
        times = [float(packet["pts_time"]) for packet in metadata["packets"]]
        duration = float(metadata["format"]["duration"])
        strip = subprocess.check_output([
            "ffmpeg", "-nostdin", "-v", "error", "-threads", "2", "-i", str(indexed),
            "-vf", f"crop=3840:4:0:2156,scale={count}:1:flags=area,format=gray",
            "-fps_mode", "passthrough", "-f", "rawvideo", "-threads", "1", "-"])
        assert len(strip) == len(times) * count, "Decoded frame count differs from video packets."
        starts = {}
        for i, pts in enumerate(times):
            row = strip[count*i:count*(i+1)]
            chapter = sum(value > 115 for value in row)
            if chapter == len(starts) + 1:
                starts.setdefault(chapter, pts)
        if list(starts) != list(range(1, count + 1)):
            raise ValueError(f"Expected all {count} chapters in order, saw {list(starts)}.")
        indexed.replace(video)
    for n, chapter in enumerate(chapters, start=1):
        chapter.update(start=starts[n], end=starts.get(n+1, duration))
    vtt = ["WEBVTT", ""]
    transcript = ["# Study Coach upgrade walkthrough", "",
                  "This recording follows the actual application using a labelled local demonstration model.", ""]
    for n, chapter in enumerate(chapters, start=1):
        start, end = timecode(chapter["start"]), timecode(chapter["end"])
        vtt.extend([str(n), f"{start} --> {end}", chapter["title"], chapter["caption"], ""])
        transcript.extend([f"## {start} {chapter['title']}", "", chapter["caption"], ""])
    (directory / (stem + ".vtt")).write_text("\n".join(vtt), encoding="utf-8")
    (directory / (stem + "-transcript.md")).write_text("\n".join(transcript), encoding="utf-8")
    receipt.update(duration_seconds=duration, size_bytes=video.stat().st_size, encoded_frames=len(times),
                   average_encoded_fps=round(len(times)/duration, 2), chapter_times="decoded visual progress bar")
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    proof = dict(stream, duration_seconds=duration, decoded_frames=len(times), chapters=count,
                 audio_tracks=0, average_encoded_fps=receipt["average_encoded_fps"],
                 median_interframe_ms=round(statistics.median(b-a for a,b in zip(times,times[1:]))*1000,1))
    (directory / "upgrade-checks" / "playback.json").write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: {len(times)} native UHD frames, all {count} chapters visible and seekable, duration {duration:.2f}s.")


if __name__ == "__main__":
    main()
