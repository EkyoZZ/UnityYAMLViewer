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
    def test_generated_long_subtitles_split_without_losing_text(self):
        helper_spec=importlib.util.spec_from_file_location("auto",Path(__file__).parents[1]/"src/auto_subtitle.py")
        helper=importlib.util.module_from_spec(helper_spec)
        helper_spec.loader.exec_module(helper)
        text="这是很长的字幕，"*20
        parts=list(helper.timed_parts(text,10.0,22.0))
        self.assertGreater(len(parts),1)
        self.assertEqual("".join(p[0] for p in parts),text)
        self.assertEqual(parts[0][1],10.0)
        self.assertEqual(parts[-1][2],22.0)
        self.assertTrue(all(len(p[0])<=32 for p in parts))
        self.assertTrue(all(a[2]==b[1] for a,b in zip(parts,parts[1:])))

    def test_generated_cues_use_duration_and_sentence_boundaries(self):
        helper_spec = importlib.util.spec_from_file_location("auto", Path(__file__).parents[1] / "src/auto_subtitle.py")
        helper = importlib.util.module_from_spec(helper_spec)
        helper_spec.loader.exec_module(helper)
        text = "第一句话已经说得很完整了。第二句话接下来继续说明。"
        parts = list(helper.timed_parts(text, 4.0, 10.0))
        self.assertGreaterEqual(len(parts), 2)
        self.assertTrue(parts[0][0].endswith("。"))
        self.assertEqual("".join(part[0] for part in parts), text)
        self.assertEqual(parts[0][1], 4.0)
        self.assertEqual(parts[-1][2], 10.0)
        self.assertTrue(all(a[2] == b[1] for a, b in zip(parts, parts[1:])))
        cues = []
        helper.add_cue(cues, text, 4.0, 6.0)
        self.assertEqual(len(cues), len(parts))
        self.assertIn("00:00:04,000 -->", cues[0])
        self.assertIn("00:00:10,000", cues[-1])
        continuous = "持续说话没有明显停顿但也应主动拆开方便阅读理解内容"
        self.assertGreater(len(list(helper.timed_parts(continuous, 0, 18))), 1)
        self.assertEqual(list(helper.timed_parts("", 0, 1)), [])
        self.assertEqual(list(helper.timed_parts("文本", 2, 1)), [])

    def test_zipformer_uses_bounded_beam_search(self):
        import tempfile
        factory = unittest.mock.Mock(return_value=object())
        module = SimpleNamespace(OnlineRecognizer=SimpleNamespace(from_transducer=factory))
        with tempfile.TemporaryDirectory(dir=Path(__file__).parents[1] / "target") as directory, patch.dict(sys.modules, sherpa_onnx=module):
            helpers = {"model_file": lambda name, *args: name, "asr_recognizer": lambda factory, folder, **kwargs: factory(num_threads=2, provider="cpu", **kwargs)}
            live.prepare(helpers, Path(directory), "zipformer", "zh")
        config = factory.call_args.kwargs
        self.assertEqual(config["decoding_method"], "modified_beam_search")
        self.assertEqual(config["max_active_paths"], 4)
        self.assertEqual(config["rule2_min_trailing_silence"], 0.45)

    def test_recognition_runtime_and_model_directories_are_independent(self):
        import tempfile, os
        helper_spec=importlib.util.spec_from_file_location("auto_runtime",Path(__file__).parents[1]/"src/auto_subtitle.py")
        helper=importlib.util.module_from_spec(helper_spec);helper_spec.loader.exec_module(helper)
        with tempfile.TemporaryDirectory(dir=Path(__file__).parents[1]/"target") as directory:
            model_root=Path(directory)/"models_cpu_location"
            runtime=Path(directory)/"gpu_runtime"
            provider=runtime/"sherpa_onnx/lib/onnxruntime_providers_cuda.dll";provider.parent.mkdir(parents=True);provider.touch()
            factory=unittest.mock.Mock(return_value="recognizer")
            with patch.dict(os.environ,UNITY_ASR_RUNTIME=str(runtime)),patch.object(helper,"ai_options",return_value={"prefer_gpu":True,"cpu_threads":2,"dll_handles":[]}):
                result=helper.asr_recognizer(factory,model_root,model="shared-model.onnx")
            self.assertEqual(result,"recognizer")
            self.assertEqual(factory.call_args.kwargs["provider"],"cuda")
            self.assertEqual(factory.call_args.kwargs["model"],"shared-model.onnx")
            factory.reset_mock()
            with patch.dict(os.environ,UNITY_ASR_RUNTIME=str(runtime)),patch.object(helper,"ai_options",return_value={"prefer_gpu":False,"cpu_threads":1,"dll_handles":[]}):
                helper.asr_recognizer(factory,model_root,model="shared-model.onnx")
            self.assertEqual(factory.call_args.kwargs["provider"],"cpu")
            self.assertEqual(factory.call_args.kwargs["num_threads"],1)

    def test_sensevoice_forwards_detected_language_to_final_event(self):
        class Vad:
            def __init__(self):self.queue=[]
            def accept_waveform(self,samples):self.queue.append(SimpleNamespace(samples=samples,start=0))
            def empty(self):return not self.queue
            @property
            def front(self):return self.queue[0]
            def pop(self):self.queue.pop(0)
            def flush(self):pass
        result=SimpleNamespace(text="星のあしあと",lang="ja")
        stream=SimpleNamespace(result=result,accept_waveform=lambda *args:None)
        recognizer=SimpleNamespace(create_stream=lambda:stream,decode_stream=lambda stream:None)
        with patch.object(live,"emit") as emit:
            live.recognize_sensevoice((recognizer,Vad(),1600),[(0.0,[0.0]*1600)])
        self.assertEqual(emit.call_args.kwargs["language"],"ja")
        self.assertTrue(emit.call_args.kwargs["final"])

    def test_continuous_zipformer_speech_gets_a_length_endpoint(self):
        class Stream:
            def accept_waveform(self,*args):pass
            def input_finished(self):pass
        resets=[]
        recognizer=SimpleNamespace(create_stream=Stream,is_ready=lambda s:False,
            get_result=lambda s:"长"*49,is_endpoint=lambda s:False,reset=lambda s:resets.append(1))
        with patch.object(live,"emit") as emit:
            live.recognize_zipformer(recognizer,[(0.0,[0.0]*1600)])
        self.assertEqual(len(resets),1)
        self.assertTrue(emit.call_args.kwargs["final"])

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
        helpers = {"asr_recognizer": lambda factory, folder, **kwargs: factory(num_threads=2, provider="cpu", **kwargs)}
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
