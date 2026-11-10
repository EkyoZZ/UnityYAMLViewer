import hashlib
import json
import struct
import sys
import tempfile
import urllib.request
from pathlib import Path

component, language = sys.argv[1:]
sys.path.insert(0, component)
SAMPLE_RATE = 16000


def timestamp(milliseconds):
    seconds, ms = divmod(milliseconds, 1000)
    minutes, second = divmod(seconds, 60)
    hour, minute = divmod(minutes, 60)
    return f"{hour:02}:{minute:02}:{second:02},{ms:03}"


def add_cue(cues, text, start_time, duration):
    text = " ".join(text.split())
    if text:
        start = max(0, round(start_time * 1000))
        end = max(start + 1, round((start_time + duration) * 1000))
        cues.append(f"{len(cues) + 1}\n{timestamp(start)} --> {timestamp(end)}\n{text}\n\n")


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
            if size == 0 or (response.headers.get("Content-Length") and
                             size != int(response.headers["Content-Length"])):
                raise ValueError(f"Incomplete model download: {name}")
        if sha256 and digest.hexdigest() != sha256:
            raise ValueError(f"Model checksum mismatch: {name}")
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    return str(destination)


def sensevoice():
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
    config.silero_vad.max_speech_duration = 15.0
    config.sample_rate = SAMPLE_RATE
    vad = sherpa_onnx.VoiceActivityDetector(config, buffer_size_in_seconds=60)
    recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
        model=model, tokens=tokens, language=language, num_threads=2, use_itn=True,
    )
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
    for samples in pcm_chunks(config.silero_vad.window_size):
        vad.accept_waveform(samples)
        drain()
    vad.flush()
    drain()
    return cues


cues = sensevoice()
print("\nUNITY_SUBTITLE " + json.dumps("".join(cues)))
