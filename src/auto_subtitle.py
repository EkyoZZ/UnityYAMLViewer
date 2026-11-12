import hashlib
import json
import unicodedata
import struct
import sys
import tempfile
import urllib.request
from pathlib import Path

def ai_options(folder):
    base = Path(folder).parent
    settings = {"prefer_gpu": True, "cpu_threads": 2}
    try:
        settings.update(json.loads((base / "ai-runtime.json").read_text(encoding="utf-8")))
    except (OSError, ValueError, TypeError):
        pass
    import os
    settings["cpu_threads"] = max(1, min(int(settings.get("cpu_threads", 2)), 4, os.cpu_count() or 1))
    settings["dll_handles"] = []
    if os.name == "nt" and hasattr(os, "add_dll_directory"):
        for package in ("cublas", "cuda_runtime", "cudnn"):
            directory = base / "ai-gpu-runtime/nvidia" / package / "bin"
            if directory.is_dir():
                settings["dll_handles"].append(os.add_dll_directory(str(directory)))
                if str(directory) not in os.environ.get("PATH", "").split(os.pathsep):
                    os.environ["PATH"] = str(directory) + os.pathsep + os.environ.get("PATH", "")
    return settings


_AI_DLL_HANDLES = []


def asr_recognizer(factory, folder, **kwargs):
    options = ai_options(folder)
    _AI_DLL_HANDLES.extend(options["dll_handles"])
    kwargs["num_threads"] = options["cpu_threads"]
    import os
    runtime = Path(os.environ.get("UNITY_ASR_RUNTIME", str(folder)))
    cuda_provider = runtime / "sherpa_onnx/lib/onnxruntime_providers_cuda.dll"
    if options["prefer_gpu"] and cuda_provider.is_file():
        try:
            return factory(provider="cuda", **kwargs)
        except (RuntimeError, ValueError) as error:
            print("GPU recognition fallback: " + str(error), file=sys.stderr, flush=True)
    elif options["prefer_gpu"]:
        print("Recognition uses CPU: installed sherpa-onnx lacks a CUDA provider", file=sys.stderr, flush=True)
    return factory(provider="cpu", **kwargs)


SAMPLE_RATE = 16000


def timestamp(milliseconds):
    seconds, ms = divmod(milliseconds, 1000)
    minutes, second = divmod(seconds, 60)
    hour, minute = divmod(minutes, 60)
    return f"{hour:02}:{minute:02}:{second:02},{ms:03}"


def visual_width(text):
    return sum(2 if unicodedata.east_asian_width(char) in "WF" else 1 for char in text)


def subtitle_parts(text, limit=64):
    """Prefer complete clauses; bound CJK width and avoid cutting Latin words."""
    text = " ".join(text.split())
    parts = []
    while text:
        width = 0
        boundaries = []
        cut = len(text)
        for i, char in enumerate(text):
            width += visual_width(char)
            if width > limit:
                # A very early comma must not leave a tiny cue followed by a full one.
                eligible = [position for position, weight in boundaries if weight >= limit * 0.5]
                cut = eligible[-1] if eligible else max(1, i)
                break
            if char.isspace() or char in "，。！？；、,.!?;":
                boundaries.append((i + 1, width))
            if char in "。！？!?；;" and width >= min(24, limit * 0.6) and i + 1 < len(text):
                cut = i + 1
                break
        parts.append(text[:cut].strip())
        text = text[cut:].strip()
    return parts


def timed_parts(text, start, end):
    """VAD gives segment times, not word alignment: estimate clause times by width."""
    import math
    text = " ".join(text.split())
    if not text or not math.isfinite(start) or not math.isfinite(end) or end <= start:
        return
    # Long uninterrupted speech also gets a duration-driven boundary. Keep a
    # useful minimum amount of text instead of showing isolated characters.
    duration = end - start
    wanted = max(1, math.ceil(duration / 6.0))
    limit = min(64, max(16, math.ceil(visual_width(text) / wanted)))
    parts = subtitle_parts(text, limit)
    # Merge tiny punctuation tails when that still fits the normal width limit.
    if len(parts) > 1 and visual_width(parts[-1]) < 8 and visual_width(parts[-2] + parts[-1]) <= 64:
        tail = parts.pop()
        separator = " " if parts[-1][-1:].isascii() and parts[-1][-1:].isalnum() and tail[:1].isascii() and tail[:1].isalnum() else ""
        parts[-1] += separator + tail
    weights = [max(1, visual_width(part)) for part in parts]
    total = sum(weights)
    consumed = 0
    previous = start
    for index, (part, weight) in enumerate(zip(parts, weights)):
        consumed += weight
        boundary = end if index + 1 == len(parts) else start + duration * consumed / total
        yield part, previous, boundary
        previous = boundary


