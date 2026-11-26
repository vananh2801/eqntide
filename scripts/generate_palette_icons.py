#!/usr/bin/env python3
"""Generate Tabler-style 40x26 SVGs and a packed 2x RGBA sprite atlas for all 547 palette items.

Usage:
  python3 scripts/generate_palette_icons.py          # Generate SVGs + atlas.png + atlas.json
  python3 scripts/generate_palette_icons.py --check  # Verify catalog sync, SVGs, atlas PNG, and hashes
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import html
import json
import math
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
import palette_svg_builders as builders  # noqa: E402

ROOT = SCRIPT_DIR.parent
CATALOG_PATH = ROOT / "src/catalog/mod.rs"
FONT_DIR = ROOT / "assets/fonts"
SVG_OUT_DIR = ROOT / "assets/icons/sources/palette"
ATLAS_OUT_DIR = ROOT / "assets/icons/generated/palette-atlas"
ATLAS_PNG_PATH = ATLAS_OUT_DIR / "atlas.png"
ATLAS_JSON_PATH = ATLAS_OUT_DIR / "atlas.json"

ICON_W = 40
ICON_H = 26
PAD = 1
SCALE = 2
GRID_COLS = 24
COLOR = "#FFFFFF"


@dataclass(frozen=True)
class CatalogEntry:
    id: str
    group: str
    tid: str
    label: str
    source: str
    glyph: str | None


def unescape_rust_str(raw: str) -> str:
    def repl(m: re.Match[str]) -> str:
        esc = m.group(1)
        if esc == "\\":
            return "\\"
        if esc == '"':
            return '"'
        if esc == "'":
            return "'"
        if esc == "n":
            return "\n"
        if esc == "r":
            return "\r"
        if esc == "t":
            return "\t"
        if esc.startswith("u{") and esc.endswith("}"):
            return chr(int(esc[2:-1], 16))
        raise ValueError(f"Unsupported Rust escape: \\{esc}")

    return re.sub(r'\\(\\|"|\x27|n|r|t|u\{[0-9a-fA-F]+\})', repl, raw)


def parse_catalog() -> list[CatalogEntry]:
    text = CATALOG_PATH.read_text(encoding="utf-8")
    body = text.split("catalog! {", 1)[1]
    syms_block = body.split("symbols: [", 1)[1].split("parser_symbols: [", 1)[0]
    tmpls_block = body.split("templates: [", 1)[1].split("commands: [", 1)[0]

    entries: list[CatalogEntry] = []
    sym_re = re.compile(r'\("([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)"\)')
    tmpl_re = re.compile(r'\("([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)"\)')

    for group, name, glyph, label, _tag in sym_re.findall(syms_block):
        entries.append(
            CatalogEntry(
                id=f"symbol-{name}",
                group=group,
                tid=name,
                label=unescape_rust_str(label),
                source=f"\\{name}",
                glyph=unescape_rust_str(glyph),
            )
        )
    for tid, group, label, source in tmpl_re.findall(tmpls_block):
        entries.append(
            CatalogEntry(
                id=f"{group}-{tid}",
                group=group,
                tid=tid,
                label=unescape_rust_str(label),
                source=unescape_rust_str(source),
                glyph=None,
            )
        )
    return entries


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


# ---------------------------------------------------------------------------
# SVG Path Bounding Box Parser (for normalizing font-to-path output)
# ---------------------------------------------------------------------------

TOKEN_RE = re.compile(r"[MmLlHhVvCcQqSsTtZz]|[-+]?(?:\d*\.\d+|\d+)(?:[eE][-+]?\d+)?")


def cubic_point(p0: float, p1: float, p2: float, p3: float, t: float) -> float:
    mt = 1.0 - t
    return mt * mt * mt * p0 + 3 * mt * mt * t * p1 + 3 * mt * t * t * p2 + t * t * t * p3


def quad_point(p0: float, p1: float, p2: float, t: float) -> float:
    mt = 1.0 - t
    return mt * mt * p0 + 2 * mt * t * p1 + t * t * p2


def path_bbox(d: str) -> tuple[float, float, float, float]:
    tokens = TOKEN_RE.findall(d)
    idx = 0
    cmd = ""
    cx = cy = sx = sy = 0.0
    prev_cx = prev_cy = 0.0
    prev_cmd = ""
    min_x = min_y = float("inf")
    max_x = max_y = float("-inf")

    def add_pt(x: float, y: float) -> None:
        nonlocal min_x, min_y, max_x, max_y
        if x < min_x:
            min_x = x
        if y < min_y:
            min_y = y
        if x > max_x:
            max_x = x
        if y > max_y:
            max_y = y

    def next_num() -> float:
        nonlocal idx
        v = float(tokens[idx])
        idx += 1
        return v

    while idx < len(tokens):
        tok = tokens[idx]
        if tok.isalpha():
            cmd = tok
            idx += 1
            if cmd in ("Z", "z"):
                cx, cy = sx, sy
                prev_cmd = cmd
                continue
        rel = cmd.islower()
        op = cmd.upper()
        if op == "M":
            nx, ny = next_num(), next_num()
            if rel:
                nx += cx
                ny += cy
            cx, cy = nx, ny
            sx, sy = nx, ny
            add_pt(cx, cy)
            cmd = "l" if rel else "L"
            prev_cmd = op
        elif op == "L":
            nx, ny = next_num(), next_num()
            if rel:
                nx += cx
                ny += cy
            cx, cy = nx, ny
            add_pt(cx, cy)
            prev_cmd = op
        elif op == "H":
            nx = next_num()
            if rel:
                nx += cx
            cx = nx
            add_pt(cx, cy)
            prev_cmd = op
        elif op == "V":
            ny = next_num()
            if rel:
                ny += cy
            cy = ny
            add_pt(cx, cy)
            prev_cmd = op
        elif op == "C":
            x1, y1 = next_num(), next_num()
            x2, y2 = next_num(), next_num()
            x3, y3 = next_num(), next_num()
            if rel:
                x1 += cx
                y1 += cy
                x2 += cx
                y2 += cy
                x3 += cx
                y3 += cy
            for step in range(1, 9):
                t = step / 8.0
                add_pt(cubic_point(cx, x1, x2, x3, t), cubic_point(cy, y1, y2, y3, t))
            prev_cx, prev_cy = x2, y2
            cx, cy = x3, y3
            prev_cmd = op
        elif op == "S":
            rx1 = 2 * cx - prev_cx if prev_cmd in ("C", "S") else cx
            ry1 = 2 * cy - prev_cy if prev_cmd in ("C", "S") else cy
            x2, y2 = next_num(), next_num()
            x3, y3 = next_num(), next_num()
            if rel:
                x2 += cx
                y2 += cy
                x3 += cx
                y3 += cy
            for step in range(1, 9):
                t = step / 8.0
                add_pt(cubic_point(cx, rx1, x2, x3, t), cubic_point(cy, ry1, y2, y3, t))
            prev_cx, prev_cy = x2, y2
            cx, cy = x3, y3
            prev_cmd = op
        elif op == "Q":
            x1, y1 = next_num(), next_num()
            x2, y2 = next_num(), next_num()
            if rel:
                x1 += cx
                y1 += cy
                x2 += cx
                y2 += cy
            for step in range(1, 7):
                t = step / 6.0
                add_pt(quad_point(cx, x1, x2, t), quad_point(cy, y1, y2, t))
            prev_cx, prev_cy = x1, y1
            cx, cy = x2, y2
            prev_cmd = op
        elif op == "T":
            rx1 = 2 * cx - prev_cx if prev_cmd in ("Q", "T") else cx
            ry1 = 2 * cy - prev_cy if prev_cmd in ("Q", "T") else cy
            x2, y2 = next_num(), next_num()
            if rel:
                x2 += cx
                y2 += cy
            for step in range(1, 7):
                t = step / 6.0
                add_pt(quad_point(cx, rx1, x2, t), quad_point(cy, ry1, y2, t))
            prev_cx, prev_cy = rx1, ry1
            cx, cy = x2, y2
            prev_cmd = op
        else:
            raise ValueError(f"Unsupported SVG path command {cmd!r} in {d[:80]}")
    if min_x == float("inf"):
        raise ValueError(f"Empty path bbox for {d[:80]}")
    return min_x, min_y, max_x, max_y


# ---------------------------------------------------------------------------
# Font-to-Path Extraction (Scoped to assets/fonts/stixfonts-2.13b171)
# ---------------------------------------------------------------------------

def extract_font_paths(specs: dict[str, builders.TextSpec]) -> dict[str, tuple[str, tuple[float, float, float, float]]]:
    if not specs:
        return {}
    if not shutil.which("inkscape"):
        raise RuntimeError("inkscape is required to convert STIX font glyphs to SVG paths")

    with tempfile.TemporaryDirectory(prefix="eqntide-fontpath-") as tmp:
        t = Path(tmp)
        fc_path = t / "fonts.conf"
        fc_path.write_text(
            f"""<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir>{FONT_DIR.resolve()}</dir>
