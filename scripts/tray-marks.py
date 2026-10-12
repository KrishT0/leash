"""Draw the tray marks, one file for each size Windows asks for.

    python scripts/tray-marks.py

The tray icon is 16 pixels across on a normal display and larger on a scaled
one. Handing Windows one large image makes it shrink that image itself, which
turns a thin ring into a smudge at 16 pixels. Each size is drawn here instead,
with the shape sampled 8 by 8 times per pixel so the edges are soft rather than
jagged.

The mark is the one from Home: an orbit, with one service resting on it. The
shapes are heavy on purpose, because this is drawn smaller than anything else in
the program. The app icon beside it, generated from scripts/icon-app.svg, uses
the same two shapes at a much finer weight.

There is no colour of its own. The tray recolours every pixel that is not
transparent, so white is a placeholder, and the app paints it green, amber or
red at run time.

The same run writes scripts/icon-check.html, which shows the four files at the
real size, magnified eight times with the pixels untouched.
"""

import base64
import math
import pathlib
import struct
import zlib

# The shipped weight. Chosen by eye from the grid in icon-candidates.html, at
# the four sizes Windows draws a tray icon: bold enough at 16 pixels to read at
# a glance, and clean at 32. A heavier dot merges into the ring, and a lighter
# ring goes faint at 16.
RING_RADIUS = 0.30
RING_STROKE = 0.14
DOT_X = 0.78
DOT_Y = 0.50
DOT_RADIUS = 0.14

# The weights the shipped one was chosen over, drawn into
# scripts/icon-candidates.html so the choice can still be checked by eye. The
# first entry is what the app uses.
VARIANTS = [
    ("Shipped: ring 0.14, dot 0.14", (0.30, 0.14, 0.78, 0.50, 0.14)),
    ("Heavier: ring 0.16, dot 0.15", (0.30, 0.16, 0.78, 0.50, 0.15)),
    ("Lighter: ring 0.12, dot 0.13", (0.30, 0.12, 0.78, 0.50, 0.13)),
    ("Separated: the dot clear of the ring", (0.26, 0.14, 0.37, 0.50, 0.10)),
    ("Ring only, no dot", (0.30, 0.16, 0.0, 0.50, 0.0)),
]

# The sizes Windows draws a tray icon at, and the display scale that asks for
# each. The app picks from these at run time.
SIZES = [16, 20, 24, 32]

SAMPLES = 8

HERE = pathlib.Path(__file__).resolve().parent
ICONS = HERE.parent / "src-tauri" / "icons"


def inside(x: float, y: float, shape: tuple[float, ...] = ()) -> bool:
    """True when a point, in the 0 to 1 space, is part of the mark."""
    radius, stroke, dot_x, dot_y, dot_radius = shape or (
        RING_RADIUS,
        RING_STROKE,
        DOT_X,
        DOT_Y,
        DOT_RADIUS,
    )

    if abs(math.hypot(x - 0.5, y - 0.5) - radius) <= stroke / 2:
        return True
    if dot_radius <= 0:
        return False
    return math.hypot(x - dot_x, y - dot_y) <= dot_radius


def coverage(px: int, py: int, size: int, shape: tuple[float, ...] = ()) -> float:
    """How much of one pixel the mark covers, 0 to 1."""
    hits = 0
    for row in range(SAMPLES):
        for column in range(SAMPLES):
            x = (px + (column + 0.5) / SAMPLES) / size
            y = (py + (row + 0.5) / SAMPLES) / size
            if inside(x, y, shape):
                hits += 1
    return hits / (SAMPLES * SAMPLES)


def chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def draw_png(size: int, shape: tuple[float, ...] = ()) -> bytes:
    rows = []
    for py in range(size):
        row = bytearray()
        for px in range(size):
            alpha = round(coverage(px, py, size, shape) * 255)
            # White, so the recolour in the app has something to replace.
            row += bytes((255, 255, 255, alpha))
        rows.append(b"\x00" + bytes(row))

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
    png += chunk(b"IEND", b"")

    return png


