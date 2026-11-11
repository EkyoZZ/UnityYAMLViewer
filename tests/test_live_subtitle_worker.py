"""Protocol and recognizer lifecycle tests without model downloads or an EXE build."""
import importlib.util
import io
import struct
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("live", Path(__file__).parents[1] / "src/live_subtitle.py")
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)


class SplitReads(io.BytesIO):
    def read(self, count=-1):
        return super().read(min(count, 3))


class LiveWorkerTests(unittest.TestCase):
    def test_sensevoice_gap_resets_media_origin_after_cache_skip(self):
        class Vad:
            def __init__(self):
                self.queue = []
                self.resets = 0
            def accept_waveform(self, samples):
                self.samples = samples
            def flush(self):
                self.queue.append(SimpleNamespace(samples=self.samples, start=0))
            def reset(self):
                self.resets += 1
            def empty(self):
                return not self.queue
            @property
            def front(self):
                return self.queue[0]
            def pop(self):
                self.queue.pop(0)
        recognizer = SimpleNamespace(create_stream=lambda: SimpleNamespace(
            accept_waveform=lambda *args: None, result=SimpleNamespace(text="text")),
            decode_stream=lambda stream: None)
        vad = Vad()
        with patch.object(live, "emit") as emit:
            live.recognize_sensevoice((recognizer, vad, 512), [(10.0, [0.0]*320), (25.0, [0.0]*320)])
        self.assertEqual(vad.resets, 1)
        self.assertEqual([c.kwargs["start"] for c in emit.call_args_list], [10.0, 25.0])

    def test_zipformer_gap_flushes_without_joining_cached_audio(self):
        class Stream:
            def accept_waveform(self, *args):
                pass
            def input_finished(self):
                pass
        recognizer = SimpleNamespace(create_stream=lambda: Stream(), is_ready=lambda stream: False,
            get_result=lambda stream: "text", is_endpoint=lambda stream: False)
        with patch.object(live, "emit") as emit:
            live.recognize_zipformer(recognizer, [(10.0, [0.0]*1600), (25.0, [0.0]*1600)])
        finals = [c.kwargs for c in emit.call_args_list if c.kwargs["final"]]
        self.assertEqual([c["start"] for c in finals], [10.0, 25.0])
        self.assertAlmostEqual(finals[0]["end"], 10.1)

    def test_zipformer_preparation_uses_pinned_files_in_separate_directory(self):
        calls = []
        helpers = {}
        def model_file(name, url, limit, digest):
            directory = Path(helpers["component"]) / "models"
            calls.append((name, url, limit, digest))
            return str(directory / name)
        helpers["model_file"] = model_file
        fake = SimpleNamespace(OnlineRecognizer=SimpleNamespace(from_transducer=lambda **kwargs: kwargs))
        with patch.object(Path, "mkdir"), patch.dict(sys.modules, {"sherpa_onnx": fake}):
            root = Path(__file__).resolve().parent / "model-fixture"
            result = live.prepare(helpers, root, "zipformer", "zh")
            self.assertEqual(Path(result["encoder"]).parent, root / "zipformer-zh-en/models")
        self.assertEqual(len(calls), 4)
        self.assertTrue(all(live.ZIP_REVISION in call[1] for call in calls))
        self.assertTrue(all(len(call[3]) == 64 for call in calls[:3]))
        self.assertTrue(result["enable_endpoint_detection"])

    def test_pipe_boundaries_and_eof_marker(self):
        data = struct.pack("<dI2f", 32.5, 2, 0.25, -0.5) + struct.pack("<dI", 32.6, 0)
        self.assertEqual(list(live.packets(SplitReads(data))), [(32.5, (0.25, -0.5))])

    def test_truncated_or_oversized_packets_fail(self):
        for data in (b"\x00", struct.pack("<dI", 0.0, 16001), struct.pack("<dI", float("nan"), 1)):
            with self.subTest(data=data), self.assertRaises((EOFError, ValueError)):
                list(live.packets(io.BytesIO(data)))

    def test_sensevoice_flushes_tail_with_media_timestamps(self):
        class Vad:
            queue = []
            def accept_waveform(self, samples):
                self.samples = samples
            def flush(self):
                self.queue.append(SimpleNamespace(samples=self.samples, start=0))
            def empty(self):
                return not self.queue
            @property
            def front(self):
                return self.queue[0]
            def pop(self):
                self.queue.pop(0)
        class Recognizer:
            def create_stream(self):
                return SimpleNamespace(accept_waveform=lambda *args: None, result=SimpleNamespace(text=" 日语 内容 "))
            def decode_stream(self, stream):
                pass
        with patch.object(live, "emit") as emit:
            live.recognize_sensevoice((Recognizer(), Vad(), 512), [(45.0, [0.1] * 320)])
        event = emit.call_args.kwargs
        self.assertEqual(event["start"], 45.0)
        self.assertTrue(event["final"])
        self.assertEqual(event["text"], "日语 内容")

    def test_zipformer_replaces_partial_and_flushes_final(self):
        class Stream:
            def accept_waveform(self, *args):
                pass
            def input_finished(self):
                pass
        class Recognizer:
            text = "hello"
            def create_stream(self):
                return Stream()
            def is_ready(self, stream):
                return False
            def get_result(self, stream):
                return self.text
            def is_endpoint(self, stream):
                return False
        with patch.object(live, "emit") as emit:
            live.recognize_zipformer(Recognizer(), [(10.0, [0.0] * 1600), (10.1, [0.0] * 1600)])
        self.assertEqual(emit.call_count, 2)  # one partial, one final; unchanged partial suppressed
        self.assertFalse(emit.call_args_list[0].kwargs["final"])
        self.assertTrue(emit.call_args_list[1].kwargs["final"])
        self.assertAlmostEqual(emit.call_args.kwargs["end"], 10.2)

    def test_zipformer_endpoint_starts_a_new_utterance(self):
        class Recognizer:
            resets = 0
            def create_stream(self):
                return SimpleNamespace(accept_waveform=lambda *args: None)
            def is_ready(self, stream):
                return False
            def get_result(self, stream):
                return "sentence"
            def is_endpoint(self, stream):
                return True
            def reset(self, stream):
                self.resets += 1
        recognizer = Recognizer()
        with patch.object(live, "emit") as emit:
            live.recognize_zipformer(recognizer, [(2.0, [0.0] * 1600), (3.0, [0.0] * 1600)])
        self.assertEqual(recognizer.resets, 2)
        self.assertEqual([call.kwargs["start"] for call in emit.call_args_list], [2.0, 3.0])


if __name__ == "__main__":
    unittest.main()
