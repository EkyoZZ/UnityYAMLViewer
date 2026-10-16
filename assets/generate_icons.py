#!/usr/bin/env python3
"""Generate application PNG icons with two-stage bilinear downsampling.

Usage: python assets/generate_icons.py
Dependency: python -m pip install Pillow
Sources are resolved beside this script, regardless of the working directory.
Existing small icons are replaced; the 1024/512 source images are never changed.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import sys

SIZES = (16, 20, 24, 32, 40, 48, 64, 72, 80, 96, 128, 192, 256)


def generate(source_dir: Path, output_dir: Path) -> list[Path]:
    source = next((source_dir / f"app_icon_{size}.png"
                   for size in (1024, 512)
                   if (source_dir / f"app_icon_{size}.png").is_file()), None)
    if source is None:
        raise FileNotFoundError(
            f"Missing source: {source_dir / 'app_icon_1024.png'} or app_icon_512.png")
    try:
        from PIL import Image
    except ImportError as exc:
        raise RuntimeError("Pillow is required: python -m pip install Pillow") from exc

    expected = 1024 if source.name == "app_icon_1024.png" else 512
    with Image.open(source) as original:
        if original.size != (expected, expected):
            raise ValueError(f"{source.name}: expected {expected}x{expected}, got {original.size}")
        # Premultiplied alpha prevents transparent edge RGB from tinting the icon.
        image = original.convert("RGBA").convert("RGBa")
    output_dir.mkdir(parents=True, exist_ok=True)
    result = []
    for size in SIZES:
        # Always derive each size from the high-resolution source, never from
        # another exported icon. Both passes explicitly use bilinear filtering.
        middle = image.resize((size * 2, size * 2), Image.Resampling.BILINEAR)
        icon = middle.resize((size, size), Image.Resampling.BILINEAR).convert("RGBA")
        target = output_dir / f"app_icon_{size}.png"
        icon.save(target, format="PNG")
        result.append(target)
    print(f"Source: {source.name}; generated {len(result)} icons in {output_dir}")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path,
                        help="Output directory (default: beside this script)")
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    try:
        generate(directory, args.output_dir or directory)
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
