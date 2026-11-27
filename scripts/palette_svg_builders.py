"""Parametric Tabler-style SVG geometry builders for all 547 EqnTide palette entries.

ViewBox is 40x26 DIP. Structural geometry uses uniform monoline strokes matching
Tabler Icons (stroke-width=1.7, round linecap/linejoin). Empty slots (#?) use
thin, muted solid rounded rectangles (stroke-width=1.15, opacity=0.68).
Font-derived letterforms (Functions, Greek, blackboard/script/fraktur Symbols,
and composite labels like `lim` or `d`) are injected as normalized `<path>` nodes.
"""

from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Callable

W = 40.0
H = 26.0
CX = 20.0
CY = 13.0

STROKE_MAIN = 1.7
STROKE_MED = 1.45
STROKE_THIN = 1.25
SLOT_STROKE = 1.15
SLOT_OPACITY = 0.68
SLOT_RX = 1.1


@dataclass(frozen=True)
class TextSpec:
    """Font text specification extracted to an SVG <path> via scoped STIX fonts."""

    key: str
    text: str
    family: str  # "STIX Two Text" or "STIX Two Math"
    italic: bool = False
    bold: bool = False
    base_size: float = 15.0


@dataclass(frozen=True)
class TextPlacement:
    """Target placement box inside 40x26 for an extracted font <path>."""

    key: str
    cx: float
    cy: float
    max_w: float
    max_h: float
    align: str = "center"  # "center" or "baseline"
    baseline_y: float = 17.5


# Registry of font-derived glyph/word paths needed by builders.
FONT_SPECS: dict[str, TextSpec] = {}


def reg_text(
    key: str,
    text: str,
    family: str = "STIX Two Math",
    *,
    italic: bool = False,
    bold: bool = False,
    base_size: float = 15.0,
) -> str:
    FONT_SPECS[key] = TextSpec(
        key=key,
        text=text,
        family=family,
        italic=italic,
        bold=bold,
        base_size=base_size,
    )
    return key


# ---------------------------------------------------------------------------
# SVG Primitive Helpers
# ---------------------------------------------------------------------------

def fmt(v: float) -> str:
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return "0" if s == "-0" or s == "" else s


def svg_path(d: str, sw: float = STROKE_MAIN, opacity: float | None = None) -> str:
    op = f' opacity="{fmt(opacity)}"' if opacity is not None else ""
    sw_attr = "" if abs(sw - STROKE_MAIN) < 1e-3 else f' stroke-width="{fmt(sw)}"'
    return f'<path d="{d}"{sw_attr}{op} />'


def svg_filled_path(d: str, opacity: float | None = None) -> str:
    op = f' opacity="{fmt(opacity)}"' if opacity is not None else ""
    return f'<path d="{d}" fill="currentColor" stroke="none"{op} />'


def svg_dot(x: float, y: float, r: float = 0.95) -> str:
    return f'<circle cx="{fmt(x)}" cy="{fmt(y)}" r="{fmt(r)}" fill="currentColor" stroke="none" />'


def svg_circle(cx: float, cy: float, r: float, sw: float = STROKE_MAIN) -> str:
    sw_attr = "" if abs(sw - STROKE_MAIN) < 1e-3 else f' stroke-width="{fmt(sw)}"'
    return f'<circle cx="{fmt(cx)}" cy="{fmt(cy)}" r="{fmt(r)}"{sw_attr} />'


def slot(
    x: float,
    y: float,
    w: float,
    h: float,
    *,
    sw: float = SLOT_STROKE,
    rx: float = SLOT_RX,
    opacity: float = SLOT_OPACITY,
) -> str:
    return (
        f'<rect x="{fmt(x)}" y="{fmt(y)}" width="{fmt(w)}" height="{fmt(h)}" '
        f'rx="{fmt(rx)}" stroke-width="{fmt(sw)}" opacity="{fmt(opacity)}" />'
    )


def slot_c(cx: float, cy: float, w: float, h: float, **kw: float) -> str:
    return slot(cx - w / 2.0, cy - h / 2.0, w, h, **kw)


def wrap_svg(elements: list[str]) -> str:
    body = "".join(elements)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{int(W)}" height="{int(H)}" '
        f'viewBox="0 0 {int(W)} {int(H)}" fill="none" stroke="currentColor" '
        f'stroke-width="{fmt(STROKE_MAIN)}" stroke-linecap="round" stroke-linejoin="round">'
        f"{body}</svg>\n"
    )


def arrow_head(tip_x: float, tip_y: float, angle_deg: float, size: float = 3.4, spread_deg: float = 38.0) -> str:
    a1 = math.radians(angle_deg + 180.0 - spread_deg)
    a2 = math.radians(angle_deg + 180.0 + spread_deg)
    x1, y1 = tip_x + size * math.cos(a1), tip_y + size * math.sin(a1)
    x2, y2 = tip_x + size * math.cos(a2), tip_y + size * math.sin(a2)
    return svg_path(f"M{fmt(x1)} {fmt(y1)} L{fmt(tip_x)} {fmt(tip_y)} L{fmt(x2)} {fmt(y2)}")


def arrow_head_sw(
    tip_x: float, tip_y: float, angle_deg: float, sw: float, size: float = 3.2, spread_deg: float = 38.0
) -> str:
    a1 = math.radians(angle_deg + 180.0 - spread_deg)
    a2 = math.radians(angle_deg + 180.0 + spread_deg)
    x1, y1 = tip_x + size * math.cos(a1), tip_y + size * math.sin(a1)
    x2, y2 = tip_x + size * math.cos(a2), tip_y + size * math.sin(a2)
    return svg_path(f"M{fmt(x1)} {fmt(y1)} L{fmt(tip_x)} {fmt(tip_y)} L{fmt(x2)} {fmt(y2)}", sw=sw)


# ---------------------------------------------------------------------------
# Delimiter / Fence Primitives
# ---------------------------------------------------------------------------

def fence_glyph(kind: str, x: float, y0: float, y1: float, side: str, sw: float = STROKE_MAIN) -> list[str]:
    ym = (y0 + y1) / 2.0
    h = y1 - y0
    if kind == "paren":
        r = h * 0.765
        s = r - math.sqrt(max(r * r - (h / 2.0) ** 2, 0.0))
        if side == "left":
            return [svg_path(f"M{fmt(x + s)} {fmt(y0)} A{fmt(r)} {fmt(r)} 0 0 0 {fmt(x + s)} {fmt(y1)}", sw=sw)]
        return [svg_path(f"M{fmt(x - s)} {fmt(y0)} A{fmt(r)} {fmt(r)} 0 0 1 {fmt(x - s)} {fmt(y1)}", sw=sw)]
    if kind == "bracket":
        tab = 2.6
        if side == "left":
            return [svg_path(f"M{fmt(x + tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)} H{fmt(x + tab)}", sw=sw)]
        return [svg_path(f"M{fmt(x - tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)} H{fmt(x - tab)}", sw=sw)]
    if kind == "brace":
        dx = 2.6
        if side == "left":
            return [
                svg_path(
                    f"M{fmt(x + dx)} {fmt(y0)} Q{fmt(x + 0.4)} {fmt(y0)} {fmt(x + 0.4)} {fmt(y0 + 3.0)} "
                    f"V{fmt(ym - 2.0)} Q{fmt(x + 0.4)} {fmt(ym)} {fmt(x - 1.4)} {fmt(ym)} "
                    f"Q{fmt(x + 0.4)} {fmt(ym)} {fmt(x + 0.4)} {fmt(ym + 2.0)} "
                    f"V{fmt(y1 - 3.0)} Q{fmt(x + 0.4)} {fmt(y1)} {fmt(x + dx)} {fmt(y1)}",
                    sw=sw,
                )
            ]
        return [
            svg_path(
                f"M{fmt(x - dx)} {fmt(y0)} Q{fmt(x - 0.4)} {fmt(y0)} {fmt(x - 0.4)} {fmt(y0 + 3.0)} "
                f"V{fmt(ym - 2.0)} Q{fmt(x - 0.4)} {fmt(ym)} {fmt(x + 1.4)} {fmt(ym)} "
                f"Q{fmt(x - 0.4)} {fmt(ym)} {fmt(x - 0.4)} {fmt(ym + 2.0)} "
                f"V{fmt(y1 - 3.0)} Q{fmt(x - 0.4)} {fmt(y1)} {fmt(x - dx)} {fmt(y1)}",
                sw=sw,
            )
        ]
    if kind == "angle":
        dx = 3.2
        if side == "left":
            return [svg_path(f"M{fmt(x + dx)} {fmt(y0)} L{fmt(x)} {fmt(ym)} L{fmt(x + dx)} {fmt(y1)}", sw=sw)]
        return [svg_path(f"M{fmt(x - dx)} {fmt(y0)} L{fmt(x)} {fmt(ym)} L{fmt(x - dx)} {fmt(y1)}", sw=sw)]
    if kind == "bar":
        return [svg_path(f"M{fmt(x)} {fmt(y0)} V{fmt(y1)}", sw=sw)]
    if kind == "norm":
        off = 1.35
        return [
            svg_path(f"M{fmt(x - off)} {fmt(y0)} V{fmt(y1)}", sw=min(sw, 1.45)),
            svg_path(f"M{fmt(x + off)} {fmt(y0)} V{fmt(y1)}", sw=min(sw, 1.45)),
        ]
    if kind == "floor":
        tab = 2.6
        if side == "left":
            return [svg_path(f"M{fmt(x)} {fmt(y0)} V{fmt(y1)} H{fmt(x + tab)}", sw=sw)]
        return [svg_path(f"M{fmt(x)} {fmt(y0)} V{fmt(y1)} H{fmt(x - tab)}", sw=sw)]
    if kind == "ceiling":
        tab = 2.6
        if side == "left":
            return [svg_path(f"M{fmt(x + tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)}", sw=sw)]
        return [svg_path(f"M{fmt(x - tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)}", sw=sw)]
    if kind == "double-bracket":
        tab = 2.8
        gap = 2.1
        if side == "left":
            return [
                svg_path(f"M{fmt(x + tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)} H{fmt(x + tab)}", sw=1.35),
                svg_path(f"M{fmt(x + gap)} {fmt(y0)} V{fmt(y1)}", sw=1.35),
            ]
        return [
            svg_path(f"M{fmt(x - tab)} {fmt(y0)} H{fmt(x)} V{fmt(y1)} H{fmt(x - tab)}", sw=1.35),
            svg_path(f"M{fmt(x - gap)} {fmt(y0)} V{fmt(y1)}", sw=1.35),
        ]
    raise ValueError(f"Unknown fence kind: {kind}")


