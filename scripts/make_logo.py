"""Рисует логотип плеера: PNG разных размеров и icon.ico.

Знак простой и говорит, чем плеер занят: треугольник «играть», от которого
отрезан кончик. Отрезанный кусок — жёлтый, тем же цветом отрезки помечены
на полосе перемотки; сам треугольник — синий, цвет пройденной части полосы.

Без сторонних библиотек: своя растеризация со сглаживанием, свой PNG
и свой контейнер ICO. Ставить Pillow ради одной картинки незачем.

Запуск:  python scripts/make_logo.py
"""

import pathlib
import struct
import zlib

# --- цвета (совпадают с темой приложения) ---------------------------------

TILE = (0x11, 0x12, 0x14)
RING = (0x36, 0x3D, 0x47)
PLAY = (0x64, 0xC8, 0xFF)
PIECE = (0xFF, 0xCD, 0x3C)

# --- геометрия в долях стороны --------------------------------------------

TILE_RADIUS = 0.22
RING_WIDTH = 0.018

# Треугольник «играть»: основание слева, вершина справа.
BASE_X = 0.30
APEX_X = 0.78
TOP_Y = 0.20
BOTTOM_Y = 0.80

# Где проходит разрез и насколько разъехались половины.
CUT_X = 0.605
GAP = 0.030
SHIFT = 0.035

# Сколько подпроб на пиксель по каждой оси — от этого гладкость краёв.
SAMPLES = 4


def rounded_rect(x, y, radius, inset=0.0):
    """Внутри ли точка скруглённого квадрата, отступив `inset` от краёв."""
    low, high = inset, 1.0 - inset

    if not (low <= x <= high and low <= y <= high):
        return False

    cx = min(max(x, low + radius), high - radius)
    cy = min(max(y, low + radius), high - radius)
    return (x - cx) ** 2 + (y - cy) ** 2 <= radius**2


def in_triangle(x, y):
    """Внутри ли точка треугольника «играть»."""
    if x < BASE_X or x > APEX_X:
        return False

    # Половина высоты линейно спадает от основания к вершине.
    progress = (x - BASE_X) / (APEX_X - BASE_X)
    half = (BOTTOM_Y - TOP_Y) / 2.0 * (1.0 - progress)
    return abs(y - 0.5) <= half


def sample(x, y):
    """Цвет точки: (r, g, b, a)."""
    if not rounded_rect(x, y, TILE_RADIUS):
        return (0, 0, 0, 0)

    # Отрезанный кусок сдвинут вправо — поэтому проверяется со сдвигом назад.
    if in_triangle(x - SHIFT, y) and x - SHIFT >= CUT_X + GAP / 2:
        return PIECE + (255,)

    if in_triangle(x, y) and x <= CUT_X - GAP / 2:
        return PLAY + (255,)

    # Тонкая светлая кайма: без неё тёмная плитка растворяется на тёмной
    # же странице, и от знака остаётся один висящий в пустоте треугольник.
    if not rounded_rect(x, y, TILE_RADIUS - RING_WIDTH, RING_WIDTH):
        return RING + (255,)

    return TILE + (255,)


def render(size):
    """Кадр size×size в виде байтов RGBA."""
    step = 1.0 / (size * SAMPLES)
    pixels = bytearray()

    for row in range(size):
        for col in range(size):
            acc = [0, 0, 0, 0]

            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    x = (col * SAMPLES + sx + 0.5) * step
                    y = (row * SAMPLES + sy + 0.5) * step
                    red, green, blue, alpha = sample(x, y)
                    # Складываем с учётом прозрачности, иначе по краям
                    # проступает чёрная кайма.
                    acc[0] += red * alpha
                    acc[1] += green * alpha
                    acc[2] += blue * alpha
                    acc[3] += alpha

            total = acc[3]
            if total == 0:
                pixels += bytes(4)
                continue

            pixels += bytes(
                (
                    round(acc[0] / total),
                    round(acc[1] / total),
                    round(acc[2] / total),
                    round(total / (SAMPLES * SAMPLES)),
                )
            )

    return bytes(pixels)


def png(size, pixels):
    """Собирает PNG из готовых байтов RGBA."""

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    raw = b"".join(
        b"\x00" + pixels[row * size * 4 : (row + 1) * size * 4] for row in range(size)
    )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def ico(images):
    """Контейнер ICO из готовых PNG. Windows читает PNG внутри ICO."""
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = len(header) + 16 * len(images)
    entries = b""
    body = b""

    for size, data in images:
        entries += struct.pack(
            "<BBBBHHII",
            size if size < 256 else 0,
            size if size < 256 else 0,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        body += data
        offset += len(data)

    return header + entries + body


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    assets = root / "assets"
    assets.mkdir(exist_ok=True)

    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = [(size, png(size, render(size))) for size in sizes]

    (assets / "logo.png").write_bytes(dict(images)[256])
    (root / "crates/pith-app/assets/icon.ico").write_bytes(ico(images))

    print("logo.png и icon.ico готовы:", ", ".join(str(s) for s in sizes))


if __name__ == "__main__":
    main()
