"""Pinned community Fairseq -> CTranslate2 INT8 model. No Torch or remote code."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sys
import tempfile

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


REVISION = "1aeed44db4dd61a486bba44acf54c76507082a2c"
BASE = "https://huggingface.co/JustFrederik/m2m_100_418m_ct2_int8/resolve/" + REVISION + "/"
FILES = (
    ("model.bin", 490667638, "6c8ef4e814a02d5947ab80710f93f3d4e776bf4b668ab3bb7996ff050b69d111"),
    ("config.json", 158, "531f7f8d8bf8b73a1580b73c04b58bf134ac490284f4f7af6c20ad238499ae37"),
    ("shared_vocabulary.txt", 1159930, "bd440aa21b8ca3453fc792a0018a1f3fe68b3464aadddd4d16a4b72f73c86d8c"),
    ("spm.128k.model", 2423393, "d8f7c76ed2a5e0822be39f0a4f95a55eb19c78f4593ce609e2edbc2aea4d380a"),
    ("README.md", 1218, "f40ae40395c600985618b5dd658e61762dd236a95b89cc5fcddb07e021053ec2"),
)
LANGUAGES = {"zh", "en", "ja", "ko"}


def valid_model(directory):
    for name, size, expected in FILES:
        path = directory / name
        if not path.is_file() or path.stat().st_size != size:
            return False
        digest = hashlib.sha256()
        with path.open("rb") as file:
            while block := file.read(1024 * 1024):
                digest.update(block)
        if digest.hexdigest() != expected:
            return False
    return True


def install(root, download):
    import ctranslate2
    import sentencepiece

    destination = root / "model"
    if destination.exists():
        if not valid_model(destination):
            raise ValueError("M2M100 installed model failed verification; move the damaged model directory before reinstalling")
        ctranslate2.Translator(str(destination), device="cpu", compute_type="int8")
        sentencepiece.SentencePieceProcessor(model_file=str(destination / "spm.128k.model"))
        print("M2M100 model verification complete", flush=True)
        return
    # Retained download cache + separate atomic publication: about 1 GB plus runtime.
    if shutil.disk_usage(root).free < 1100 * 1024 * 1024:
        raise OSError("M2M100 requires at least 1.1 GiB free disk space for download and staging; 2 GiB recommended")
    with tempfile.TemporaryDirectory(dir=root) as temporary:
        staged = Path(temporary) / "model"
        staged.mkdir()
        for name, size, digest in FILES:
            path = download(root, name, BASE + name, size, digest, label="M2M100", suffix="")
            shutil.copyfile(path, staged / name)
        print("UNITY_INSTALL_STAGE 5", flush=True)
        print("M2M100 checking INT8 model loading", flush=True)
        ctranslate2.Translator(str(staged), device="cpu", compute_type="int8")
        sentencepiece.SentencePieceProcessor(model_file=str(staged / "spm.128k.model"))
        staged.rename(destination)
    print("M2M100 installation complete", flush=True)


def translate(root, source_language, target_language, bilingual, text, engine=None):
    import ctranslate2
    import sentencepiece

    if source_language not in LANGUAGES or target_language not in LANGUAGES or source_language == target_language:
        raise ValueError("Unsupported M2M100 language pair")
    model = root / "model"
    if engine is None:
        loaded, _backend = live_engine(root)
        sp, translator = loaded[:2]
    else:
        sp, translator = engine
    single, dual = [], []
    for block in re.split(r"\n[ \t]*\n+", text.replace("\r\n", "\n").replace("\r", "\n").strip()):
        index, timing, body = block.split("\n", 2)
        if not index.strip().isdigit() or "-->" not in timing or not body.strip():
            raise ValueError("Invalid SRT cue")
        original = " ".join(body.split())
        clean = re.sub(r"</?(?:i|b|u|font)\b[^>]*>", "", original, flags=re.IGNORECASE)
        pieces = sp.encode(clean, out_type=str)
        if len(pieces) > 512:
            raise ValueError(f"Subtitle {index} is too long; split it before translation")
        # Fairseq model supplies source EOS itself; language prefixes match its vocabulary.
        results = translator.translate_batch([[f"__{source_language}__"] + pieces],
            target_prefix=[[f"__{target_language}__"]], beam_size=2, max_decoding_length=512)
        hypothesis = results[0].hypotheses[0]
        if not hypothesis or hypothesis[0] != f"__{target_language}__":
            raise ValueError("M2M100 target language prefix missing")
        translated = " ".join(sp.decode(hypothesis[1:]).split())
        if not translated:
            raise ValueError(f"Empty translation for subtitle {index}")
        single.append(f"{index}\n{timing}\n{translated}\n\n")
        if bilingual:
            dual.append(f"{index}\n{timing}\n{original}\n{translated}\n\n")
    if not single:
        raise ValueError("No SRT cues")
    # This dedicated process exits after output; model memory is not retained by the GUI.
    return ["".join(single), "".join(dual)]


def live_engine(root, cpu_threads=2, force_cpu=False):
    """Prefer validated CUDA inference; fall back without saturating the CPU."""
    import ctranslate2
    import sentencepiece
    model = root / "model"
    sp = sentencepiece.SentencePieceProcessor(model_file=str(model / "spm.128k.model"))
    options = ai_options(root)
    threads = options["cpu_threads"]
    gpu_error = ""
    # NVIDIA pip runtime packages keep DLLs outside PATH on Windows.
    dll_handles = options["dll_handles"]
    if os.name == "nt" and hasattr(os, "add_dll_directory"):
        for directory in (root / "runtime/nvidia/cublas/bin", root / "runtime/nvidia/cudnn/bin"):
            if directory.is_dir():
                dll_handles.append(os.add_dll_directory(str(directory)))
    try:
        if options["prefer_gpu"] and not force_cpu and ctranslate2.get_cuda_device_count() > 0:
            types = ctranslate2.get_supported_compute_types("cuda")
            compute = "int8_float16" if "int8_float16" in types else "int8_float32"
            translator = ctranslate2.Translator(str(model), device="cuda", compute_type=compute, inter_threads=1, intra_threads=1)
            # Construction alone may succeed even when cuBLAS is missing.
            translator.translate_batch([["__en__"] + sp.encode("Hello", out_type=str)], target_prefix=[["__zh__"]], beam_size=1, max_decoding_length=16)
            return (sp, translator, dll_handles), "GPU · CUDA " + compute
    except (RuntimeError, ValueError) as error:
        gpu_error = str(error)
    translator = ctranslate2.Translator(str(model), device="cpu", compute_type="int8", inter_threads=1, intra_threads=threads)
    backend = f"CPU · INT8 · {threads} threads"
    if gpu_error:
        backend += " · GPU fallback: " + gpu_error[:300]
    return (sp, translator, dll_handles), backend


def live_translate(root, source_language, target_language, input_stream, output_stream, cpu_threads=2):
    if source_language not in LANGUAGES or target_language not in LANGUAGES or source_language == target_language:
        raise ValueError("Unsupported M2M100 language pair")
    loaded, backend = live_engine(root, cpu_threads)
    engine = loaded[:2]
    for line in input_stream:
        request = json.loads(line)
        text = request["text"]
        try:
            if len(text) > 8192:
                raise ValueError("Live subtitle too long")
            srt = "1\n00:00:00,000 --> 00:00:01,000\n" + " ".join(text.split()) + "\n\n"
            try:
                translated = translate(root, source_language, target_language, False, srt, engine)[0]
            except RuntimeError as error:
                if not backend.startswith("GPU"):
                    raise
                loaded, backend = live_engine(root, force_cpu=True)
                engine = loaded[:2]
                backend += " · GPU fallback: " + str(error)[:300]
                translated = translate(root, source_language, target_language, False, srt, engine)[0]
            result = {"source": text, "text": translated.split("\n", 2)[2].strip(), "backend": backend}
        except (ValueError, RuntimeError) as error:
            result = {"source": text, "error": str(error)}
        output_stream.write(json.dumps(result) + "\n")
        output_stream.flush()


def save_peak(root):
    if os.name != "nt":
        return
    try:
        from ctypes import wintypes
        class Counters(ctypes.Structure):
            _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
                (name, ctypes.c_size_t) for name in ("PeakWorkingSetSize", "WorkingSetSize",
                "QuotaPeakPagedPoolUsage", "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
                "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage")]
        current = ctypes.windll.kernel32.GetCurrentProcess
        current.restype = wintypes.HANDLE
        measure = ctypes.windll.psapi.GetProcessMemoryInfo
        measure.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
        counters = Counters()
        counters.cb = ctypes.sizeof(counters)
        if measure(current(), ctypes.byref(counters), counters.cb):
            temporary = root / f"peak-{os.getpid()}.tmp"
            temporary.write_text(json.dumps({"peak_working_set_bytes": counters.PeakWorkingSetSize}), encoding="utf-8")
            temporary.replace(root / "last-peak.json")
    except (OSError, ValueError, AttributeError):
        pass  # Resource telemetry must not break translation output.


if __name__ == "__main__":
    helpers, action, directory = sys.argv[1:4]
    root = Path(directory)
    shared = root / "runtime"
    sys.path.insert(0, str(shared))
    namespace = {"__name__": "subtitle_helpers"}
    exec(compile(helpers, "translation_download.py", "exec"), namespace)
    completed = False
    try:
        if action == "install":
            install(root, namespace["verified_archive"])
        elif action == "live":
            live_translate(root, sys.argv[4], sys.argv[5], sys.stdin, sys.stdout, int(sys.argv[6]) if len(sys.argv)>6 else 2)
        else:
            output = translate(root, sys.argv[4], sys.argv[5], sys.argv[6] == "1", sys.stdin.buffer.read().decode("utf-8-sig"))
            print(json.dumps(output))
        completed = True
    finally:
        if completed:
            save_peak(root)