# ---------------------------------------------------------------------------
# Big Operator & Integral Glyphs
# ---------------------------------------------------------------------------

def draw_integral(cx: float, cy: float, h: float, sw: float = STROKE_MAIN, count: int = 1, loop: str = "") -> list[str]:
    half = h / 2.0
    slant = h * 0.12
    curve = h * 0.14
    spacing = 3.8 if count <= 2 else 3.2
    xs = [cx + (i - (count - 1) / 2.0) * spacing for i in range(count)]
    out: list[str] = []
    for x in xs:
        y0, y1 = cy - half, cy + half
        out.append(
            svg_path(
                f"M{fmt(x + slant + 1.1)} {fmt(y0 + 0.9)} "
                f"C{fmt(x + slant)} {fmt(y0 - 0.5)} {fmt(x + 0.4)} {fmt(y0)} {fmt(x + 0.3)} {fmt(y0 + curve)} "
                f"L{fmt(x - 0.3)} {fmt(y1 - curve)} "
                f"C{fmt(x - 0.4)} {fmt(y1)} {fmt(x - slant)} {fmt(y1 + 0.5)} {fmt(x - slant - 1.1)} {fmt(y1 - 0.9)}",
                sw=sw,
            )
        )
    if loop:
        rx = (count - 1) * spacing * 0.5 + 2.6
        ry = 2.4 if count == 1 else 2.2
        out.append(
            f'<ellipse cx="{fmt(cx)}" cy="{fmt(cy)}" rx="{fmt(rx)}" ry="{fmt(ry)}" stroke-width="{fmt(min(sw, 1.35))}" />'
        )
        if loop in ("cw", "ccw"):
            # Clockwise travel points down at the right edge of the loop.
            direction = 1.0 if loop == "cw" else -1.0
            arrow_x = cx + rx
            wings_y = cy - direction * 1.3
            tip_y = cy + direction * 0.7
            out.append(
                svg_path(
                    f"M{fmt(arrow_x - 1.05)} {fmt(wings_y)} "
                    f"L{fmt(arrow_x)} {fmt(tip_y)} "
                    f"L{fmt(arrow_x + 1.05)} {fmt(wings_y)}",
                    sw=1.15,
                )
            )
    return out


def draw_big_op(kind: str, cx: float, cy: float, h: float, sw: float = STROKE_MAIN) -> list[str]:
    half_h = h / 2.0
    half_w = h * 0.36
    y0, y1 = cy - half_h, cy + half_h
    x0, x1 = cx - half_w, cx + half_w
    if kind == "sum":
        ser = 1.6
        return [
            svg_path(
                f"M{fmt(x1)} {fmt(y0 + ser)} V{fmt(y0)} H{fmt(x0)} L{fmt(cx + 0.8)} {fmt(cy)} "
                f"L{fmt(x0)} {fmt(y1)} H{fmt(x1)} V{fmt(y1 - ser)}",
                sw=sw,
            )
        ]
    if kind == "prod":
        inset = half_w * 0.28
        return [
            svg_path(f"M{fmt(x0 - 0.6)} {fmt(y0)} H{fmt(x1 + 0.6)}", sw=sw),
            svg_path(f"M{fmt(x0 + inset)} {fmt(y0)} V{fmt(y1)}", sw=sw),
            svg_path(f"M{fmt(x1 - inset)} {fmt(y0)} V{fmt(y1)}", sw=sw),
        ]
    if kind == "coprod":
        inset = half_w * 0.28
        return [
            svg_path(f"M{fmt(x0 - 0.6)} {fmt(y1)} H{fmt(x1 + 0.6)}", sw=sw),
            svg_path(f"M{fmt(x0 + inset)} {fmt(y0)} V{fmt(y1)}", sw=sw),
            svg_path(f"M{fmt(x1 - inset)} {fmt(y0)} V{fmt(y1)}", sw=sw),
        ]
    if kind == "bigcap":
        return [
            svg_path(
                f"M{fmt(x0)} {fmt(y1)} V{fmt(cy - half_h * 0.15)} "
                f"A{fmt(half_w)} {fmt(half_h * 0.85)} 0 0 1 {fmt(x1)} {fmt(cy - half_h * 0.15)} V{fmt(y1)}",
                sw=sw,
            )
        ]
    if kind == "bigcup":
        return [
            svg_path(
                f"M{fmt(x0)} {fmt(y0)} V{fmt(cy + half_h * 0.15)} "
                f"A{fmt(half_w)} {fmt(half_h * 0.85)} 0 0 0 {fmt(x1)} {fmt(cy + half_h * 0.15)} V{fmt(y0)}",
                sw=sw,
            )
        ]
    if kind == "bigwedge":
        return [svg_path(f"M{fmt(x0)} {fmt(y1)} L{fmt(cx)} {fmt(y0)} L{fmt(x1)} {fmt(y1)}", sw=sw)]
    if kind == "bigvee":
        return [svg_path(f"M{fmt(x0)} {fmt(y0)} L{fmt(cx)} {fmt(y1)} L{fmt(x1)} {fmt(y0)}", sw=sw)]
    if kind in ("bigotimes", "bigoplus", "bigodot"):
        r = half_h * 0.86
        out = [svg_circle(cx, cy, r, sw=sw)]
        if kind == "bigotimes":
            d = r * 0.707
            out.append(svg_path(f"M{fmt(cx - d)} {fmt(cy - d)} L{fmt(cx + d)} {fmt(cy + d)}", sw=min(sw, 1.35)))
            out.append(svg_path(f"M{fmt(cx - d)} {fmt(cy + d)} L{fmt(cx + d)} {fmt(cy - d)}", sw=min(sw, 1.35)))
        elif kind == "bigoplus":
            out.append(svg_path(f"M{fmt(cx - r)} {fmt(cy)} H{fmt(cx + r)}", sw=min(sw, 1.35)))
            out.append(svg_path(f"M{fmt(cx)} {fmt(cy - r)} V{fmt(cy + r)}", sw=min(sw, 1.35)))
        else:
            out.append(svg_dot(cx, cy, r=max(1.1, r * 0.18)))
        return out
    raise ValueError(f"Unknown big op: {kind}")


# ---------------------------------------------------------------------------
# Arrow Primitives
# ---------------------------------------------------------------------------

def draw_arrow_shape(kind: str, cx: float, cy: float, span: float, sw: float = STROKE_MAIN) -> list[str]:
    x0, x1 = cx - span / 2.0, cx + span / 2.0
    head = 3.2 if span >= 16.0 else 2.8
    if kind == "rightarrow":
        return [svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)}", sw=sw), arrow_head_sw(x1, cy, 0, sw, head)]
    if kind == "leftarrow":
        return [svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)}", sw=sw), arrow_head_sw(x0, cy, 180, sw, head)]
    if kind == "leftrightarrow":
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)}", sw=sw),
            arrow_head_sw(x0, cy, 180, sw, head),
            arrow_head_sw(x1, cy, 0, sw, head),
        ]
    if kind == "Rightarrow":
        gap = 1.65
        dbl_head = 5.4 if span >= 18.0 else 4.8
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy - gap)} H{fmt(x1 - 1.7)}", sw=min(sw, 1.4)),
            svg_path(f"M{fmt(x0)} {fmt(cy + gap)} H{fmt(x1 - 1.7)}", sw=min(sw, 1.4)),
            arrow_head_sw(x1, cy, 0, min(sw, 1.5), dbl_head, 45),
        ]
    if kind == "Leftarrow":
        gap = 1.65
        dbl_head = 5.4 if span >= 18.0 else 4.8
        return [
            svg_path(f"M{fmt(x0 + 1.7)} {fmt(cy - gap)} H{fmt(x1)}", sw=min(sw, 1.4)),
            svg_path(f"M{fmt(x0 + 1.7)} {fmt(cy + gap)} H{fmt(x1)}", sw=min(sw, 1.4)),
            arrow_head_sw(x0, cy, 180, min(sw, 1.5), dbl_head, 45),
        ]
    if kind == "Leftrightarrow":
        gap = 1.65
        dbl_head = 5.4 if span >= 18.0 else 4.8
        return [
            svg_path(f"M{fmt(x0 + 1.7)} {fmt(cy - gap)} H{fmt(x1 - 1.7)}", sw=min(sw, 1.4)),
            svg_path(f"M{fmt(x0 + 1.7)} {fmt(cy + gap)} H{fmt(x1 - 1.7)}", sw=min(sw, 1.4)),
            arrow_head_sw(x0, cy, 180, min(sw, 1.5), dbl_head, 45),
            arrow_head_sw(x1, cy, 0, min(sw, 1.5), dbl_head, 45),
        ]
    if kind == "rightharpoonup":
        return [svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)} L{fmt(x1 - head)} {fmt(cy - head * 0.72)}", sw=sw)]
    if kind == "rightharpoondown":
        return [svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)} L{fmt(x1 - head)} {fmt(cy + head * 0.72)}", sw=sw)]
    if kind == "leftharpoonup":
        return [svg_path(f"M{fmt(x1)} {fmt(cy)} H{fmt(x0)} L{fmt(x0 + head)} {fmt(cy - head * 0.72)}", sw=sw)]
    if kind == "leftharpoondown":
        return [svg_path(f"M{fmt(x1)} {fmt(cy)} H{fmt(x0)} L{fmt(x0 + head)} {fmt(cy + head * 0.72)}", sw=sw)]
    if kind == "rightleftharpoons":
        g = 2.0
        h2 = 2.8
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy - g)} H{fmt(x1)} L{fmt(x1 - h2)} {fmt(cy - g - 2.1)}", sw=min(sw, 1.45)),
            svg_path(f"M{fmt(x1)} {fmt(cy + g)} H{fmt(x0)} L{fmt(x0 + h2)} {fmt(cy + g + 2.1)}", sw=min(sw, 1.45)),
        ]
    if kind == "leftrightharpoons":
        g = 2.0
        h2 = 2.8
        return [
            svg_path(f"M{fmt(x1)} {fmt(cy - g)} H{fmt(x0)} L{fmt(x0 + h2)} {fmt(cy - g - 2.1)}", sw=min(sw, 1.45)),
            svg_path(f"M{fmt(x0)} {fmt(cy + g)} H{fmt(x1)} L{fmt(x1 - h2)} {fmt(cy + g + 2.1)}", sw=min(sw, 1.45)),
        ]
    if kind == "mapsto":
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy - 2.4)} V{fmt(cy + 2.4)}", sw=sw),
            svg_path(f"M{fmt(x0)} {fmt(cy)} H{fmt(x1)}", sw=sw),
            arrow_head_sw(x1, cy, 0, sw, head),
        ]
    if kind == "rightleftarrows":
        g = 2.4
        h2 = 2.6
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy - g)} H{fmt(x1)}", sw=min(sw, 1.4)),
            arrow_head_sw(x1, cy - g, 0, min(sw, 1.4), h2),
            svg_path(f"M{fmt(x0)} {fmt(cy + g)} H{fmt(x1)}", sw=min(sw, 1.4)),
            arrow_head_sw(x0, cy + g, 180, min(sw, 1.4), h2),
        ]
    if kind == "leftrightarrows":
        g = 2.4
        h2 = 2.6
        return [
            svg_path(f"M{fmt(x0)} {fmt(cy - g)} H{fmt(x1)}", sw=min(sw, 1.4)),
            arrow_head_sw(x0, cy - g, 180, min(sw, 1.4), h2),
            svg_path(f"M{fmt(x0)} {fmt(cy + g)} H{fmt(x1)}", sw=min(sw, 1.4)),
            arrow_head_sw(x1, cy + g, 0, min(sw, 1.4), h2),
        ]
    raise ValueError(f"Unknown arrow kind: {kind}")


