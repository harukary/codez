#!/usr/bin/env python3
"""
コントラスト比計算ユーティリティ

使い方
- python3 contrast_ratio.py --fg "#111827" --bg "#FFFFFF"
- python3 contrast_ratio.py --fg 111827 --bg ffffff --large-text

判定の目安
- 通常テキスト: 4.5:1
- 大きいテキスト: 3.0:1
- UI部品/境界: 3.0:1
"""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from typing import Tuple


HEX_RE = re.compile(r"^#?([0-9a-fA-F]{6})$")


def parse_hex_color(s: str) -> Tuple[float, float, float]:
    m = HEX_RE.match(s.strip())
    if not m:
        raise ValueError(f"Invalid hex color: {s} (expected RRGGBB)")
    h = m.group(1)
    r = int(h[0:2], 16) / 255.0
    g = int(h[2:4], 16) / 255.0
    b = int(h[4:6], 16) / 255.0
    return r, g, b


def srgb_to_linear(c: float) -> float:
    if c <= 0.04045:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def relative_luminance(rgb: Tuple[float, float, float]) -> float:
    r, g, b = rgb
    rl = srgb_to_linear(r)
    gl = srgb_to_linear(g)
    bl = srgb_to_linear(b)
    return 0.2126 * rl + 0.7152 * gl + 0.0722 * bl


def contrast_ratio(fg: Tuple[float, float, float], bg: Tuple[float, float, float]) -> float:
    l1 = relative_luminance(fg)
    l2 = relative_luminance(bg)
    lighter = max(l1, l2)
    darker = min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)


@dataclass(frozen=True)
class Thresholds:
    aa_normal: float = 4.5
    aaa_normal: float = 7.0
    aa_large: float = 3.0
    aaa_large: float = 4.5
    ui_component: float = 3.0


def verdict(ratio: float, large_text: bool) -> str:
    t = Thresholds()
    aa = t.aa_large if large_text else t.aa_normal
    aaa = t.aaa_large if large_text else t.aaa_normal
    parts = []
    parts.append(f"AA({'large' if large_text else 'normal'}): {'PASS' if ratio >= aa else 'FAIL'} (>= {aa})")
    parts.append(f"AAA({'large' if large_text else 'normal'}): {'PASS' if ratio >= aaa else 'FAIL'} (>= {aaa})")
    parts.append(f"UI components: {'PASS' if ratio >= t.ui_component else 'FAIL'} (>= {t.ui_component})")
    return " | ".join(parts)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fg", required=True, help="foreground color in RRGGBB or #RRGGBB")
    ap.add_argument("--bg", required=True, help="background color in RRGGBB or #RRGGBB")
    ap.add_argument("--large-text", action="store_true", help="use large-text thresholds")
    args = ap.parse_args()

    fg = parse_hex_color(args.fg)
    bg = parse_hex_color(args.bg)
    ratio = contrast_ratio(fg, bg)

    print(f"FG: {args.fg}  BG: {args.bg}")
    print(f"Contrast ratio: {ratio:.2f}:1")
    print(verdict(ratio, args.large_text))


if __name__ == "__main__":
    main()
