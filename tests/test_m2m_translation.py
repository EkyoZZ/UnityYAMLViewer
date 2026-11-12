"""Language routing and lifetime entrypoint tests without downloading model weights."""
import importlib.util
from pathlib import Path
import sys
import unittest
from types import SimpleNamespace
from unittest.mock import patch

spec=importlib.util.spec_from_file_location("m2m",Path(__file__).parents[1]/"src/m2m_translation.py")
m2m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m2m)

class ModelTests(unittest.TestCase):
    def setUp(self):
        self.calls=[]
        self.output=["__zh__","译文"]
        owner=self
        class Translator:
            def __init__(self,*args,**kwargs):
                owner.settings=kwargs
            def translate_batch(self,tokens,**kwargs):
                owner.calls.append((tokens,kwargs))
                return [SimpleNamespace(hypotheses=[owner.output])]
        class SentencePiece:
            def __init__(self,**kwargs):pass
            def encode(self,text,**kwargs):
                owner.clean=text
                return text.split()
            def decode(self,tokens):return " ".join(tokens)
        self.modules=patch.dict(sys.modules,{"ctranslate2":SimpleNamespace(Translator=Translator,get_cuda_device_count=lambda:0),"sentencepiece":SimpleNamespace(SentencePieceProcessor=SentencePiece)})
        self.modules.start()
        self.addCleanup(self.modules.stop)
        self.srt="1\n00:00:02,000 --> 00:00:03,500\n<i>日本語</i> 内容\n\n"
    def test_japanese_chinese_direct_route_and_bilingual_timing(self):
        single,dual=m2m.translate(Path("unused"),"ja","zh",True,self.srt)
        self.assertEqual(self.calls[0][0],[["__ja__","日本語","内容"]])
        self.assertEqual(self.calls[0][1]["target_prefix"],[["__zh__"]])
        self.assertEqual(self.settings["compute_type"],"int8")
        self.assertEqual(self.settings["device"],"cpu")
        self.assertIn("00:00:02,000 --> 00:00:03,500\n译文",single)
        self.assertIn("<i>日本語</i> 内容\n译文",dual)
    def test_all_distinct_supported_pairs(self):
        for source in m2m.LANGUAGES:
            for target in m2m.LANGUAGES-{source}:
                self.output=[f"__{target}__","translation"]
                single,dual=m2m.translate(Path("unused"),source,target,False,self.srt)
                self.assertTrue(single)
                self.assertEqual(dual,"")
    def test_cantonese_and_identical_languages_fail(self):
        for source,target in [("yue","zh"),("zh","yue"),("ja","ja")]:
            with self.assertRaisesRegex(ValueError,"language pair"):
                m2m.translate(Path("unused"),source,target,False,self.srt)
    def test_wrong_target_prefix_is_rejected(self):
        self.output=["__en__","wrong"]
        with self.assertRaisesRegex(ValueError,"prefix"):
            m2m.translate(Path("unused"),"ja","zh",False,self.srt)
    def test_long_cues_are_not_silently_truncated(self):
        text="1\n00:00:00,000 --> 00:00:02,000\n"+"word "*513
        with self.assertRaisesRegex(ValueError,"too long"):
            m2m.translate(Path("unused"),"ja","zh",False,text)
        self.assertEqual(self.calls,[])
    def test_live_translation_reuses_loaded_engine_and_returns_paired_sources(self):
        import io, json
        source = io.StringIO(json.dumps({"text":"first"})+"\n"+json.dumps({"text":"second"})+"\n")
        output = io.StringIO()
        m2m.live_translate(Path("unused"), "en", "zh", source, output)
        results = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([item["source"] for item in results], ["first", "second"])
        self.assertTrue(all(item["text"] == "译文" for item in results))
        self.assertEqual(len(self.calls), 2)

    def test_gpu_must_pass_inference_before_it_is_selected(self):
        options={"prefer_gpu":True,"cpu_threads":2,"dll_handles":[]}
        cuda=unittest.mock.Mock()
        cuda.get_cuda_device_count.return_value=1
        cuda.get_supported_compute_types.return_value={"int8_float16"}
        gpu=unittest.mock.Mock();gpu.translate_batch.side_effect=RuntimeError("missing cublas")
        cpu=unittest.mock.Mock();cuda.Translator.side_effect=[gpu,cpu]
        with patch.object(m2m,"ai_options",return_value=options),patch.dict(sys.modules,ctranslate2=cuda):
            loaded,backend=m2m.live_engine(Path("unused"))
        self.assertIs(loaded[1],cpu)
        self.assertIn("GPU fallback",backend)
        self.assertEqual(cuda.Translator.call_args.kwargs["intra_threads"],2)
        self.assertEqual(cuda.Translator.call_args.kwargs["inter_threads"],1)

    def test_gpu_preference_off_does_not_probe_cuda(self):
        options={"prefer_gpu":False,"cpu_threads":1,"dll_handles":[]}
        cuda=unittest.mock.Mock()
        with patch.object(m2m,"ai_options",return_value=options),patch.dict(sys.modules,ctranslate2=cuda):
            _,backend=m2m.live_engine(Path("unused"))
        cuda.get_cuda_device_count.assert_not_called()
        self.assertIn("CPU",backend)
        self.assertEqual(cuda.Translator.call_args.kwargs["intra_threads"],1)

    def test_pinned_download_size_and_checksums(self):
        self.assertEqual(sum(size for _,size,_ in m2m.FILES),494252337)
        self.assertTrue(all(len(digest)==64 for _,_,digest in m2m.FILES))
        self.assertIn(m2m.REVISION,m2m.BASE)

if __name__=="__main__":unittest.main()