# ---------------------------------------------------------------------------
# Group Builders
# ---------------------------------------------------------------------------

def build_fractions(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "frac":
        return [slot_c(CX, 6.2, 9.5, 6.8), svg_path("M9.5 13 H30.5"), slot_c(CX, 19.8, 9.5, 6.8)], []
    if tid == "tfrac":
        return [slot_c(CX, 7.2, 7.5, 5.5), svg_path("M12 13 H28", sw=1.45), slot_c(CX, 18.8, 7.5, 5.5)], []
    if tid == "dfrac":
        return [slot_c(CX, 5.8, 12.0, 7.6), svg_path("M7 13 H33", sw=1.85), slot_c(CX, 20.2, 12.0, 7.6)], []
    if tid == "sfrac":
        return [
            slot_c(12.2, 7.4, 8.0, 6.2),
            svg_path("M23.5 4.2 L16.5 21.8"),
            slot_c(27.8, 18.6, 8.0, 6.2),
        ], []
    if tid == "binom":
        els = fence_glyph("paren", 9.0, 3.0, 23.0, "left") + fence_glyph("paren", 31.0, 3.0, 23.0, "right")
        els += [slot_c(CX, 7.2, 9.0, 6.4), slot_c(CX, 18.8, 9.0, 6.4)]
        return els, []
    if tid == "linear":
        return [
            slot_c(11.0, CY, 9.0, 9.5),
            svg_path("M22.8 5.2 L17.2 20.8"),
            slot_c(29.0, CY, 9.0, 9.5),
        ], []
    if tid == "small-linear":
        return [
            slot_c(12.5, CY, 7.2, 7.2),
            svg_path("M22.2 6.8 L17.8 19.2", sw=1.45),
            slot_c(27.5, CY, 7.2, 7.2),
        ], []
    if tid == "sqrt":
        return [
            svg_path("M5.5 14 H8.5 L12.5 21.5 L17.5 4.5 H34.5"),
            slot_c(25.5, 14.5, 11.0, 10.5),
        ], []
    if tid == "root":
        return [
            slot_c(7.5, 6.8, 6.0, 5.6),
            svg_path("M6.5 14.8 H9.5 L13.5 21.8 L18.5 4.5 H34.5"),
            slot_c(26.0, 14.5, 10.5, 10.5),
        ], []
    if tid == "root-fraction":
        return [
            svg_path("M5.5 14.2 H8.5 L12.5 22 L17.5 3.5 H34.5"),
            slot_c(26.0, 8.2, 7.6, 5.0),
            svg_path("M19.5 13.5 H32.5", sw=1.45),
            slot_c(26.0, 18.8, 7.6, 5.0),
        ], []
    if tid == "small-root":
        return [
            svg_path("M7 14.2 H9.8 L13.2 20.8 L17.6 5.2 H33"),
            slot_c(25.2, 9.2, 6.6, 4.4),
            svg_path("M19.8 13.8 H30.6", sw=1.35),
            slot_c(25.2, 18.2, 6.6, 4.4),
        ], []
    if tid == "long-division":
        return [
            svg_path("M10.5 21 C13 17.5 13 9.5 10.5 6.5 H31"),
            slot_c(21.5, 14.5, 11.5, 9.5),
        ], []
    if tid == "long-division-quotient":
        return [
            slot_c(21.5, 5.0, 9.5, 5.2),
            svg_path("M10.5 22.8 C13 19.5 13 12.5 10.5 10 H31"),
            slot_c(21.5, 16.8, 10.5, 8.2),
        ], []
    raise ValueError(tid)


def build_scripts(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "sup":
        return [slot_c(16.0, 14.5, 10.5, 11.5), slot_c(26.8, 7.2, 6.5, 6.0)], []
    if tid == "sub":
        return [slot_c(16.0, 11.5, 10.5, 11.5), slot_c(26.8, 18.8, 6.5, 6.0)], []
    if tid == "subsup":
        return [
            slot_c(16.0, CY, 10.0, 11.0),
            slot_c(26.5, 6.8, 6.5, 5.8),
            slot_c(26.5, 19.2, 6.5, 5.8),
        ], []
    if tid == "overset":
        return [slot_c(CX, 5.5, 9.0, 5.6), slot_c(CX, 16.5, 12.0, 10.0)], []
    if tid == "underset":
        return [slot_c(CX, 9.5, 12.0, 10.0), slot_c(CX, 20.5, 9.0, 5.6)], []
    if tid == "overunderset":
        return [
            slot_c(CX, 4.6, 8.5, 4.8),
            slot_c(CX, 13.0, 11.0, 6.8),
            slot_c(CX, 21.4, 8.5, 4.8),
        ], []
    if tid.startswith("pre-"):
        _, pre, _, post = tid.split("-")
        els = [slot_c(CX, CY, 9.5, 10.5)]
        if pre in ("sup", "both"):
            els.append(slot_c(10.0, 6.8, 6.0, 5.6))
        if pre in ("sub", "both"):
            els.append(slot_c(10.0, 19.2, 6.0, 5.6))
        if post in ("sup", "both"):
            els.append(slot_c(30.0, 6.8, 6.0, 5.6))
        if post in ("sub", "both"):
            els.append(slot_c(30.0, 19.2, 6.0, 5.6))
        return els, []
    raise ValueError(tid)


def build_fences(tid: str) -> tuple[list[str], list[TextPlacement]]:
    y0, y1 = 3.5, 22.5
    simple_pairs = {
        "paren-both": ("paren", "paren"),
        "paren-left": ("paren", None),
        "paren-right": (None, "paren"),
        "bracket-both": ("bracket", "bracket"),
        "bracket-left": ("bracket", None),
        "bracket-right": (None, "bracket"),
        "brace-both": ("brace", "brace"),
        "brace-left": ("brace", None),
        "brace-right": (None, "brace"),
        "angle-both": ("angle", "angle"),
        "angle-left": ("angle", None),
        "angle-right": (None, "angle"),
        "bar-both": ("bar", "bar"),
        "bar-left": ("bar", None),
        "bar-right": (None, "bar"),
        "norm-both": ("norm", "norm"),
        "norm-left": ("norm", None),
        "norm-right": (None, "norm"),
        "floor-both": ("floor", "floor"),
        "floor-left": ("floor", None),
        "floor-right": (None, "floor"),
        "ceiling-both": ("ceiling", "ceiling"),
        "ceiling-left": ("ceiling", None),
        "ceiling-right": (None, "ceiling"),
        "bracket-paren": ("bracket", "paren"),
        "paren-bracket": ("paren", "bracket"),
        "ket": ("bar", "angle"),
        "bra": ("angle", "bar"),
        "double-bracket-both": ("double-bracket", "double-bracket"),
        "double-bracket-left": ("double-bracket", None),
        "double-bracket-right": (None, "double-bracket"),
    }
    if tid in simple_pairs:
        left_k, right_k = simple_pairs[tid]
        els = [slot_c(CX, CY, 11.5, 11.5)]
        if left_k:
            lx = 9.0 if left_k != "double-bracket" else 8.0
            els.extend(fence_glyph(left_k, lx, y0, y1, "left"))
        if right_k:
            rx = 31.0 if right_k != "double-bracket" else 32.0
            els.extend(fence_glyph(right_k, rx, y0, y1, "right"))
        return els, []
    # Reversed brackets: [ [, ] ], ] [
    if tid == "bracket-left-left":
        return fence_glyph("bracket", 9.0, y0, y1, "left") + [slot_c(CX, CY, 11.5, 11.5)] + fence_glyph("bracket", 29.0, y0, y1, "left"), []
    if tid == "bracket-right-right":
        return fence_glyph("bracket", 11.0, y0, y1, "right") + [slot_c(CX, CY, 11.5, 11.5)] + fence_glyph("bracket", 31.0, y0, y1, "right"), []
    if tid == "bracket-open-open":
        return fence_glyph("bracket", 11.0, y0, y1, "right") + [slot_c(CX, CY, 11.5, 11.5)] + fence_glyph("bracket", 29.0, y0, y1, "left"), []
    if tid == "bra-ket":
        els = fence_glyph("angle", 5.0, y0, y1, "left") + [slot_c(13.2, CY, 7.8, 10.5)]
        els += fence_glyph("bar", CX, y0, y1, "left")
        els += [slot_c(26.8, CY, 7.8, 10.5)] + fence_glyph("angle", 35.0, y0, y1, "right")
        return els, []
    if tid == "evaluation":
        els = [slot_c(14.0, CY, 11.0, 11.5)] + fence_glyph("bar", 23.0, y0, y1, "right")
        els += [slot_c(29.5, 6.8, 6.0, 5.5), slot_c(29.5, 19.2, 6.0, 5.5)]
        return els, []
    if tid == "cases-two":
        els = fence_glyph("brace", 8.5, 4.0, 22.0, "left")
        els += [slot_c(17.5, 8.2, 7.5, 6.2), slot_c(28.0, 8.2, 7.5, 6.2)]
        els += [slot_c(17.5, 17.8, 7.5, 6.2), slot_c(28.0, 17.8, 7.5, 6.2)]
        return els, []
    if tid == "cases-three":
        els = fence_glyph("brace", 8.5, 2.5, 23.5, "left")
        for r, ry in enumerate((5.8, 13.0, 20.2)):
            els += [slot_c(17.5, ry, 7.2, 4.5, sw=1.1), slot_c(28.0, ry, 7.2, 4.5, sw=1.1)]
        return els, []
    if tid in ("system-brace-two", "system-bracket-two"):
        fk = "brace" if "brace" in tid else "bracket"
        els = fence_glyph(fk, 10.0, 4.0, 22.0, "left")
        els += [slot_c(22.5, 8.2, 15.0, 6.2), slot_c(22.5, 17.8, 15.0, 6.2)]
        return els, []
    if tid in ("system-brace-three", "system-bracket-three"):
        fk = "brace" if "brace" in tid else "bracket"
        els = fence_glyph(fk, 10.0, 2.5, 23.5, "left")
        for ry in (5.8, 13.0, 20.2):
            els.append(slot_c(22.5, ry, 15.0, 4.5, sw=1.1))
        return els, []
    raise ValueError(tid)


def build_integrals_and_big_ops(group: str, tid: str) -> tuple[list[str], list[TextPlacement]]:
    d_key = reg_text("d-letter", "d", family="STIX Two Text", base_size=14.0)
    # Standalone symbols
    int_map = {
        "int": (1, ""),
        "iint": (2, ""),
        "iiint": (3, ""),
        "oint": (1, "loop"),
        "oiint": (2, "loop"),
        "oiiint": (3, "loop"),
        "varointclockwise": (1, "cw"),
        "ointctrclockwise": (1, "ccw"),
    }
    if group == "Integrals" and tid in int_map:
        cnt, lp = int_map[tid]
        return draw_integral(CX, CY, 19.0, count=cnt, loop=lp), []

    if tid == "int-differential":
        els = draw_integral(8.5, CY, 18.0) + [slot_c(16.5, CY, 7.2, 8.5), slot_c(32.0, CY, 7.0, 8.5)]
        return els, [TextPlacement(d_key, 25.0, CY + 0.5, 5.5, 11.0)]
    if tid == "definite-differential":
        els = draw_integral(8.2, CY, 12.5, sw=1.55) + [
            slot_c(8.2, 3.6, 5.2, 4.2, sw=1.05),
            slot_c(8.2, 22.4, 5.2, 4.2, sw=1.05),
            slot_c(17.2, CY, 6.8, 8.0),
            slot_c(32.5, CY, 6.6, 8.0),
        ]
        return els, [TextPlacement(d_key, 25.5, CY + 0.5, 5.5, 10.5)]

    big_ops = ("sum", "prod", "coprod", "bigcap", "bigcup", "bigwedge", "bigvee", "bigotimes", "bigoplus", "bigodot")
    if tid in big_ops:
        return draw_big_op(tid, CX, CY, 16.5), []

    # Templates: <op>-<side|stack>-<lower|both>
    parts = tid.split("-")
    if len(parts) == 3 and parts[1] in ("side", "stack") and parts[2] in ("lower", "both"):
        op_name, placement, bounds = parts
        is_int = op_name in int_map
        if placement == "side":
            op_x = 11.2 if not is_int or int_map[op_name][0] == 1 else 10.5
            op_h = 15.0 if is_int else 14.0
            op_sw = 1.55 if not (is_int and int_map[op_name][0] == 3) else 1.4
            els = (
                draw_integral(op_x, CY, op_h, sw=op_sw, count=int_map[op_name][0], loop=int_map[op_name][1])
                if is_int
                else draw_big_op(op_name, op_x, CY, op_h, sw=op_sw)
            )
            lim_x = 19.8 if not (is_int and int_map[op_name][0] >= 2) else 21.2
            els.append(slot_c(lim_x, 18.8, 5.6, 5.0, sw=1.05))
            if bounds == "both":
                els.append(slot_c(lim_x, 7.2, 5.6, 5.0, sw=1.05))
            els.append(slot_c(30.2, CY, 8.6, 9.5))
            return els, []
        else:  # stack
            op_x = 12.2
            op_h = 10.5 if bounds == "both" else 13.0
            op_cy = CY if bounds == "both" else 10.2
            op_sw = 1.5 if bounds == "both" else 1.55
            els = (
                draw_integral(op_x, op_cy, op_h, sw=op_sw, count=int_map[op_name][0], loop=int_map[op_name][1])
                if is_int
                else draw_big_op(op_name, op_x, op_cy, op_h, sw=op_sw)
            )
            els.append(slot_c(op_x, 21.8 if bounds == "both" else 20.5, 6.2, 4.4, sw=1.05))
            if bounds == "both":
                els.append(slot_c(op_x, 4.2, 6.2, 4.4, sw=1.05))
            els.append(slot_c(28.5, CY, 10.0, 10.5))
            return els, []
    raise ValueError((group, tid))


def build_matrices(tid: str) -> tuple[list[str], list[TextPlacement]]:
    def grid(rows: int, cols: int, fence: str | None = None) -> list[str]:
        els: list[str] = []
        # Full hollow square/rect cells up to 4x4 within 40x26
        cell_h = {1: 8.5, 2: 6.5, 3: 4.5, 4: 3.5}[rows]
        gap_y = {1: 0.0, 2: 4.0, 3: 3.2, 4: 2.5}[rows]
        cell_w = {1: 8.5, 2: 6.5, 3: 4.5, 4: 4.0}[cols]
        gap_x = {1: 0.0, 2: 4.5, 3: 3.5, 4: 3.2}[cols]
        if fence and cols == 3:
            cell_w, gap_x = 4.2, 2.8
        sw = {1: 1.2, 2: 1.2, 3: 1.1, 4: 1.0}[max(rows, cols)]
        rx = 1.1 if max(rows, cols) <= 2 else 0.75
        total_w = cols * cell_w + (cols - 1) * gap_x
        total_h = rows * cell_h + (rows - 1) * gap_y
        x0 = CX - total_w / 2.0
        y0 = CY - total_h / 2.0
        for r in range(rows):
            for c in range(cols):
                els.append(
                    slot(
                        x0 + c * (cell_w + gap_x),
                        y0 + r * (cell_h + gap_y),
                        cell_w,
                        cell_h,
                        sw=sw,
                        rx=rx,
                    )
                )
        if fence:
            pad_x = 5.5 if fence == "paren" else (5.2 if fence == "norm" else 4.5)
            fy0, fy1 = max(2.2, y0 - 2.0), min(23.8, y0 + total_h + 2.0)
            els.extend(fence_glyph(fence, x0 - pad_x, fy0, fy1, "left", sw=1.55))
            els.extend(fence_glyph(fence, x0 + total_w + pad_x, fy0, fy1, "right", sw=1.55))
        return els

    plain_dims = {
        "plain-1x2": (1, 2),
        "plain-2x1": (2, 1),
        "plain-1x3": (1, 3),
        "plain-3x1": (3, 1),
        "plain-2x2": (2, 2),
        "plain-3x3": (3, 3),
        "plain-2x3": (2, 3),
        "plain-3x2": (3, 2),
        "plain-1x4": (1, 4),
        "plain-4x1": (4, 1),
        "plain-4x4": (4, 4),
    }
    if tid in plain_dims:
        r, c = plain_dims[tid]
        return grid(r, c), []
    fenced_map = {
        "pmatrix2": (2, 2, "paren"),
        "pmatrix3": (3, 3, "paren"),
        "bmatrix2": (2, 2, "bracket"),
        "bmatrix3": (3, 3, "bracket"),
        "Bmatrix2": (2, 2, "brace"),
        "Bmatrix3": (3, 3, "brace"),
        "vmatrix2": (2, 2, "bar"),
        "vmatrix3": (3, 3, "bar"),
        "Vmatrix2": (2, 2, "norm"),
        "Vmatrix3": (3, 3, "norm"),
    }
    if tid in fenced_map:
        r, c, fk = fenced_map[tid]
        return grid(r, c, fk), []
    if tid == "dots-row":
        return [
            slot_c(9.0, CY, 6.5, 6.5),
            svg_dot(17.0, CY, 0.85),
            svg_dot(20.0, CY, 0.85),
            svg_dot(23.0, CY, 0.85),
            slot_c(31.0, CY, 6.5, 6.5),
        ], []
    if tid == "dots-column":
        return [
            slot_c(CX, 5.2, 6.5, 5.5),
            svg_dot(CX, 10.6, 0.85),
            svg_dot(CX, 13.0, 0.85),
            svg_dot(CX, 15.4, 0.85),
            slot_c(CX, 20.8, 6.5, 5.5),
        ], []
    if tid == "dots-matrix":
        els = [
            slot_c(10.0, 5.5, 5.0, 4.8, sw=1.1, rx=0.8),
            slot_c(30.0, 5.5, 5.0, 4.8, sw=1.1, rx=0.8),
            slot_c(10.0, 20.5, 5.0, 4.8, sw=1.1, rx=0.8),
            slot_c(30.0, 20.5, 5.0, 4.8, sw=1.1, rx=0.8),
        ]
        for x in (17.5, 20.0, 22.5):
            els += [svg_dot(x, 5.5, 0.75), svg_dot(x, 20.5, 0.75)]
        for y in (10.5, 13.0, 15.5):
            els += [svg_dot(10.0, y, 0.75), svg_dot(30.0, y, 0.75)]
        for k in (-2.5, 0.0, 2.5):
            els.append(svg_dot(CX + k, CY + k, 0.75))
        return els, []
    if tid == "aligned":
        els = []
        for ry in (7.8, 18.2):
            els.append(slot_c(11.0, ry, 7.5, 6.2))
            els.append(svg_path(f"M17.5 {fmt(ry - 1.3)} H22.5 M17.5 {fmt(ry + 1.3)} H22.5", sw=1.35))
            els.append(slot_c(29.0, ry, 7.5, 6.2))
        return els, []
    if tid == "gathered":
        return [slot_c(CX, 7.8, 13.0, 6.4), slot_c(CX, 18.2, 13.0, 6.4)], []
    raise ValueError(tid)


def build_boxes(tid: str) -> tuple[list[str], list[TextPlacement]]:
    x0, y0, x1, y1 = 10.5, 4.5, 29.5, 21.5
    els = [slot_c(CX, CY, 11.5, 10.0)]
    if tid == "top-left":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y1)} V{fmt(y0)} H{fmt(x1)}"))
    elif tid == "top-right":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y0)} H{fmt(x1)} V{fmt(y1)}"))
    elif tid == "bottom-left":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y0)} V{fmt(y1)} H{fmt(x1)}"))
    elif tid == "bottom-right":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y1)} H{fmt(x1)} V{fmt(y0)}"))
    elif tid == "box":
        els.append(f'<rect x="{fmt(x0)}" y="{fmt(y0)}" width="{fmt(x1 - x0)}" height="{fmt(y1 - y0)}" />')
    elif tid == "roundedbox":
        els.append(f'<rect x="{fmt(x0)}" y="{fmt(y0)}" width="{fmt(x1 - x0)}" height="{fmt(y1 - y0)}" rx="3.5" />')
    elif tid == "top-bottom-left":
        els.append(svg_path(f"M{fmt(x1)} {fmt(y0)} H{fmt(x0)} V{fmt(y1)} H{fmt(x1)}"))
    elif tid == "top-bottom-right":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y0)} H{fmt(x1)} V{fmt(y1)} H{fmt(x0)}"))
    elif tid == "top-left-right":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y1)} V{fmt(y0)} H{fmt(x1)} V{fmt(y1)}"))
    elif tid == "bottom-left-right":
        els.append(svg_path(f"M{fmt(x0)} {fmt(y0)} V{fmt(y1)} H{fmt(x1)} V{fmt(y0)}"))
    else:
        raise ValueError(tid)
    return els, []


