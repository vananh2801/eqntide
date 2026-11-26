#!/usr/bin/env python3
"""Pre-rasterize selected Tabler SVG icons into committed PNG assets.

Usage:
  python3 scripts/render_icons.py          # Regenerate PNGs + manifest.json
  python3 scripts/render_icons.py --check  # Verify SVGs, PNGs, dimensions, and hashes

Supported SVG renderers (checked in order when regenerating):
  resvg, rsvg-convert, cairosvg (Python), inkscape, magick
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SVG_DIR = ROOT / "assets/icons/sources/tabler"
DESKTOP_OUT_DIR = ROOT / "assets/icons/generated/desktop"
RIBBON_OUT_DIR = ROOT / "assets/icons/generated/office-ribbon"
WPS_RIBBON_OUT_DIR = ROOT / "wps-addin/assets/icons"
MANIFEST_PATH = DESKTOP_OUT_DIR / "manifest.json"

DESKTOP_SIZE = 48
DESKTOP_COLOR = "#FFFFFF"
RIBBON_SIZE = 32
RIBBON_COLOR = "#1D78B8"

DESKTOP_ICONS = (
    # Sidebar 14 category tiles
    "history",
    "math-x-divide-y",
    "square-root-2",
    "x-power-y",
    "parentheses",
    "math-integral",
    "sum",
    "matrix",
    "overline",
    "underline",
    "math-sin",
    "math-equal-lower",
    "arrows-left-right",
    "arrow-autofit-right",
    "math-avg",
    "alpha",
    "omega",
    # Palette custom matrix button
    "dots",
    # Sidebar tabs
    "file-type-docx",
    "settings",
    "info-circle",
    # Footer history controls
    "arrow-back-up",
    "arrow-forward-up",
    # Secondary views & Batch DOCX
    "chevron-left",
    "file",
    "folder",
    "folder-open",
    "file-upload",
    # About metadata
    "user",
    "license",
    "brand-github",
)

RIBBON_ICONS = (
    "pencil-plus",
    "edit",
    "tex",
    "arrow-big-up-lines",
    "file-type-docx",
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png_dimensions_rgba(path: Path) -> tuple[int, int, int]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a valid PNG")
    length, chunk_type = struct.unpack(">I4s", data[8:16])
    if chunk_type != b"IHDR" or length != 13:
        raise ValueError(f"{path} missing IHDR chunk")
    width, height, _bit_depth, color_type, *_ = struct.unpack(">IIBBBBB", data[16:29])
    return width, height, color_type


def detect_renderer() -> str:
    for cmd in ("resvg", "rsvg-convert"):
        if shutil.which(cmd):
            return cmd
    try:
        import cairosvg  # noqa: F401
        return "cairosvg"
    except ImportError:
        pass
    for cmd in ("inkscape", "magick"):
        if shutil.which(cmd):
            return cmd
    raise RuntimeError(
        "No SVG-to-PNG renderer found. Install resvg, librsvg (rsvg-convert), "
        "cairosvg, inkscape, or ImageMagick."
    )


def render_svg_to_png(renderer: str, svg_path: Path, png_path: Path, size: int, color: str, tmp_dir: Path) -> None:
    svg_text = svg_path.read_text(encoding="utf-8").replace("currentColor", color)
    colored_svg = tmp_dir / f"{svg_path.stem}-{size}.svg"
    colored_svg.write_text(svg_text, encoding="utf-8")
    png_path.parent.mkdir(parents=True, exist_ok=True)

    if renderer == "resvg":
        subprocess.run(
            ["resvg", "-w", str(size), "-h", str(size), str(colored_svg), str(png_path)],
            check=True,
            capture_output=True,
        )
    elif renderer == "rsvg-convert":
        subprocess.run(
            ["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(png_path), str(colored_svg)],
            check=True,
            capture_output=True,
        )
    elif renderer == "cairosvg":
        import cairosvg
        cairosvg.svg2png(bytestring=svg_text.encode("utf-8"), write_to=str(png_path), output_width=size, output_height=size)
    elif renderer == "inkscape":
        subprocess.run(
            [
                "inkscape",
                str(colored_svg),
                "--export-type=png",
                f"--export-filename={png_path}",
                "-w",
                str(size),
                "-h",
                str(size),
            ],
            check=True,
            capture_output=True,
        )
    elif renderer == "magick":
        subprocess.run(
            ["magick", "-background", "none", str(colored_svg), "-resize", f"{size}x{size}", str(png_path)],
            check=True,
            capture_output=True,
        )
    else:
        raise ValueError(f"Unsupported renderer: {renderer}")

    w, h, color_type = png_dimensions_rgba(png_path)
    if (w, h, color_type) != (size, size, 6):
        raise RuntimeError(f"Rendered {png_path} as {w}x{h} color_type={color_type}, expected {size}x{size} RGBA (6)")


def expected_specs() -> list[tuple[str, str, Path, int, str]]:
    specs: list[tuple[str, str, Path, int, str]] = []
    for name in DESKTOP_ICONS:
        specs.append(("desktop", name, DESKTOP_OUT_DIR / f"{name}.png", DESKTOP_SIZE, DESKTOP_COLOR))
    for name in RIBBON_ICONS:
        specs.append(("ribbon", name, RIBBON_OUT_DIR / f"{name}-32.png", RIBBON_SIZE, RIBBON_COLOR))
    return specs


def generate_all() -> None:
    renderer = detect_renderer()
    manifest: dict[str, dict[str, dict[str, object]]] = {"desktop": {}, "ribbon": {}}
    WPS_RIBBON_OUT_DIR.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="eqntide-icons-") as tmp:
        tmp_dir = Path(tmp)
        for group, name, out_path, size, color in expected_specs():
            svg_path = SVG_DIR / f"{name}.svg"
            if not svg_path.is_file():
                raise FileNotFoundError(f"Missing source SVG: {svg_path}")
            render_svg_to_png(renderer, svg_path, out_path, size, color, tmp_dir)
            entry: dict[str, object] = {
                "path": out_path.relative_to(ROOT).as_posix(),
                "size": size,
                "color": color,
                "svg_sha256": sha256_file(svg_path),
                "png_sha256": sha256_file(out_path),
            }
            if group == "ribbon":
                mirror_path = WPS_RIBBON_OUT_DIR / out_path.name
                shutil.copy2(out_path, mirror_path)
                entry["mirror_path"] = mirror_path.relative_to(ROOT).as_posix()
            manifest[group][name] = entry
    MANIFEST_PATH.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Rendered {len(DESKTOP_ICONS)} desktop icons and {len(RIBBON_ICONS)} ribbon icons using {renderer}.")


def check_all() -> int:
    errors: list[str] = []
    if not MANIFEST_PATH.is_file():
        print(f"ERROR: Missing manifest {MANIFEST_PATH}", file=sys.stderr)
        return 1
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    for group, name, out_path, size, color in expected_specs():
        svg_path = SVG_DIR / f"{name}.svg"
        if not svg_path.is_file():
            errors.append(f"Missing source SVG: {svg_path}")
            continue
        if not out_path.is_file():
            errors.append(f"Missing rendered PNG: {out_path}")
            continue
        w, h, color_type = png_dimensions_rgba(out_path)
        if (w, h, color_type) != (size, size, 6):
            errors.append(f"{out_path}: expected {size}x{size} RGBA (6), got {w}x{h} ({color_type})")
        entry = manifest.get(group, {}).get(name)
        if not entry:
            errors.append(f"{MANIFEST_PATH}: missing manifest entry for {group}/{name}")
            continue
        expected_rel = out_path.relative_to(ROOT).as_posix()
        if entry.get("path") != expected_rel:
            errors.append(f"{group}/{name}: manifest path mismatch ({entry.get('path')!r} != {expected_rel!r})")
        if entry.get("size") != size or entry.get("color") != color:
            errors.append(f"{group}/{name}: manifest metadata mismatch ({entry})")
        if entry.get("svg_sha256") != sha256_file(svg_path):
            errors.append(f"{group}/{name}: SVG changed since last render; re-run scripts/render_icons.py")
        expected_png_sha = sha256_file(out_path)
        if entry.get("png_sha256") != expected_png_sha:
            errors.append(f"{group}/{name}: PNG checksum mismatch; re-run scripts/render_icons.py")
        if group == "ribbon":
            mirror_path = WPS_RIBBON_OUT_DIR / out_path.name
            expected_mirror_rel = mirror_path.relative_to(ROOT).as_posix()
            if entry.get("mirror_path") != expected_mirror_rel:
                errors.append(
                    f"{group}/{name}: manifest mirror_path mismatch "
                    f"({entry.get('mirror_path')!r} != {expected_mirror_rel!r})"
                )
            if not mirror_path.is_file():
                errors.append(f"Missing WPS ribbon mirror PNG: {mirror_path}")
            elif sha256_file(mirror_path) != expected_png_sha:
                errors.append(f"{group}/{name}: WPS ribbon mirror checksum mismatch ({mirror_path}); re-run scripts/render_icons.py")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1
    print(f"Verified {len(DESKTOP_ICONS)} desktop icons and {len(RIBBON_ICONS)} ribbon icons.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Verify committed PNGs match source SVGs and manifest")
    args = parser.parse_args()
    if args.check:
        return check_all()
    generate_all()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