def write_png(path: pathlib.Path, size: int) -> bytes:
    png = draw_png(size)
    path.write_bytes(png)
    return png


def write_check_page(drawn: list[tuple[int, bytes]]) -> None:
    figures = []
    for size, png in drawn:
        data = base64.b64encode(png).decode("ascii")
        scale = size / 16
        percent = round(scale * 100)
        figures.append(
            f"""    <figure style="--size: {size}px">
      <img src="data:image/png;base64,{data}" alt="{size}" />
      <figcaption>{size} px, {percent} percent</figcaption>
    </figure>"""
        )

    html = f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Leash tray mark, at the real sizes</title>
    <style>
      body {{
        margin: 0;
        padding: 32px;
        background: #1c1c1c;
        color: #ddd;
        font: 12px/1.6 system-ui, sans-serif;
        display: flex;
        gap: 36px;
        align-items: flex-end;
      }}
      figure {{
        margin: 0;
        display: grid;
        gap: 10px;
        justify-items: center;
      }}
      img {{
        /* The files themselves, magnified with the pixels untouched, so what is
           on screen is what the tray paints. */
        width: calc(var(--size) * 8);
        height: calc(var(--size) * 8);
        image-rendering: pixelated;
      }}
      figcaption {{
        font-family: monospace;
        color: #999;
      }}
    </style>
  </head>
  <body>
{chr(10).join(figures)}
  </body>
</html>
"""
    (HERE / "icon-check.html").write_text(html, encoding="utf8")


def write_candidates_page() -> None:
    """Write the comparison, so the choice of weight can be looked at."""
    rows = []
    for label, shape in VARIANTS:
        cells = []
        for size in SIZES:
            data = base64.b64encode(draw_png(size, shape)).decode("ascii")
            cells.append(f'<img src="data:image/png;base64,{data}" alt="{size}" />')
        rows.append(
            f'      <tr><th>{label}</th>'
            + "".join(f"<td>{cell}</td>" for cell in cells)
            + "</tr>"
        )

    heads = "".join(f"<th>{size} px</th>" for size in SIZES)
    html = f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Leash tray mark, the weights that were compared</title>
    <style>
      body {{
        margin: 0;
        padding: 32px;
        background: #1c1c1c;
        color: #ddd;
        font: 13px/1.6 system-ui, sans-serif;
      }}
      table {{ border-collapse: collapse; }}
      th, td {{ padding: 14px 20px; text-align: left; }}
      thead th, tbody th {{ font-weight: 500; color: #999; }}
      tbody th {{ text-align: left; }}
      tbody tr + tr {{ border-top: 1px solid #333; }}
      img {{
        /* The real pixels, magnified without smoothing. */
        width: calc(var(--size) * 6);
        height: calc(var(--size) * 6);
        image-rendering: pixelated;
      }}
      td:nth-child(2) img {{ --size: 16px; }}
      td:nth-child(3) img {{ --size: 20px; }}
      td:nth-child(4) img {{ --size: 24px; }}
      td:nth-child(5) img {{ --size: 32px; }}
    </style>
  </head>
  <body>
    <p>The first row is what the app uses. The others are the weights it was
    chosen over, at the four sizes Windows draws a tray icon.</p>
    <table>
      <thead><tr><th>Weight</th>{heads}</tr></thead>
      <tbody>
{chr(10).join(rows)}
      </tbody>
    </table>
  </body>
</html>
"""
    (HERE / "icon-candidates.html").write_text(html, encoding="utf8")


def main() -> None:
    drawn = []
    for size in SIZES:
        path = ICONS / f"tray-{size}.png"
        png = write_png(path, size)
        drawn.append((size, png))
        print(f"wrote {path.relative_to(HERE.parent)} ({len(png)} bytes)")

    write_check_page(drawn)
    print(f"wrote {(HERE / 'icon-check.html').relative_to(HERE.parent)}")

    write_candidates_page()
    print(f"wrote {(HERE / 'icon-candidates.html').relative_to(HERE.parent)}")


if __name__ == "__main__":
    main()