def build_embellishments(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid in ("dot", "ddot", "dddot", "ddddot"):
        n = {"dot": 1, "ddot": 2, "dddot": 3, "ddddot": 4}[tid]
        els = [slot_c(CX, 15.2, 12.0, 10.5)]
        sp = 3.2 if n <= 3 else 2.7
        for i in range(n):
            els.append(svg_dot(CX + (i - (n - 1) / 2.0) * sp, 6.2, 1.05 if n <= 3 else 0.9))
        return els, []
    if tid in ("underdot", "underddot"):
        n = 1 if tid == "underdot" else 2
        els = [slot_c(CX, 10.8, 12.0, 10.5)]
        sp = 3.4
        for i in range(n):
            els.append(svg_dot(CX + (i - (n - 1) / 2.0) * sp, 19.8, 1.05))
        return els, []
    if tid == "hat":
        return [slot_c(CX, 15.2, 12.0, 10.5), svg_path("M14.5 7.2 L20 4.0 L25.5 7.2")], []
    if tid == "check":
        return [slot_c(CX, 15.2, 12.0, 10.5), svg_path("M14.5 4.5 L20 7.5 L25.5 4.5")], []
    if tid == "tilde":
        return [slot_c(CX, 15.2, 12.0, 10.5), svg_path("M13.5 6.4 Q16.8 3.6 20 6.0 Q23.2 8.4 26.5 5.6")], []
    if tid == "undertilde":
        return [slot_c(CX, 10.8, 12.0, 10.5), svg_path("M13.5 20.2 Q16.8 17.6 20 19.8 Q23.2 22.0 26.5 19.4")], []
    if tid == "breve":
        return [slot_c(CX, 15.2, 12.0, 10.5), svg_path("M15.0 4.6 Q20 8.5 25.0 4.6")], []
    if tid == "mathring":
        return [slot_c(CX, 15.5, 12.0, 10.2), svg_circle(CX, 5.5, 1.8, sw=1.45)], []
    if tid == "vec":
        return [slot_c(CX, 15.2, 12.0, 10.5)] + draw_arrow_shape("rightarrow", CX, 5.8, 13.0, sw=1.5), []
    if tid == "overleftharpoon":
        return [slot_c(CX, 15.2, 12.0, 10.5)] + draw_arrow_shape("leftharpoonup", CX, 6.2, 13.0, sw=1.5), []
    if tid == "overrightharpoon":
        return [slot_c(CX, 15.2, 12.0, 10.5)] + draw_arrow_shape("rightharpoonup", CX, 6.2, 13.0, sw=1.5), []
    if tid == "underleftharpoon":
        return [slot_c(CX, 10.8, 12.0, 10.5)] + draw_arrow_shape("leftharpoondown", CX, 19.5, 13.0, sw=1.5), []
    if tid == "underrightharpoon":
        return [slot_c(CX, 10.8, 12.0, 10.5)] + draw_arrow_shape("rightharpoondown", CX, 19.5, 13.0, sw=1.5), []
    if tid == "cancel":
        return [slot_c(CX, CY, 12.5, 11.5), svg_path("M11.5 20.5 L28.5 5.5")], []
    if tid == "bcancel":
        return [slot_c(CX, CY, 12.5, 11.5), svg_path("M11.5 5.5 L28.5 20.5")], []
    if tid == "xcancel":
        return [slot_c(CX, CY, 12.5, 11.5), svg_path("M11.5 20.5 L28.5 5.5 M11.5 5.5 L28.5 20.5")], []
    if tid == "strike-horizontal":
        return [slot_c(CX, CY, 12.5, 11.5), svg_path("M10.5 13 H29.5")], []
    if tid in ("prime", "double-prime", "triple-prime"):
        n = {"prime": 1, "double-prime": 2, "triple-prime": 3}[tid]
        els = [slot_c(16.0, 14.5, 10.5, 10.5)]
        for i in range(n):
            px = 24.5 + i * 3.0
            els.append(svg_path(f"M{fmt(px + 1.2)} 4.5 L{fmt(px)} 10.2", sw=1.55))
        return els, []
    if tid == "left-prime":
        return [svg_path("M13.2 4.5 L12.0 10.2", sw=1.55), slot_c(21.5, 14.5, 10.5, 10.5)], []
    if tid == "back-prime":
        return [slot_c(16.5, 14.5, 10.5, 10.5), svg_path("M24.2 4.5 L25.6 10.2", sw=1.55)], []
    raise ValueError(tid)


def build_bars(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "overline":
        return [svg_path("M11.5 6.0 H28.5"), slot_c(CX, 15.0, 12.5, 10.5)], []
    if tid == "underline":
        return [slot_c(CX, 11.0, 12.5, 10.5), svg_path("M11.5 20.0 H28.5")], []
    if tid == "doubleoverline":
        return [svg_path("M11.5 4.2 H28.5 M11.5 7.4 H28.5", sw=1.45), slot_c(CX, 16.0, 12.5, 10.0)], []
    if tid == "doubleunderline":
        return [slot_c(CX, 10.0, 12.5, 10.0), svg_path("M11.5 18.6 H28.5 M11.5 21.8 H28.5", sw=1.45)], []
    if tid == "widehat":
        return [svg_path("M11.5 7.5 L20 3.8 L28.5 7.5"), slot_c(CX, 15.5, 12.5, 10.2)], []
    if tid == "widetilde":
        return [svg_path("M11.5 6.5 Q15.7 3.2 20 5.8 Q24.3 8.4 28.5 5.1"), slot_c(CX, 15.5, 12.5, 10.2)], []
    if tid == "wideparen":
        return [svg_path("M11.5 7.8 Q20 2.5 28.5 7.8"), slot_c(CX, 15.5, 12.5, 10.2)], []
    if tid == "underparen":
        return [slot_c(CX, 10.5, 12.5, 10.2), svg_path("M11.5 18.2 Q20 23.5 28.5 18.2")], []
    if tid == "overleftarrow":
        return draw_arrow_shape("leftarrow", CX, 5.8, 17.0, sw=1.5) + [slot_c(CX, 15.2, 12.5, 10.2)], []
    if tid == "overrightarrow":
        return draw_arrow_shape("rightarrow", CX, 5.8, 17.0, sw=1.5) + [slot_c(CX, 15.2, 12.5, 10.2)], []
    if tid == "overleftrightarrow":
        return draw_arrow_shape("leftrightarrow", CX, 5.8, 17.0, sw=1.5) + [slot_c(CX, 15.2, 12.5, 10.2)], []
    if tid == "underleftarrow":
        return [slot_c(CX, 10.8, 12.5, 10.2)] + draw_arrow_shape("leftarrow", CX, 20.2, 17.0, sw=1.5), []
    if tid == "underrightarrow":
        return [slot_c(CX, 10.8, 12.5, 10.2)] + draw_arrow_shape("rightarrow", CX, 20.2, 17.0, sw=1.5), []
    if tid == "underleftrightarrow":
        return [slot_c(CX, 10.8, 12.5, 10.2)] + draw_arrow_shape("leftrightarrow", CX, 20.2, 17.0, sw=1.5), []
    if tid in ("overbrace", "overbrace-label"):
        labeled = tid.endswith("-label")
        by = 9.8 if labeled else 7.4
        els = [
            svg_path(
                f"M10.5 {fmt(by + 1.4)} Q10.5 {fmt(by)} 13.0 {fmt(by)} H18.2 Q20 {fmt(by)} 20 {fmt(by - 2.0)} "
                f"Q20 {fmt(by)} 21.8 {fmt(by)} H27.0 Q29.5 {fmt(by)} 29.5 {fmt(by + 1.4)}",
                sw=1.5,
            ),
            slot_c(CX, 17.5 if labeled else 15.8, 12.5, 8.8 if labeled else 10.0),
        ]
        if labeled:
            els.append(slot_c(CX, 4.2, 8.5, 4.4, sw=1.05))
        return els, []
    if tid in ("underbrace", "underbrace-label"):
        labeled = tid.endswith("-label")
        by = 16.2 if labeled else 18.6
        els = [
            slot_c(CX, 8.5 if labeled else 10.2, 12.5, 8.8 if labeled else 10.0),
            svg_path(
                f"M10.5 {fmt(by - 1.4)} Q10.5 {fmt(by)} 13.0 {fmt(by)} H18.2 Q20 {fmt(by)} 20 {fmt(by + 2.0)} "
                f"Q20 {fmt(by)} 21.8 {fmt(by)} H27.0 Q29.5 {fmt(by)} 29.5 {fmt(by - 1.4)}",
                sw=1.5,
            ),
        ]
        if labeled:
            els.append(slot_c(CX, 21.8, 8.5, 4.4, sw=1.05))
        return els, []
    if tid in ("overbracket", "overbracket-label"):
        labeled = tid.endswith("-label")
        by = 9.0 if labeled else 6.2
        els = [
            svg_path(f"M11 {fmt(by + 2.5)} V{fmt(by)} H29 V{fmt(by + 2.5)}", sw=1.55),
            slot_c(CX, 17.5 if labeled else 15.5, 12.5, 9.0 if labeled else 10.2),
        ]
        if labeled:
            els.append(slot_c(CX, 4.5, 8.5, 4.4, sw=1.05))
        return els, []
    if tid in ("underbracket", "underbracket-label"):
        labeled = tid.endswith("-label")
        by = 17.0 if labeled else 19.8
        els = [
            slot_c(CX, 8.5 if labeled else 10.5, 12.5, 9.0 if labeled else 10.2),
            svg_path(f"M11 {fmt(by - 2.5)} V{fmt(by)} H29 V{fmt(by - 2.5)}", sw=1.55),
        ]
        if labeled:
            els.append(slot_c(CX, 21.5, 8.5, 4.4, sw=1.05))
        return els, []
    if tid == "overline-label":
        return [slot_c(CX, 5.0, 9.0, 4.8, sw=1.05), svg_path("M11 10.2 H29", sw=1.55), slot_c(CX, 17.5, 12.0, 9.0)], []
    if tid == "underline-label":
        return [slot_c(CX, 8.5, 12.0, 9.0), svg_path("M11 15.8 H29", sw=1.55), slot_c(CX, 21.0, 9.0, 4.8, sw=1.05)], []
    if tid == "doubleoverline-label":
        return [
            slot_c(CX, 4.2, 8.5, 4.2, sw=1.05),
            svg_path("M11 8.6 H29 M11 11.4 H29", sw=1.35),
            slot_c(CX, 18.2, 12.0, 8.2),
        ], []
    if tid == "doubleunderline-label":
        return [
            slot_c(CX, 7.8, 12.0, 8.2),
            svg_path("M11 14.6 H29 M11 17.4 H29", sw=1.35),
            slot_c(CX, 21.8, 8.5, 4.2, sw=1.05),
        ], []
    raise ValueError(tid)


def build_arrows(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid in ("rightarrow", "leftarrow", "leftrightarrow", "Rightarrow", "Leftarrow", "Leftrightarrow",
               "rightharpoonup", "rightharpoondown", "leftharpoonup", "leftharpoondown",
               "rightleftharpoons", "leftrightharpoons", "mapsto", "rightleftarrows", "leftrightarrows"):
        return draw_arrow_shape(tid, CX, CY, 19.0), []
    long_map = {
        "longrightarrow": "rightarrow",
        "longleftarrow": "leftarrow",
        "longleftrightarrow": "leftrightarrow",
        "Longrightarrow": "Rightarrow",
        "Longleftarrow": "Leftarrow",
        "Longleftrightarrow": "Leftrightarrow",
    }
    if tid in long_map:
        return draw_arrow_shape(long_map[tid], CX, CY, 29.0), []
    if tid == "uparrow":
        return [svg_path("M20 21 V5"), arrow_head(20, 5, -90)], []
    if tid == "downarrow":
        return [svg_path("M20 5 V21"), arrow_head(20, 21, 90)], []
    if tid == "updownarrow":
        return [svg_path("M20 4.5 V21.5"), arrow_head(20, 4.5, -90), arrow_head(20, 21.5, 90)], []
    if tid == "Uparrow":
        return [svg_path("M18.4 21 V6.2", sw=1.4), svg_path("M21.6 21 V6.2", sw=1.4), arrow_head_sw(20, 4.5, -90, 1.5, 5.4, 45)], []
    if tid == "Downarrow":
        return [svg_path("M18.4 5 V19.8", sw=1.4), svg_path("M21.6 5 V19.8", sw=1.4), arrow_head_sw(20, 21.5, 90, 1.5, 5.4, 45)], []
    if tid == "Updownarrow":
        return [
            svg_path("M18.4 6.2 V19.8", sw=1.4),
            svg_path("M21.6 6.2 V19.8", sw=1.4),
            arrow_head_sw(20, 4.5, -90, 1.5, 5.4, 45),
            arrow_head_sw(20, 21.5, 90, 1.5, 5.4, 45),
        ], []
    if tid == "nearrow":
        return [svg_path("M13.5 19.5 L26.5 6.5"), arrow_head(26.5, 6.5, -45)], []
    if tid == "searrow":
        return [svg_path("M13.5 6.5 L26.5 19.5"), arrow_head(26.5, 19.5, 45)], []
    if tid == "swarrow":
        return [svg_path("M26.5 6.5 L13.5 19.5"), arrow_head(13.5, 19.5, 135)], []
    if tid == "nwarrow":
        return [svg_path("M26.5 19.5 L13.5 6.5"), arrow_head(13.5, 6.5, -135)], []
    if tid == "neswarrow":
        return [svg_path("M13.5 19.5 L26.5 6.5"), arrow_head(26.5, 6.5, -45), arrow_head(13.5, 19.5, 135)], []
    if tid == "nwsearrow":
        return [svg_path("M13.5 6.5 L26.5 19.5"), arrow_head(13.5, 6.5, -135), arrow_head(26.5, 19.5, 45)], []
    if tid == "hookleftarrow":
        return [
            svg_path("M11 14.5 H24.5 A3.5 3.5 0 0 0 24.5 7.5 H22.5"),
            arrow_head(11, 14.5, 180),
        ], []
    if tid == "hookrightarrow":
        return [
            svg_path("M29 14.5 H15.5 A3.5 3.5 0 0 1 15.5 7.5 H17.5"),
            arrow_head(29, 14.5, 0),
        ], []
    if tid == "curvearrowleft":
        return [svg_path("M28 16.5 A8.5 8.5 0 0 0 12 16.5"), arrow_head(12, 16.5, 135)], []
    if tid == "curvearrowright":
        return [svg_path("M12 16.5 A8.5 8.5 0 0 1 28 16.5"), arrow_head(28, 16.5, 45)], []
    raise ValueError(tid)


def build_labeled_arrows(tid: str) -> tuple[list[str], list[TextPlacement]]:
    base, pos = tid.rsplit("-", 1)
    if pos == "top":
        return [slot_c(CX, 6.8, 10.0, 6.2)] + draw_arrow_shape(base, CX, 17.5, 24.0, sw=1.55), []
    if pos == "bottom":
        return draw_arrow_shape(base, CX, 8.5, 24.0, sw=1.55) + [slot_c(CX, 19.2, 10.0, 6.2)], []
    if pos == "both":
        return (
            [slot_c(CX, 4.6, 9.5, 4.8, sw=1.05)]
            + draw_arrow_shape(base, CX, CY, 24.0, sw=1.45)
            + [slot_c(CX, 21.4, 9.5, 4.8, sw=1.05)]
        ), []
    raise ValueError(tid)


def build_operators(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "pm":
        return [svg_path("M14.5 11 H25.5 M20 5.5 V16.5 M14.5 20 H25.5")], []
    if tid == "mp":
        return [svg_path("M14.5 6 H25.5 M14.5 15 H25.5 M20 9.5 V20.5")], []
    if tid == "times":
        return [svg_path("M14.5 7.5 L25.5 18.5 M25.5 7.5 L14.5 18.5")], []
    if tid == "ast":
        els = [svg_path("M20 7.2 V18.8", sw=1.5)]
        for ang in (30, -30):
            dx = 5.4 * math.cos(math.radians(ang))
            dy = 5.4 * math.sin(math.radians(ang))
            els.append(svg_path(f"M{fmt(CX - dx)} {fmt(CY - dy)} L{fmt(CX + dx)} {fmt(CY + dy)}", sw=1.5))
        return els, []
    if tid == "div":
        return [svg_path("M13.5 13 H26.5"), svg_dot(CX, 7.8, 1.15), svg_dot(CX, 18.2, 1.15)], []
    if tid == "oplus":
        return [svg_circle(CX, CY, 6.8), svg_path("M15.2 13 H24.8 M20 8.2 V17.8", sw=1.45)], []
    if tid == "otimes":
        return [svg_circle(CX, CY, 6.8), svg_path("M15.4 8.4 L24.6 17.6 M24.6 8.4 L15.4 17.6", sw=1.45)], []
    if tid == "odot":
        return [svg_circle(CX, CY, 6.8), svg_dot(CX, CY, 1.35)], []
    if tid == "cdot":
        return [svg_dot(CX, CY, 1.45)], []
    if tid == "bullet":
        return [svg_dot(CX, CY, 3.1)], []
    if tid == "circ":
        return [svg_circle(CX, CY, 3.2)], []
    if tid == "diamond":
        return [svg_path("M20 8 L24.8 13 L20 18 L15.2 13 Z")], []
    if tid == "dagger":
        return [svg_path("M20 5 V21 M15.5 9.5 H24.5")], []
    if tid == "ddagger":
        return [svg_path("M20 4.8 V21.2 M15.5 9.2 H24.5 M15.5 16.8 H24.5")], []
    if tid == "amalg":
        return [svg_path("M14 19 H26 M16.2 7 V19 M23.8 7 V19")], []
    raise ValueError(tid)


def build_relations(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "leq":
        return [svg_path("M25.5 6 L14.5 10.5 L25.5 15 M14.5 19.2 H25.5")], []
    if tid == "geq":
        return [svg_path("M14.5 6 L25.5 10.5 L14.5 15 M14.5 19.2 H25.5")], []
    if tid == "nleq":
        return [svg_path("M25.5 6 L14.5 10.5 L25.5 15 M14.5 19.2 H25.5 M15.5 21 L24.5 5")], []
    if tid == "ngeq":
        return [svg_path("M14.5 6 L25.5 10.5 L14.5 15 M14.5 19.2 H25.5 M15.5 21 L24.5 5")], []
    if tid == "ll":
        return [svg_path("M19 7.5 L12.5 13 L19 18.5 M27 7.5 L20.5 13 L27 18.5")], []
    if tid == "gg":
        return [svg_path("M13 7.5 L19.5 13 L13 18.5 M21 7.5 L27.5 13 L21 18.5")], []
    if tid == "prec":
        return [svg_path("M25.5 7 Q19.5 12.2 14.5 13 Q19.5 13.8 25.5 19")], []
    if tid == "succ":
        return [svg_path("M14.5 7 Q20.5 12.2 25.5 13 Q20.5 13.8 14.5 19")], []
    if tid == "preceq":
        return [svg_path("M25.5 6 Q19.5 10 14.5 10.8 Q19.5 11.6 25.5 15.5 M14.5 19.5 H25.5")], []
    if tid == "succeq":
        return [svg_path("M14.5 6 Q20.5 10 25.5 10.8 Q20.5 11.6 14.5 15.5 M14.5 19.5 H25.5")], []
    if tid == "triangleleft":
        return [svg_path("M14 13 L25 7 V19 Z")], []
    if tid == "triangleright":
        return [svg_path("M26 13 L15 7 V19 Z")], []
    if tid == "sim":
        return [svg_path("M13 13.2 Q16.5 9.2 20 13 Q23.5 16.8 27 12.8")], []
    if tid == "approx":
        return [svg_path("M13 10.6 Q16.5 7.2 20 10.4 Q23.5 13.6 27 10.2 M13 16.2 Q16.5 12.8 20 16.0 Q23.5 19.2 27 15.8")], []
    if tid == "simeq":
        return [svg_path("M13 11.2 Q16.5 7.6 20 11.0 Q23.5 14.4 27 10.8 M13.5 17.5 H26.5")], []
    if tid == "cong":
        return [svg_path("M13 9.2 Q16.5 6.0 20 9.0 Q23.5 12.0 27 8.8 M13.5 14.5 H26.5 M13.5 18.5 H26.5")], []
    if tid == "neq":
        return [svg_path("M13 10.2 H27 M13 15.8 H27 M15.5 20.5 L24.5 5.5")], []
    if tid == "equiv":
        return [svg_path("M13 8.5 H27 M13 13 H27 M13 17.5 H27")], []
    if tid == "doteq":
        return [svg_dot(CX, 7.5, 1.15), svg_path("M13.5 12.2 H26.5 M13.5 16.8 H26.5")], []
    if tid == "triangleq":
        return [svg_path("M17 10 L20 5 L23 10 Z", sw=1.4), svg_path("M13.5 14.5 H26.5 M13.5 19 H26.5")], []
    if tid == "estimates":
        return [svg_path("M16.5 9.8 L20 5.8 L23.5 9.8", sw=1.5), svg_path("M13.5 14.5 H26.5 M13.5 19 H26.5")], []
    if tid == "fallingdotseq":
        return [svg_dot(16.2, 7.2, 1.05), svg_dot(23.8, 18.8, 1.05), svg_path("M13.5 11.2 H26.5 M13.5 15.0 H26.5")], []
    if tid == "risingdotseq":
        return [svg_dot(23.8, 7.2, 1.05), svg_dot(16.2, 18.8, 1.05), svg_path("M13.5 11.2 H26.5 M13.5 15.0 H26.5")], []
    if tid == "asymp":
        return [svg_path("M13.5 7.5 Q20 12.5 26.5 7.5 M13.5 18.5 Q20 13.5 26.5 18.5")], []
    if tid == "propto":
        return [svg_path("M27 9.5 C24 9.5 22 13 19.8 14.8 C17.8 16.5 15.2 16.8 13.8 15.2 C12.4 13.6 12.4 12.4 13.8 10.8 C15.2 9.2 17.8 9.5 19.8 11.2 C22 13 24 16.5 27 16.5")], []
    if tid == "parallel":
        return [svg_path("M17.2 5.5 V20.5 M22.8 5.5 V20.5")], []
    if tid == "perp":
        return [svg_path("M13 19.5 H27 M20 6 V19.5")], []
    raise ValueError(tid)


def build_sets(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "in":
        return [svg_path("M25 7.5 H19.5 A5.5 5.5 0 0 0 19.5 18.5 H25 M14.2 13 H24.5")], []
    if tid == "notin":
        return [svg_path("M25 7.5 H19.5 A5.5 5.5 0 0 0 19.5 18.5 H25 M14.2 13 H24.5 M15.5 20.5 L24.5 5.5")], []
    if tid == "ni":
        return [svg_path("M15 7.5 H20.5 A5.5 5.5 0 0 1 20.5 18.5 H15 M15.5 13 H25.8")], []
    if tid == "notni":
        return [svg_path("M15 7.5 H20.5 A5.5 5.5 0 0 1 20.5 18.5 H15 M15.5 13 H25.8 M15.5 20.5 L24.5 5.5")], []
    if tid == "cup":
        return [svg_path("M14 7 V13.5 A6 6 0 0 0 26 13.5 V7")], []
    if tid == "cap":
        return [svg_path("M14 19 V12.5 A6 6 0 0 1 26 12.5 V19")], []
    if tid == "subset":
        return [svg_path("M25.5 7.5 H19.5 A5.5 5.5 0 0 0 19.5 18.5 H25.5")], []
    if tid == "supset":
        return [svg_path("M14.5 7.5 H20.5 A5.5 5.5 0 0 1 20.5 18.5 H14.5")], []
    if tid == "nsubset":
        return [svg_path("M25.5 7.5 H19.5 A5.5 5.5 0 0 0 19.5 18.5 H25.5 M15.5 20.5 L24.5 5.5")], []
    if tid == "subseteq":
        return [svg_path("M25.5 6.2 H19.5 A4.8 4.8 0 0 0 19.5 15.8 H25.5 M14.5 19.8 H25.5")], []
    if tid == "supseteq":
        return [svg_path("M14.5 6.2 H20.5 A4.8 4.8 0 0 1 20.5 15.8 H14.5 M14.5 19.8 H25.5")], []
    if tid == "nsubseteq":
        return [svg_path("M25.5 6.2 H19.5 A4.8 4.8 0 0 0 19.5 15.8 H25.5 M14.5 19.8 H25.5 M15.5 21.5 L24.5 4.8")], []
    if tid == "nsupseteq":
        return [svg_path("M14.5 6.2 H20.5 A4.8 4.8 0 0 1 20.5 15.8 H14.5 M14.5 19.8 H25.5 M15.5 21.5 L24.5 4.8")], []
    if tid == "emptyset":
        return [svg_circle(CX, CY, 6.2), svg_path("M14 20.5 L26 5.5")], []
    if tid == "setminus":
        return [svg_path("M15.5 6 L24.5 20")], []
    if tid == "subsetneq":
        return [svg_path("M25.5 6.2 H19.5 A4.8 4.8 0 0 0 19.5 15.8 H25.5 M14.5 19.8 H25.5 M17.5 22 L22.5 17.6")], []
    if tid == "supsetneq":
        return [svg_path("M14.5 6.2 H20.5 A4.8 4.8 0 0 1 20.5 15.8 H14.5 M14.5 19.8 H25.5 M17.5 22 L22.5 17.6")], []
    raise ValueError(tid)


def build_logic(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "therefore":
        return [svg_dot(CX, 8.2, 1.25), svg_dot(15.2, 17.2, 1.25), svg_dot(24.8, 17.2, 1.25)], []
    if tid == "because":
        return [svg_dot(15.2, 8.8, 1.25), svg_dot(24.8, 8.8, 1.25), svg_dot(CX, 17.8, 1.25)], []
    if tid == "exists":
        return [svg_path("M15 6.5 H25 V19.5 H15 M16 13 H25")], []
    if tid == "backepsilon":
        return [svg_path("M15.5 6.8 H22 A5.2 5.2 0 0 1 22 19.2 H15.5 M15.5 13 H25")], []
    if tid == "nexists":
        return [svg_path("M15 6.5 H25 V19.5 H15 M16 13 H25 M15.5 21 L24.5 5")], []
    if tid == "forall":
        return [svg_path("M14 6 L20 20 L26 6 M16.4 11.8 H23.6")], []
    if tid == "neg":
        return [svg_path("M13.5 10.5 H26.5 V16")], []
    if tid == "land":
        return [svg_path("M14 19 L20 7 L26 19")], []
    if tid == "lor":
        return [svg_path("M14 7 L20 19 L26 7")], []
    if tid == "top":
        return [svg_path("M13 6.5 H27 M20 6.5 V20")], []
    if tid == "bot":
        return [svg_path("M13 19.5 H27 M20 6 V19.5")], []
    if tid == "vdash":
        return [svg_path("M15 6 V20 M15 13 H26")], []
    if tid == "dashv":
        return [svg_path("M25 6 V20 M14 13 H25")], []
    if tid == "models":
        return [svg_path("M15 6 V20 M15 10.5 H26 M15 15.5 H26")], []
    raise ValueError(tid)


def build_spaces(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "ldots":
        return [svg_dot(14.5, 17.5, 1.15), svg_dot(CX, 17.5, 1.15), svg_dot(25.5, 17.5, 1.15)], []
    if tid == "cdots":
        return [svg_dot(14.5, CY, 1.15), svg_dot(CX, CY, 1.15), svg_dot(25.5, CY, 1.15)], []
    if tid == "vdots":
        return [svg_dot(CX, 7.5, 1.15), svg_dot(CX, CY, 1.15), svg_dot(CX, 18.5, 1.15)], []
    if tid == "ddots":
        return [svg_dot(14.5, 7.5, 1.15), svg_dot(CX, CY, 1.15), svg_dot(25.5, 18.5, 1.15)], []
    if tid == "iddots":
        return [svg_dot(14.5, 18.5, 1.15), svg_dot(CX, CY, 1.15), svg_dot(25.5, 7.5, 1.15)], []
    widths = {
        "space-0.083333": 4.0,
        "space-0.166667": 6.5,
        "space-0.222222": 9.5,
        "space-0.277778": 13.0,
        "space-1": 18.0,
        "space-2": 26.0,
    }
    if tid in widths:
        half = widths[tid] / 2.0
        els = [
            svg_path(f"M{fmt(CX - half)} 6.5 V19.5", sw=1.55),
            svg_path(f"M{fmt(CX + half)} 6.5 V19.5", sw=1.55),
        ]
        if widths[tid] >= 9.0:
            els.append(
                svg_path(
                    f"M{fmt(CX - half + 2.2)} 19.5 H{fmt(CX + half - 2.2)}",
                    sw=1.1,
                    opacity=0.55,
                )
            )
        return els, []
    raise ValueError(tid)


def build_functions(tid: str) -> tuple[list[str], list[TextPlacement]]:
    if tid == "lim":
        key = reg_text("fn-lim", "lim", family="Nunito", base_size=11.0)
        els = [slot_c(12.2, 19.8, 5.8, 4.8, sw=1.05)] + draw_arrow_shape("rightarrow", 20.0, 19.8, 5.4, sw=1.2) + [slot_c(27.8, 19.8, 5.8, 4.8, sw=1.05)]
        return els, [TextPlacement(key, CX, 8.2, 24.0, 10.5, align="baseline", baseline_y=12.4)]
    if tid in ("min", "max"):
        key = reg_text(f"fn-{tid}", tid, family="Nunito", base_size=11.0)
        return [slot_c(CX, 19.8, 7.0, 4.8, sw=1.05)], [TextPlacement(key, CX, 8.2, 26.0, 10.5, align="baseline", baseline_y=12.4)]
    key = reg_text(f"fn-{tid}", tid, family="Nunito", base_size=11.5)
    return [], [TextPlacement(key, CX, CY, 34.5, 14.0, align="baseline", baseline_y=16.0)]


def build_greek(group: str, tid: str, glyph: str) -> tuple[list[str], list[TextPlacement]]:
    key = reg_text(f"{group}-{tid}", glyph, family="STIX Two Math", base_size=17.0)
    # Keep lowercase vs uppercase typographic proportions by aligning baseline for Greek
    return [], [TextPlacement(key, CX, CY, 17.5, 16.5)]


def build_symbols(tid: str, glyph: str | None) -> tuple[list[str], list[TextPlacement]]:
    geom: dict[str, Callable[[], list[str]]] = {
        "infty": lambda: [svg_path("M20 13 C18 9.2 11 9.2 11 13 C11 16.8 18 16.8 20 13 C22 9.2 29 9.2 29 13 C29 16.8 22 16.8 20 13 Z")],
        "nabla": lambda: [svg_path("M13 7 H27 L20 20 Z")],
        "triangle": lambda: [svg_path("M20 6 L27 19 H13 Z")],
        "angle": lambda: [svg_path("M26.5 7 L13.5 19 H26.5")],
        "measuredangle": lambda: [svg_path("M26.5 7 L13.5 19 H26.5"), svg_path("M19 19 A5.5 5.5 0 0 0 17.4 15.2", sw=1.35)],
        "sphericalangle": lambda: [svg_path("M26.5 7 L13.5 19 H26.5"), svg_path("M19 19 A5.5 5.5 0 0 0 15.5 13.8", sw=1.35)],
        "square": lambda: ['<rect x="13.5" y="6.5" width="13" height="13" rx="1.2" />'],
        "rectangle": lambda: ['<rect x="10.5" y="8" width="19" height="10" rx="1.2" />'],
        "parallelogram": lambda: [svg_path("M13 8 H29 L27 18 H11 Z")],
        "bigcirc": lambda: [svg_circle(CX, CY, 7.2)],
        "lozenge": lambda: [svg_path("M20 5.8 L25.5 13 L20 20.2 L14.5 13 Z")],
        "degree": lambda: [svg_circle(CX, CY, 3.2)],
        "prime": lambda: [svg_path("M21.5 6.5 L18.5 18.5")],
        "dprime": lambda: [svg_path("M19.2 6.5 L16.2 18.5 M23.8 6.5 L20.8 18.5")],
        "trprime": lambda: [svg_path("M17.0 6.5 L14.0 18.5 M21.2 6.5 L18.2 18.5 M25.4 6.5 L22.4 18.5")],
        "backprime": lambda: [svg_path("M18.5 6.5 L21.5 18.5")],
    }
    if tid in geom:
        return geom[tid](), []

    # Font-derived mathematical letterforms
    special_chars = {
        "number-set-R": "\u211D",
        "number-set-Z": "\u2124",
        "number-set-C": "\u2102",
        "number-set-Q": "\u211A",
        "number-set-N": "\u2115",
        "script-A": "\U0001D49C",
        "script-B": "\u212C",
        "script-S": "\U0001D4AE",
        "script-R": "\u211B",
        "script-N": "\U0001D4A9",
        "fraktur-A": "\U0001D504",
        "fraktur-B": "\U0001D505",
        "fraktur-S": "\U0001D516",
        "fraktur-R": "\u211C",
        "fraktur-N": "\U0001D511",
    }
    if tid in special_chars:
        key = reg_text(f"sym-{tid}", special_chars[tid], family="STIX Two Math", base_size=18.0)
        return [], [TextPlacement(key, CX, CY, 20.0, 17.5)]

    if tid in ("imath", "jmath"):
        key = reg_text(f"sym-{tid}", glyph or tid, family="STIX Two Text", italic=True, base_size=17.5)
        return [], [TextPlacement(key, CX, CY, 16.0, 16.5)]

    assert glyph is not None, f"Unhandled symbol: {tid}"
    key = reg_text(f"sym-{tid}", glyph, family="STIX Two Math", base_size=17.5)
    return [], [TextPlacement(key, CX, CY, 18.5, 17.0)]


def build_entry(group: str, tid: str, glyph: str | None) -> tuple[list[str], list[TextPlacement]]:
    if group == "Fractions":
        return build_fractions(tid)
    if group == "Scripts":
        return build_scripts(tid)
    if group == "Fences":
        return build_fences(tid)
    if group in ("Integrals", "Sums", "BigOperators"):
        return build_integrals_and_big_ops(group, tid)
    if group == "Matrices":
        return build_matrices(tid)
    if group == "Boxes":
        return build_boxes(tid)
    if group == "Embellishments":
        return build_embellishments(tid)
    if group == "Bars":
        return build_bars(tid)
    if group == "Arrows":
        return build_arrows(tid)
    if group == "LabeledArrows":
        return build_labeled_arrows(tid)
    if group == "Operators":
        return build_operators(tid)
    if group == "Relations":
        return build_relations(tid)
    if group == "Sets":
        return build_sets(tid)
    if group == "Logic":
        return build_logic(tid)
    if group == "Spaces":
        return build_spaces(tid)
    if group == "Functions":
        return build_functions(tid)
    if group in ("GreekLower", "GreekUpper"):
        assert glyph is not None
        return build_greek(group, tid, glyph)
    if group == "Symbols":
        return build_symbols(tid, glyph)
    raise ValueError(f"Unknown catalog group: {group}")