def add_cue(cues, text, start_time, duration):
    text = " ".join(text.split())
    if text:
        for part, a, b in timed_parts(text, start_time, start_time + duration):
            start = max(0, round(a * 1000))
            end = max(start + 1, round(b * 1000))
            cues.append(f"{len(cues) + 1}\n{timestamp(start)} --> {timestamp(end)}\n{part}\n\n")


def pcm_chunks(samples_per_read):
    while chunk := sys.stdin.buffer.read(2 * samples_per_read):
        yield [sample / 32768.0 for sample, in struct.iter_unpack("<h", chunk)]


def start_transcribing():
    print("UNITY_SUBTITLE_STAGE transcribe", file=sys.stderr, flush=True)


def model_file(name, url, limit, sha256=None):
    directory = Path(component) / "models"
    directory.mkdir(exist_ok=True)
    destination = directory / name
    if destination.is_file() and 0 < destination.stat().st_size <= limit:
        if sha256 is None or destination.stat().st_size == limit:
            return str(destination)

    digest = hashlib.sha256()
    size = 0
    with tempfile.NamedTemporaryFile(dir=directory, delete=False) as staging:
        temporary = Path(staging.name)
    try:
        with urllib.request.urlopen(url, timeout=60) as response, temporary.open("wb") as staging:
            while chunk := response.read(1024 * 1024):
                size += len(chunk)
                if size > limit:
                    raise ValueError(f"Model file exceeds download limit: {name}")
                digest.update(chunk)
                staging.write(chunk)
                print(f"Progress {size} of {limit}", file=sys.stderr, flush=True)
            if size == 0 or (response.headers.get("Content-Length") and
                             size != int(response.headers["Content-Length"])):
                raise ValueError(f"Incomplete model download: {name}")
        if sha256 and digest.hexdigest() != sha256:
            raise ValueError(f"Model checksum mismatch: {name}")
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    return str(destination)


def sensevoice_engine(language, max_speech_duration=6.0):
    import sherpa_onnx

    release = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/"
    repository = ("https://huggingface.co/csukuangfj/"
                  "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/"
                  "2365baeacb507f821a0c8120fcee3d484dba7a07/")
    model = model_file(
        "model.int8.onnx", repository + "model.int8.onnx", 239233841,
        "c71f0ce00bec95b07744e116345e33d8cbbe08cef896382cf907bf4b51a2cd51",
    )
    tokens = model_file("tokens.txt", repository + "tokens.txt", 1024 * 1024)
    config = sherpa_onnx.VadModelConfig()
    config.silero_vad.model = model_file("silero_vad.onnx", release + "silero_vad.onnx", 8 * 1024 * 1024)
    config.silero_vad.min_silence_duration = 0.3
    config.silero_vad.max_speech_duration = max_speech_duration
    config.sample_rate = SAMPLE_RATE
    config.num_threads = 1
    vad = sherpa_onnx.VoiceActivityDetector(config, buffer_size_in_seconds=60)
    recognizer = asr_recognizer(sherpa_onnx.OfflineRecognizer.from_sense_voice, component,
        model=model, tokens=tokens, language=language, use_itn=True,
    )
    return recognizer, vad, config.silero_vad.window_size


def sensevoice():
    recognizer, vad, window_size = sensevoice_engine(language)
    cues = []

    def drain():
        while not vad.empty():
            segment = vad.front
            samples = segment.samples
            start = segment.start / SAMPLE_RATE
            duration = len(samples) / SAMPLE_RATE
            stream = recognizer.create_stream()
            stream.accept_waveform(SAMPLE_RATE, samples)
            vad.pop()
            recognizer.decode_stream(stream)
            add_cue(cues, stream.result.text, start, duration)

    start_transcribing()
    for samples in pcm_chunks(window_size):
        vad.accept_waveform(samples)
        drain()
    vad.flush()
    drain()
    return cues


if __name__ == "__main__":
    component, language = sys.argv[1:]
    import os
    sys.path.insert(0, os.environ.get("UNITY_ASR_RUNTIME", component))
    cues = sensevoice()
    print("\nUNITY_SUBTITLE " + json.dumps("".join(cues)))