</fontconfig>
""",
            encoding="utf-8",
        )
        texts: list[str] = []
        for idx, spec in enumerate(specs.values()):
            style_parts = [
                f"font-family:'{spec.family}'",
                f"font-size:{spec.base_size:.2f}px",
            ]
            if spec.italic:
                style_parts.append("font-style:italic")
            if spec.bold:
                style_parts.append("font-weight:bold")
            texts.append(
                f'<text id="txt-{idx}" data-key="{html.escape(spec.key)}" x="0" y="0" '
                f'style="{";".join(style_parts)}">{html.escape(spec.text)}</text>'
            )
        in_svg = t / "texts.svg"
        out_svg = t / "paths.svg"
        in_svg.write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">'
            + "".join(texts)
            + "</svg>\n",
            encoding="utf-8",
        )
        env = {"PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": str(t), "FONTCONFIG_FILE": str(fc_path)}
        subprocess.run(
            [
                "inkscape",
                str(in_svg),
                "--export-text-to-path",
                "--export-plain-svg",
                f"--export-filename={out_svg}",
            ],
            env=env,
            check=True,
            capture_output=True,
        )
        tree = ET.fromstring(out_svg.read_text(encoding="utf-8"))
        ns_svg = "{http://www.w3.org/2000/svg}"
        by_id: dict[str, ET.Element] = {}
        for elem in tree.iter():
            eid = elem.attrib.get("id")
            if eid and eid.startswith("txt-"):
                by_id[eid] = elem

        result: dict[str, tuple[str, tuple[float, float, float, float]]] = {}
        for idx, spec in enumerate(specs.values()):
            elem = by_id.get(f"txt-{idx}")
            if elem is None:
                raise RuntimeError(f"Inkscape did not emit path for text key {spec.key!r}")
            if elem.tag in ("path", f"{ns_svg}path"):
                d_parts = [elem.attrib["d"]]
            else:
                d_parts = [
                    child.attrib["d"]
                    for child in elem.iter()
                    if child.tag in ("path", f"{ns_svg}path") and "d" in child.attrib
                ]
            combined_d = " ".join(part for part in d_parts if part.strip())
            if not combined_d:
                raise RuntimeError(f"Empty glyph path for {spec.key!r} ({spec.text!r})")
            result[spec.key] = (combined_d, path_bbox(combined_d))
        return result


def place_font_path(
    d: str,
    bbox: tuple[float, float, float, float],
    placement: builders.TextPlacement,
) -> str:
    min_x, min_y, max_x, max_y = bbox
    bw = max(max_x - min_x, 1e-3)
    bh = max(max_y - min_y, 1e-3)
    scale = min(1.0, placement.max_w / bw, placement.max_h / bh)
    gx = (min_x + max_x) / 2.0
    gy = (min_y + max_y) / 2.0
    tx = placement.cx - gx * scale
    ty = placement.baseline_y if placement.align == "baseline" else placement.cy - gy * scale
    if abs(scale - 1.0) < 1e-4:
        transform = f"translate({builders.fmt(tx)} {builders.fmt(ty)})"
    else:
        transform = f"translate({builders.fmt(tx)} {builders.fmt(ty)}) scale({builders.fmt(scale)})"
    return f'<g transform="{transform}">{builders.svg_filled_path(d)}</g>'


# ---------------------------------------------------------------------------
# Atlas Renderer
# ---------------------------------------------------------------------------

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
    raise RuntimeError("No SVG-to-PNG renderer found")


def render_sheet_png(renderer: str, sheet_svg: Path, out_png: Path, width_px: int, height_px: int) -> None:
    out_png.parent.mkdir(parents=True, exist_ok=True)
    if renderer == "resvg":
        subprocess.run(
            ["resvg", "-w", str(width_px), "-h", str(height_px), str(sheet_svg), str(out_png)],
            check=True,
            capture_output=True,
        )
    elif renderer == "rsvg-convert":
        subprocess.run(
            ["rsvg-convert", "-w", str(width_px), "-h", str(height_px), "-o", str(out_png), str(sheet_svg)],
            check=True,
            capture_output=True,
        )
    elif renderer == "cairosvg":
        import cairosvg
        cairosvg.svg2png(
            url=str(sheet_svg),
            write_to=str(out_png),
            output_width=width_px,
            output_height=height_px,
        )
    elif renderer == "inkscape":
        subprocess.run(
            [
                "inkscape",
                str(sheet_svg),
                "--export-type=png",
                f"--export-filename={out_png}",
                "-w",
                str(width_px),
                "-h",
                str(height_px),
            ],
            check=True,
            capture_output=True,
        )
    elif renderer == "magick":
        subprocess.run(
            ["magick", "-background", "none", str(sheet_svg), "-resize", f"{width_px}x{height_px}", str(out_png)],
            check=True,
            capture_output=True,
        )
    else:
        raise ValueError(renderer)

    w, h, color_type = png_dimensions_rgba(out_png)
    if (w, h, color_type) != (width_px, height_px, 6):
        raise RuntimeError(f"Expected {width_px}x{height_px} RGBA (6), got {w}x{h} ({color_type})")


def build_sheet_svg(
    entries: list[CatalogEntry],
    elements_list: list[list[str]],
    sheet_w_dip: int,
    sheet_h_dip: int,
) -> str:
    slot_w = ICON_W + 2 * PAD
    slot_h = ICON_H + 2 * PAD
    sheet_groups: list[str] = []
    for idx, (_entry, full_els) in enumerate(zip(entries, elements_list)):
        col = idx % GRID_COLS
        row = idx // GRID_COLS
        cell_x_dip = col * slot_w + PAD
        cell_y_dip = row * slot_h + PAD
        inner_body = "".join(full_els).replace("currentColor", COLOR)
        sheet_groups.append(
            f'<g transform="translate({cell_x_dip} {cell_y_dip})" '
            f'stroke="{COLOR}" stroke-width="{builders.fmt(builders.STROKE_MAIN)}" '
            f'stroke-linecap="round" stroke-linejoin="round">{inner_body}</g>'
        )
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{sheet_w_dip}" height="{sheet_h_dip}" '
        f'viewBox="0 0 {sheet_w_dip} {sheet_h_dip}" fill="none">'
        + "".join(sheet_groups)
        + "</svg>\n"
    )


def center_elements(
    entries: list[CatalogEntry],
    elements_list: list[list[str]],
    renderer: str,
    sheet_w_dip: int,
    sheet_h_dip: int,
    sheet_w_px: int,
    sheet_h_px: int,
) -> list[list[str]]:
    from PIL import Image

    slot_w = ICON_W + 2 * PAD
    slot_h = ICON_H + 2 * PAD
    with tempfile.TemporaryDirectory(prefix="eqntide-palette-center-") as tmp:
        pass1_svg = Path(tmp) / "pass1.svg"
        pass1_png = Path(tmp) / "pass1.png"
        pass1_svg.write_text(
            build_sheet_svg(entries, elements_list, sheet_w_dip, sheet_h_dip),
            encoding="utf-8",
        )
        render_sheet_png(renderer, pass1_svg, pass1_png, sheet_w_px, sheet_h_px)
        img = Image.open(pass1_png).convert("RGBA")

    centered: list[list[str]] = []
    for idx, (entry, full_els) in enumerate(zip(entries, elements_list)):
        col = idx % GRID_COLS
        row = idx // GRID_COLS
        x0_px = (col * slot_w + PAD) * SCALE
        y0_px = (row * slot_h + PAD) * SCALE
        cell = img.crop((x0_px, y0_px, x0_px + ICON_W * SCALE, y0_px + ICON_H * SCALE))
        bbox = cell.getbbox()
        if bbox is None:
            centered.append(full_els)
            continue
        bx0, by0, bx1, by1 = bbox
        cx_dip = (bx0 + bx1) / (2.0 * SCALE)
        cy_dip = (by0 + by1) / (2.0 * SCALE)
        dx = round(ICON_W / 2.0 - cx_dip, 2)
        keep_baseline_y = entry.group == "Functions" or entry.id == "symbol-ldots"
        dy = 0.0 if keep_baseline_y else round(ICON_H / 2.0 - cy_dip, 2)
        if abs(dx) < 0.2:
            dx = 0.0
        if abs(dy) < 0.2:
            dy = 0.0
        if dx == 0.0 and dy == 0.0:
            centered.append(full_els)
        else:
            centered.append([f'<g transform="translate({builders.fmt(dx)} {builders.fmt(dy)})">{"".join(full_els)}</g>'])
    return centered


def generate_all() -> None:
    entries = parse_catalog()
    builders.FONT_SPECS.clear()
    planned: list[tuple[CatalogEntry, list[str], list[builders.TextPlacement]]] = []
    for entry in entries:
        els, placements = builders.build_entry(entry.group, entry.tid, entry.glyph)
        planned.append((entry, els, placements))

    font_paths = extract_font_paths(builders.FONT_SPECS)

    SVG_OUT_DIR.mkdir(parents=True, exist_ok=True)
    expected_files = {f"{entry.id}.svg" for entry in entries}
    for old_svg in SVG_OUT_DIR.glob("*.svg"):
        if old_svg.name not in expected_files:
            old_svg.unlink()

    slot_w = ICON_W + 2 * PAD
    slot_h = ICON_H + 2 * PAD
    rows = math.ceil(len(entries) / GRID_COLS)
    sheet_w_dip = GRID_COLS * slot_w
    sheet_h_dip = rows * slot_h
    sheet_w_px = sheet_w_dip * SCALE
    sheet_h_px = sheet_h_dip * SCALE

    raw_elements: list[list[str]] = []
    for _entry, els, placements in planned:
        full_els = list(els)
        for p in placements:
            d, bbox = font_paths[p.key]
            full_els.append(place_font_path(d, bbox, p))
        raw_elements.append(full_els)

    renderer = detect_renderer()
    centered_elements = center_elements(
        entries,
        raw_elements,
        renderer,
        sheet_w_dip,
        sheet_h_dip,
        sheet_w_px,
        sheet_h_px,
    )

    atlas_entries: list[dict[str, object]] = []
    for idx, (entry, full_els) in enumerate(zip(entries, centered_elements)):
        svg_text = builders.wrap_svg(full_els)
        svg_path = SVG_OUT_DIR / f"{entry.id}.svg"
        svg_path.write_text(svg_text, encoding="utf-8")

        col = idx % GRID_COLS
        row = idx // GRID_COLS
        cell_x_dip = col * slot_w + PAD
        cell_y_dip = row * slot_h + PAD
        atlas_entries.append(
            {
                "id": entry.id,
                "source": entry.source,
                "x": cell_x_dip * SCALE,
                "y": cell_y_dip * SCALE,
                "w": ICON_W * SCALE,
                "h": ICON_H * SCALE,
                "svg_sha256": sha256_file(svg_path),
            }
        )

    with tempfile.TemporaryDirectory(prefix="eqntide-palette-atlas-") as tmp:
        sheet_svg = Path(tmp) / "sheet.svg"
        sheet_svg.write_text(
            build_sheet_svg(entries, centered_elements, sheet_w_dip, sheet_h_dip),
            encoding="utf-8",
        )
        render_sheet_png(renderer, sheet_svg, ATLAS_PNG_PATH, sheet_w_px, sheet_h_px)

    atlas_meta = {
        "atlas_width": sheet_w_px,
        "atlas_height": sheet_h_px,
        "icon_width": ICON_W,
        "icon_height": ICON_H,
        "scale": SCALE,
        "atlas_sha256": sha256_file(ATLAS_PNG_PATH),
        "entries": atlas_entries,
    }
    ATLAS_JSON_PATH.write_text(json.dumps(atlas_meta, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(entries)} palette SVGs and {sheet_w_px}x{sheet_h_px} atlas using {renderer}.")


def check_all() -> int:
    errors: list[str] = []
    entries = parse_catalog()
    if not ATLAS_PNG_PATH.is_file():
        errors.append(f"Missing atlas PNG: {ATLAS_PNG_PATH}")
    if not ATLAS_JSON_PATH.is_file():
        errors.append(f"Missing atlas JSON: {ATLAS_JSON_PATH}")
    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    meta = json.loads(ATLAS_JSON_PATH.read_text(encoding="utf-8"))
    w, h, color_type = png_dimensions_rgba(ATLAS_PNG_PATH)
    if (w, h, color_type) != (meta["atlas_width"], meta["atlas_height"], 6):
        errors.append(f"Atlas PNG dimension/type mismatch: got {(w, h, color_type)}")
    if meta.get("atlas_sha256") != sha256_file(ATLAS_PNG_PATH):
        errors.append("Atlas PNG checksum mismatch; re-run scripts/generate_palette_icons.py")

    expected_by_id = {e.id: e for e in entries}
    actual_svgs = {p.stem for p in SVG_OUT_DIR.glob("*.svg")}
    if actual_svgs != set(expected_by_id.keys()):
        missing = sorted(set(expected_by_id.keys()) - actual_svgs)
        extra = sorted(actual_svgs - set(expected_by_id.keys()))
        errors.append(f"Palette SVG set mismatch (missing={missing[:5]}, extra={extra[:5]})")

    atlas_by_id = {item["id"]: item for item in meta.get("entries", [])}
    if set(atlas_by_id.keys()) != set(expected_by_id.keys()):
        errors.append("Atlas entry IDs do not match src/catalog/mod.rs")

    for eid, entry in expected_by_id.items():
        svg_path = SVG_OUT_DIR / f"{eid}.svg"
        if not svg_path.is_file():
            continue
        svg_text = svg_path.read_text(encoding="utf-8")
        if "<text" in svg_text:
            errors.append(f"{svg_path}: contains <text> element (must be path-only)")
        item = atlas_by_id.get(eid)
        if not item:
            continue
        if item.get("source") != entry.source:
            errors.append(f"{eid}: source mismatch in atlas.json ({item.get('source')!r} != {entry.source!r})")
        if item.get("svg_sha256") != sha256_file(svg_path):
            errors.append(f"{eid}: SVG changed since last atlas build; re-run scripts/generate_palette_icons.py")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1
    print(f"Verified {len(entries)} palette SVGs and {w}x{h} sprite atlas.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Verify generated SVGs and atlas match catalog")
    args = parser.parse_args()
    if args.check:
        return check_all()
    generate_all()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
