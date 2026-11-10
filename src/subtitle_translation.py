import hashlib
import json
import re
import shutil
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

MODELS = {
    "zh-to-en": ("zho-eng", 288654138, "9749101f963f552a270ff4de0c8b69c88f51c358d6b4cc87760aff2cffcf4067"),
    "en-to-zh": ("eng-zho", 288947487, "ab90e5583fa3ec46d8c5c50f106cbd65221978cd052c7e2059981439325ae24b"),
}


def install(root, direction):
    import ctranslate2

    pair, size, expected = MODELS[direction]
    url = f"https://object.pouta.csc.fi/Tatoeba-MT-models/{pair}/opus-2020-07-17.zip"
    destination = root / direction
    if destination.is_dir():
        if all((destination / name).is_file() for name in (
            "model.bin", "config.json", "shared_vocabulary.json", "source.spm", "target.spm"
        )):
            return
        raise ValueError(f"Incomplete installed model: {destination}")
    with tempfile.TemporaryDirectory(dir=root) as temp:
        temp = Path(temp)
        archive = temp / "model.zip"
        digest = hashlib.sha256()
        downloaded = 0
        print(f"Progress 0 of {size}", flush=True)
        with urllib.request.urlopen(url, timeout=60) as response, archive.open("wb") as file:
            while chunk := response.read(1024 * 1024):
                downloaded += len(chunk)
                if downloaded > size:
                    raise ValueError("OPUS-MT archive exceeds the expected size")
                digest.update(chunk)
                file.write(chunk)
                print(f"Progress {downloaded} of {size}", flush=True)
        if downloaded != size or digest.hexdigest() != expected:
            raise ValueError("OPUS-MT archive failed SHA-256 verification")
        print("Converting OPUS-MT to INT8", flush=True)
        original = temp / "original"
        original.mkdir()
        with zipfile.ZipFile(archive) as file:
            for entry in file.infolist():
                if Path(entry.filename).name == entry.filename and entry.filename.endswith(
                    (".npz", ".yml", ".spm", "LICENSE", "README.md")
                ):
                    file.extract(entry, original)
        converted = temp / "converted"
        ctranslate2.converters.OpusMTConverter(str(original)).convert(str(converted), quantization="int8")
        for name in ("source.spm", "target.spm", "LICENSE", "README.md"):
            shutil.copyfile(original / name, converted / name)
        ctranslate2.Translator(str(converted), device="cpu", compute_type="int8")
        converted.rename(destination)


def translate(root, direction, bilingual, text):
    import ctranslate2
    import sentencepiece

    model = root / direction
    source = sentencepiece.SentencePieceProcessor(model_file=str(model / "source.spm"))
    target = sentencepiece.SentencePieceProcessor(model_file=str(model / "target.spm"))
    translator = ctranslate2.Translator(str(model), device="cpu", compute_type="int8", intra_threads=4)
    tag = [">>cmn_Hans<<"] if direction == "en-to-zh" else []
    cues = []
    for block in re.split(r"\n[ \t]*\n+", text.replace("\r\n", "\n").replace("\r", "\n").strip()):
        index, timing, body = block.split("\n", 2)
        if not index.strip().isdigit() or "-->" not in timing or not body.strip():
            raise ValueError("Invalid SRT cue")
        cues.append((index, timing, body))
    if not cues:
        raise ValueError("No SRT cues")
    single, dual = [], []
    for start in range(0, len(cues), 32):
        group = cues[start:start + 32]
        originals = [" ".join(body.split()) for _, _, body in group]
        # Keep formatting tags out of the translation model.
        tokens = [tag + source.encode(re.sub(r"</?(?:i|b|u|font)\b[^>]*>", "", line,
                                             flags=re.IGNORECASE), out_type=str) for line in originals]
        results = translator.translate_batch(tokens, max_batch_size=16, beam_size=2)
        for (index, timing, _), original, result in zip(group, originals, results, strict=True):
            translated = " ".join(target.decode(result.hypotheses[0]).split())
            if not translated:
                raise ValueError(f"Empty translation for subtitle {index}")
            single.append(f"{index}\n{timing}\n{translated}\n\n")
            if bilingual:
                chinese, english = (translated, original) if tag else (original, translated)
                dual.append(f"{index}\n{timing}\n{chinese}\n{english}\n\n")
    return ["".join(single), "".join(dual)]


if __name__ == "__main__":
    action, directory, direction = sys.argv[1:4]
    root = Path(directory)
    sys.path.insert(0, str(root / "runtime"))
    if action == "install":
        install(root, direction)
    else:
        output = translate(root, direction, sys.argv[4] == "1", sys.stdin.buffer.read().decode("utf-8-sig"))
        print(json.dumps(output))
