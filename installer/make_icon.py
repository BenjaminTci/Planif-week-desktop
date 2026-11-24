"""Génère assets/planif-week.ico (16 à 256 px) sans dépendance externe.

Même dessin que l'icône de fenêtre de src/main.rs : un calendrier bleu.
Usage : python installer/make_icon.py
"""

import struct
import zlib
from pathlib import Path

BLUE = (0x2F, 0x6B, 0xF2)
CELL = (0xE3, 0xEB, 0xFE)
WHITE = (0xFF, 0xFF, 0xFF)
SIZES = [16, 24, 32, 48, 64, 128, 256]
SUPERSAMPLE = 4


def in_rounded(x, y, x0, y0, x1, y1, r):
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def sample(x, y):
    """Couleur au point (x, y) d'une grille de 64 × 64, ou None (transparent)."""
    if not in_rounded(x, y, 2, 4, 62, 62, 12):
        return None
    if y < 20:
        return BLUE
    if (
        in_rounded(x, y, 12, 28, 52, 52, 3)
        and (x - 12) % 14 < 10
        and (y - 28) % 13 < 9
    ):
        return CELL
    return WHITE


def render(size):
    rows = []
    n = SUPERSAMPLE
    for py in range(size):
        row = bytearray([0])  # filtre PNG « aucun »
        for px in range(size):
            r = g = b = a = 0
            for sy in range(n):
                for sx in range(n):
                    x = (px + (sx + 0.5) / n) * 64 / size
                    y = (py + (sy + 0.5) / n) * 64 / size
                    c = sample(x, y)
                    if c:
                        r += c[0]
                        g += c[1]
                        b += c[2]
                        a += 1
            if a:
                row += bytes((r // a, g // a, b // a, 255 * a // (n * n)))
            else:
                row += b"\0\0\0\0"
        rows.append(bytes(row))
    return b"".join(rows)


def png(size):
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(render(size), 9))
        + chunk(b"IEND", b"")
    )


def main():
    images = [png(s) for s in SIZES]
    out = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    for size, data in zip(SIZES, images):
        dim = 0 if size == 256 else size
        out += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    out += b"".join(images)
    target = Path(__file__).resolve().parent.parent / "assets" / "planif-week.ico"
    target.parent.mkdir(exist_ok=True)
    target.write_bytes(out)
    print(f"{target} ({len(out)} octets)")


if __name__ == "__main__":
    main()
