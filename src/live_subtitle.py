"""Local recognizers. Input: <f64 media time, u32 count>, then count f32 samples."""
import json
import math
import struct
import sys
from pathlib import Path


ZIP_REVISION = "98590b7ed6443e77b714204da2757d75e1a642f4"
ZIP_FILES = (
    ("encoder-epoch-99-avg-1.int8.onnx", 181895032, "8fa764187a261844f859d7143ebaa563af5d10adfece4c18a8f414c88cba2a9b"),
    ("decoder-epoch-99-avg-1.int8.onnx", 13091040, "1a70c593d71e53f023f5f55b0b4cfff5055abb786ee3992e5f63dc2e273cc4fa"),
    ("joiner-epoch-99-avg-1.int8.onnx", 3228404, "1ed689c5ed19dbaa725d9d191bb4822b5f4855a39e1ffd28cbc1f340d25b2ee0"),
    ("tokens.txt", 56317, None),
)


def emit(**event):
    print(json.dumps(event, ensure_ascii=True), flush=True)


def read_exact(stream, length):
    data = bytearray()
    while len(data) < length:
        chunk = stream.read(length - len(data))
        if not chunk:
            raise EOFError("Truncated subtitle audio packet")
        data.extend(chunk)
    return data


def packets(stream):
    while True:
        header = stream.read(1)
        if not header:
            return
        timestamp, count = struct.unpack("<dI", header + read_exact(stream, 11))
        if not math.isfinite(timestamp) or timestamp < 0 or count > 16000:
            raise ValueError("Invalid subtitle audio packet")
        if count == 0:
            return
        yield timestamp, struct.unpack(f"<{count}f", read_exact(stream, count * 4))


def prepare(helpers, root, engine, language):
    import sherpa_onnx

    if engine == "sensevoice":
        recognizer, vad, window = helpers["sensevoice_engine"](language, max_speech_duration=6.0)
        return recognizer, vad, window
    if engine != "zipformer" or language not in ("zh", "en"):
        raise ValueError("Unsupported recognizer language")
    directory = root / "zipformer-zh-en"
    directory.mkdir(parents=True, exist_ok=True)
    # Reuse the atomic, bounded downloader; keep this model separate from SenseVoice.
    helpers["component"] = str(root / "zipformer-zh-en")
    url = ("https://huggingface.co/csukuangfj/"
           "sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20/resolve/"
           + ZIP_REVISION + "/")
    files = [helpers["model_file"](name, url + name, size, digest)
             for name, size, digest in ZIP_FILES]
    return sherpa_onnx.OnlineRecognizer.from_transducer(
        encoder=files[0], decoder=files[1], joiner=files[2], tokens=files[3],
        num_threads=2, sample_rate=16000, decoding_method="greedy_search",
        model_type="zipformer", enable_endpoint_detection=True,
        rule1_min_trailing_silence=2.4, rule2_min_trailing_silence=0.8,
        rule3_min_utterance_length=12.0,
    )


def recognize_sensevoice(engine, source):
    recognizer, vad, window = engine
    pending = []
    origin = None
    previous_end = None

    def drain():
        while not vad.empty():
            segment = vad.front
            samples = segment.samples
            start = origin + segment.start / 16000
            end = start + len(samples) / 16000
            stream = recognizer.create_stream()
            stream.accept_waveform(16000, samples)
            vad.pop()
            recognizer.decode_stream(stream)
            text = " ".join(stream.result.text.split())
            if text:
                emit(start=start, end=end, text=text, final=True)

    for timestamp, samples in source:
        if previous_end is not None and abs(timestamp - previous_end) > 0.002:
            if pending:
                vad.accept_waveform(pending + [0.0] * (window - len(pending)))
            vad.flush()
            drain()
            vad.reset()
            pending.clear()
            origin = timestamp
        if origin is None:
            origin = timestamp
        previous_end = timestamp + len(samples) / 16000
        pending.extend(samples)
        while len(pending) >= window:
            vad.accept_waveform(pending[:window])
            del pending[:window]
            drain()
    if origin is not None:
        if pending:
            vad.accept_waveform(pending + [0.0] * (window - len(pending)))
        vad.flush()
        drain()


def recognize_zipformer(recognizer, source):
    stream = recognizer.create_stream()
    start = None
    end = 0.0
    previous = ""
    for timestamp, samples in source:
        if start is not None and abs(timestamp - end) > 0.002:
            stream.accept_waveform(16000, [0.0] * 8000)
            stream.input_finished()
            while recognizer.is_ready(stream):
                recognizer.decode_stream(stream)
            text = " ".join(recognizer.get_result(stream).split())
            if text:
                emit(start=start, end=end, text=text, final=True)
            stream = recognizer.create_stream()
            start = None
            previous = ""
        if start is None:
            start = timestamp
        end = timestamp + len(samples) / 16000
        stream.accept_waveform(16000, samples)
        while recognizer.is_ready(stream):
            recognizer.decode_stream(stream)
        text = " ".join(recognizer.get_result(stream).split())
        final = recognizer.is_endpoint(stream)
        if text and (text != previous or final):
            emit(start=start, end=end, text=text, final=final)
        previous = text
        if final:
            recognizer.reset(stream)
            start = None
            previous = ""
    if start is not None:
        stream.accept_waveform(16000, [0.0] * 8000)
        stream.input_finished()
        while recognizer.is_ready(stream):
            recognizer.decode_stream(stream)
        text = " ".join(recognizer.get_result(stream).split())
        if text:
            emit(start=start, end=end, text=text, final=True)


def main():
    helper_source, directory, engine, language, action = sys.argv[1:]
    root = Path(directory)
    sys.path.insert(0, str(root))
    helpers = {"__name__": "subtitle_helpers"}
    exec(compile(helper_source, "auto_subtitle.py", "exec"), helpers)
    helpers["component"] = directory
    prepared = prepare(helpers, root, engine, language)
    if action == "prepare":
        return
    emit(ready=True)
    source = packets(sys.stdin.buffer)
    if engine == "sensevoice":
        recognize_sensevoice(prepared, source)
    else:
        recognize_zipformer(prepared, source)
    emit(done=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        emit(error=str(error))
        sys.exit(1)
