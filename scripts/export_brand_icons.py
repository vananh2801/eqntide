#!/usr/bin/env python3
"""Export brand logo SVG into PNG and multi-size Windows ICO assets.

Usage:
  python3 scripts/export_brand_icons.py          # Regenerate brand PNGs and icon.ico
  python3 scripts/export_brand_icons.py --check  # Verify brand PNG and ICO formats/sizes

Supported SVG renderers (checked in order when exporting):
  resvg, rsvg-convert, cairosvg (Python), inkscape, magick
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = Path(__file__).resolve().parent.parent
LOGO_SVG = ROOT / "assets/brand/logo.svg"
BRAND_PNG = ROOT / "assets/brand/icon.png"
BRAND_ICO = ROOT / "assets/brand/icon.ico"
WPS_ICON_32 = ROOT / "wps-addin/assets/icon-32.png"
WPS_ICON_64 = ROOT / "wps-addin/assets/icon-64.png"

PNG_TARGETS: tuple[tuple[Path, int], ...] = (
    (BRAND_PNG, 512),
    (WPS_ICON_32, 32),
    (WPS_ICON_64, 64),
)
ICO_DIB_SIZES: tuple[int, ...] = (16, 24, 32, 48, 64, 128)
ICO_PNG_SIZE = 256
ICO_ALL_SIZES: tuple[int, ...] = (*ICO_DIB_SIZES, ICO_PNG_SIZE)


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


def png_info_from_bytes(data: bytes, label: str = "PNG") -> tuple[int, int, int, int]:
    if len(data) < 33 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{label} is not a valid PNG")
    length, chunk_type = struct.unpack(">I4s", data[8:16])
    if chunk_type != b"IHDR" or length != 13:
        raise ValueError(f"{label} missing IHDR chunk")
    width, height, bit_depth, color_type, *_ = struct.unpack(">IIBBBBB", data[16:29])
    return width, height, bit_depth, color_type


def render_svg_to_png(renderer: str, svg_path: Path, png_path: Path, size: int) -> None:
    png_path.parent.mkdir(parents=True, exist_ok=True)
    if renderer == "resvg":
        subprocess.run(
            ["resvg", "-w", str(size), "-h", str(size), str(svg_path), str(png_path)],
            check=True,
            capture_output=True,
        )
    elif renderer == "rsvg-convert":
        subprocess.run(
            ["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(png_path), str(svg_path)],
            check=True,
            capture_output=True,
        )
    elif renderer == "cairosvg":
        import cairosvg

        cairosvg.svg2png(
            url=str(svg_path),
            write_to=str(png_path),
            output_width=size,
            output_height=size,
        )
    elif renderer == "inkscape":
        subprocess.run(
            [
                "inkscape",
                str(svg_path),
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
            [
                "magick",
                "-background",
                "none",
                str(svg_path),
                "-resize",
                f"{size}x{size}",
                f"PNG32:{png_path}",
            ],
            check=True,
            capture_output=True,
        )
    else:
        raise ValueError(f"Unsupported renderer: {renderer}")

    w, h, bit_depth, color_type = png_info_from_bytes(png_path.read_bytes(), str(png_path))
    if (w, h, bit_depth, color_type) != (size, size, 8, 6):
        raise RuntimeError(
            f"Rendered {png_path} as {w}x{h} bit_depth={bit_depth} color_type={color_type}, "
            f"expected {size}x{size} 8-bit RGBA (6)"
        )


def decode_rgba_png(png_bytes: bytes) -> tuple[int, int, bytes]:
    """Decode an 8-bit non-interlaced RGBA PNG into raw top-down RGBA bytes."""
    width, height, bit_depth, color_type = png_info_from_bytes(png_bytes)
    if (bit_depth, color_type) != (8, 6):
        raise ValueError(f"Expected 8-bit RGBA PNG, got bit_depth={bit_depth} color_type={color_type}")

    idat_chunks: list[bytes] = []
    pos = 8
    while pos < len(png_bytes):
        length, chunk_type = struct.unpack(">I4s", png_bytes[pos : pos + 8])
        data_start = pos + 8
        data_end = data_start + length
        if chunk_type == b"IHDR":
            interlace = png_bytes[data_start + 12]
            if interlace != 0:
                raise ValueError("Interlaced PNGs are not supported")
        elif chunk_type == b"IDAT":
            idat_chunks.append(png_bytes[data_start:data_end])
        elif chunk_type == b"IEND":
            break
        pos = data_end + 4

    raw = zlib.decompress(b"".join(idat_chunks))
    bpp = 4
    stride = width * bpp
    expected_len = height * (1 + stride)
    if len(raw) != expected_len:
        raise ValueError(f"Unexpected decompressed PNG size: {len(raw)} != {expected_len}")

    out = bytearray(height * stride)
    prev_row = bytes(stride)
    for row_idx in range(height):
        src_start = row_idx * (1 + stride)
        filter_type = raw[src_start]
        scanline = raw[src_start + 1 : src_start + 1 + stride]
        recon = bytearray(stride)

        if filter_type == 0:
            recon[:] = scanline
        elif filter_type == 1:
            for col in range(stride):
                left = recon[col - bpp] if col >= bpp else 0
                recon[col] = (scanline[col] + left) & 0xFF
        elif filter_type == 2:
            for col in range(stride):
                recon[col] = (scanline[col] + prev_row[col]) & 0xFF
        elif filter_type == 3:
            for col in range(stride):
                left = recon[col - bpp] if col >= bpp else 0
                up = prev_row[col]
                recon[col] = (scanline[col] + ((left + up) >> 1)) & 0xFF
        elif filter_type == 4:
            for col in range(stride):
                left = recon[col - bpp] if col >= bpp else 0
                up = prev_row[col]
                up_left = prev_row[col - bpp] if col >= bpp else 0
                p = left + up - up_left
                pa = abs(p - left)
                pb = abs(p - up)
                pc = abs(p - up_left)
                if pa <= pb and pa <= pc:
                    predictor = left
                elif pb <= pc:
                    predictor = up
                else:
                    predictor = up_left
                recon[col] = (scanline[col] + predictor) & 0xFF
        else:
            raise ValueError(f"Unsupported PNG filter type: {filter_type}")

        dst_start = row_idx * stride
        out[dst_start : dst_start + stride] = recon
        prev_row = bytes(recon)

    return width, height, bytes(out)


def rgba_png_to_ico_dib(png_bytes: bytes) -> bytes:
    """Encode an 8-bit RGBA PNG as a 32-bit ICO DIB (BITMAPINFOHEADER + BGRA + AND mask)."""
    width, height, rgba = decode_rgba_png(png_bytes)
    xor_stride = width * 4
    xor_mask = bytearray(height * xor_stride)
    and_stride = ((width + 31) // 32) * 4
    and_mask = bytearray(height * and_stride)

    for dst_row in range(height):
        src_row = height - 1 - dst_row
        src_row_off = src_row * xor_stride
        dst_row_off = dst_row * xor_stride
        and_row_off = dst_row * and_stride
        for col in range(width):
            sp = src_row_off + col * 4
            dp = dst_row_off + col * 4
            r, g, b, a = rgba[sp : sp + 4]
            xor_mask[dp : dp + 4] = bytes((b, g, r, a))
            if a == 0:
                and_mask[and_row_off + (col // 8)] |= 1 << (7 - (col % 8))

    header = struct.pack(
        "<IiiHHIIiiII",
        40,  # biSize
        width,  # biWidth
        height * 2,  # biHeight (XOR + AND)
        1,  # biPlanes
        32,  # biBitCount
        0,  # biCompression (BI_RGB)
        len(xor_mask) + len(and_mask),  # biSizeImage
        0,  # biXPelsPerMeter
        0,  # biYPelsPerMeter
        0,  # biClrUsed
        0,  # biClrImportant
    )
    return header + bytes(xor_mask) + bytes(and_mask)


def build_ico(entries: list[tuple[int, bytes]]) -> bytes:
    """Assemble a Windows .ico container from `(size, payload_bytes)` pairs."""
    header = struct.pack("<HHH", 0, 1, len(entries))
    directory = bytearray()
    payloads = bytearray()
    offset = 6 + len(entries) * 16
    for size, data in entries:
        dim = 0 if size >= 256 else size
        directory += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        payloads += data
        offset += len(data)
    return header + bytes(directory) + bytes(payloads)


def export_all() -> None:
    if not LOGO_SVG.is_file():
        raise FileNotFoundError(f"Missing source SVG: {LOGO_SVG}")
    renderer = detect_renderer()

    for out_path, size in PNG_TARGETS:
        render_svg_to_png(renderer, LOGO_SVG, out_path, size)

    ico_entries: list[tuple[int, bytes]] = []
    with tempfile.TemporaryDirectory(prefix="eqntide-brand-") as tmp:
        tmp_dir = Path(tmp)
        for size in ICO_DIB_SIZES:
            tmp_png = tmp_dir / f"icon-{size}.png"
            render_svg_to_png(renderer, LOGO_SVG, tmp_png, size)
            ico_entries.append((size, rgba_png_to_ico_dib(tmp_png.read_bytes())))
        png_256 = tmp_dir / f"icon-{ICO_PNG_SIZE}.png"
        render_svg_to_png(renderer, LOGO_SVG, png_256, ICO_PNG_SIZE)
        ico_entries.append((ICO_PNG_SIZE, png_256.read_bytes()))

    BRAND_ICO.parent.mkdir(parents=True, exist_ok=True)
    BRAND_ICO.write_bytes(build_ico(ico_entries))
    print(
        f"Exported {len(PNG_TARGETS)} PNGs and {len(ico_entries)}-size ICO "
        f"from {LOGO_SVG.relative_to(ROOT)} using {renderer}."
    )


def check_all() -> int:
    errors: list[str] = []
    if not LOGO_SVG.is_file():
        errors.append(f"Missing source SVG: {LOGO_SVG}")

    for out_path, size in PNG_TARGETS:
        if not out_path.is_file():
            errors.append(f"Missing exported PNG: {out_path}")
            continue
        try:
            w, h, bit_depth, color_type = png_info_from_bytes(out_path.read_bytes(), str(out_path))
            if (w, h, bit_depth, color_type) != (size, size, 8, 6):
                errors.append(
                    f"{out_path}: expected {size}x{size} 8-bit RGBA (6), "
                    f"got {w}x{h} bit_depth={bit_depth} color_type={color_type}"
                )
        except ValueError as exc:
            errors.append(str(exc))

    if not BRAND_ICO.is_file():
        errors.append(f"Missing exported ICO: {BRAND_ICO}")
    else:
        ico = BRAND_ICO.read_bytes()
        if len(ico) < 6:
            errors.append(f"{BRAND_ICO}: truncated ICO header")
        else:
            reserved, ico_type, count = struct.unpack("<HHH", ico[:6])
            if (reserved, ico_type) != (0, 1):
                errors.append(f"{BRAND_ICO}: invalid ICO header ({reserved}, {ico_type})")
            elif len(ico) < 6 + count * 16:
                errors.append(f"{BRAND_ICO}: truncated ICO directory")
            else:
                found_sizes: list[int] = []
                for idx in range(count):
                    off = 6 + idx * 16
                    raw_w, raw_h, _cc, _res, planes, bpp, byte_len, img_off = struct.unpack(
                        "<BBBBHHII", ico[off : off + 16]
                    )
                    w = 256 if raw_w == 0 else raw_w
                    h = 256 if raw_h == 0 else raw_h
                    if w != h:
                        errors.append(f"{BRAND_ICO} entry {idx}: non-square icon {w}x{h}")
                    if planes != 1 or bpp != 32:
                        errors.append(f"{BRAND_ICO} entry {idx} ({w}x{h}): expected 1 plane 32bpp, got {planes}/{bpp}")
                    if img_off + byte_len > len(ico):
                        errors.append(f"{BRAND_ICO} entry {idx} ({w}x{h}): out-of-bounds payload")
                        continue
                    payload = ico[img_off : img_off + byte_len]
                    if w == ICO_PNG_SIZE:
                        try:
                            pw, ph, pbit, pcol = png_info_from_bytes(payload, f"{BRAND_ICO}[{w}]")
                            if (pw, ph, pbit, pcol) != (w, h, 8, 6):
                                errors.append(f"{BRAND_ICO} entry {w}: expected RGBA PNG, got {(pw, ph, pbit, pcol)}")
                        except ValueError as exc:
                            errors.append(str(exc))
                    else:
                        if len(payload) < 40:
                            errors.append(f"{BRAND_ICO} entry {w}: truncated DIB header")
                        else:
                            bi_size, bi_w, bi_h, bi_planes, bi_bpp, bi_comp = struct.unpack(
                                "<IiiHHI", payload[:20]
                            )
                            if (bi_size, bi_w, bi_h, bi_planes, bi_bpp, bi_comp) != (40, w, h * 2, 1, 32, 0):
                                errors.append(
                                    f"{BRAND_ICO} entry {w}: unexpected DIB header "
                                    f"{(bi_size, bi_w, bi_h, bi_planes, bi_bpp, bi_comp)}"
                                )
                    found_sizes.append(w)
                if tuple(found_sizes) != ICO_ALL_SIZES:
                    errors.append(
                        f"{BRAND_ICO}: expected sizes {ICO_ALL_SIZES}, got {tuple(found_sizes)}"
                    )

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1
    print(f"Verified {len(PNG_TARGETS)} brand PNGs and {len(ICO_ALL_SIZES)}-size {BRAND_ICO.relative_to(ROOT)}.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify brand PNGs and icon.ico match expected formats and dimensions",
    )
    args = parser.parse_args()
    if args.check:
        return check_all()
    export_all()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
