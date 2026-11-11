"""Draws the Windows installers' pictures: the horizon, in Paper.

Writes each as an SVG, to look at and to edit from, and as the 24-bit BMP
the installers want. Shapes only: the installers scale these with no
smoothing, which ruins lettering. Run from the repository root:

    python design/installer/draw.py      (needs Pillow)

The two NSIS pictures are drawn at twice their size, so they stay sharp on
a scaled display; MSI takes its two at exactly the size it shows them.
"""
import os
from PIL import Image, ImageDraw

OUT = 'design/installer'
os.makedirs(OUT, exist_ok=True)

PAPER, WHITE = '#f4f0e7', '#ffffff'
INK, SUN, SUN_DIM = '#1c1b18', '#dca544', '#cdb98c'
SKY, GROUND = '#dfe8ea', '#e8dfca'
SS = 4  # supersampling


def rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


class Pic:
    """Shapes, kept twice: as SVG text and as pixels."""

    def __init__(self, w, h, scale, bg):
        self.w, self.h, self.scale = w, h, scale
        self.k = scale * SS
        self.im = Image.new('RGB', (w * self.k, h * self.k), rgb(bg))
        self.svg = [f'<rect width="{w}" height="{h}" fill="{bg}"/>']

    def rect(self, x, y, w, h, fill, opacity=1.0, rx=0.0):
        k = self.k
        layer = Image.new('RGBA', self.im.size, (0, 0, 0, 0))
        ImageDraw.Draw(layer).rounded_rectangle(
            [x * k, y * k, (x + w) * k - 1, (y + h) * k - 1], radius=rx * k,
            fill=rgb(fill) + (round(255 * opacity),))
        self.im = Image.alpha_composite(self.im.convert('RGBA'), layer).convert('RGB')
        o = '' if opacity == 1 else f' opacity="{opacity}"'
        r = '' if rx == 0 else f' rx="{rx}"'
        self.svg.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}"{r} fill="{fill}"{o}/>')

    def circle(self, cx, cy, r, fill, opacity=1.0):
        k = self.k
        layer = Image.new('RGBA', self.im.size, (0, 0, 0, 0))
        ImageDraw.Draw(layer).ellipse(
            [(cx - r) * k, (cy - r) * k, (cx + r) * k, (cy + r) * k],
            fill=rgb(fill) + (round(255 * opacity),))
        self.im = Image.alpha_composite(self.im.convert('RGBA'), layer).convert('RGB')
        o = '' if opacity == 1 else f' opacity="{opacity}"'
        self.svg.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}"{o}/>')

    def save(self, name):
        final = self.im.resize((self.w * self.scale, self.h * self.scale), Image.LANCZOS)
        final.save(f'{OUT}/{name}.bmp')
        body = '\n  '.join(self.svg)
        open(f'{OUT}/{name}.svg', 'w', encoding='utf-8', newline='\n').write(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" '
            f'width="{self.w}" height="{self.h}">\n  {body}\n</svg>\n')


STARS = [(22, 30), (58, 18), (97, 44), (131, 24), (144, 70), (36, 78), (112, 92)]


def scene(p, x0, w, h, horizon, sun_r, sun_y):
    """The horizon as Home draws it, in a column `w` wide from `x0`."""
    p.rect(x0, 0, w, horizon, SKY)
    p.rect(x0, horizon, w, h - horizon, GROUND)
    for sx, sy in STARS:
        p.circle(x0 + sx * w / 164, sy * horizon / 206, 0.8, INK, 0.22)
    for band in (54, 32, 14):
        band = band * horizon / 206
        p.rect(x0, horizon - band, w, band, SUN, 0.13)
    p.circle(x0 + w / 2, sun_y, sun_r, SUN)
    p.rect(x0 + 12, horizon - 0.75, w - 24, 1.5, INK)
    p.rect(x0 + w / 2 - 18, horizon + 18, 36, 2.5, SUN, rx=1.25)
    p.rect(x0 + w / 2 - 10, horizon + 28, 20, 2.5, SUN_DIM, rx=1.25)


# NSIS: the welcome and finish pages' side, 164 x 314, drawn at 2x.
p = Pic(164, 314, 2, SKY)
scene(p, 0, 164, 314, 206, 26, 142)
p.save('nsis-sidebar')

# NSIS: the header of the pages between, 150 x 57. The header is paper too
# (MUI_BGCOLOR in src-tauri/windows/installer.nsh).
p = Pic(150, 57, 2, PAPER)
p.rect(0, 0, 150, 38, SKY)
p.rect(0, 38, 150, 19, GROUND)
for band in (16, 9):
    p.rect(0, 38 - band, 150, band, SUN, 0.13)
p.circle(75, 26, 9, SUN)
p.rect(8, 37.25, 134, 1.5, INK)
p.rect(66, 45, 18, 2, SUN, rx=1)
p.save('nsis-header')

# MSI: the first and last dialogs, 493 x 312: the scene down the left, the
# installer's own black text on white to the right of it.
p = Pic(493, 312, 1, WHITE)
scene(p, 0, 164, 312, 205, 26, 141)
p.save('wix-dialog')

# MSI: the banner over the other dialogs, 493 x 58; its text is on the left.
p = Pic(493, 58, 1, WHITE)
p.rect(403, 0, 90, 39, SKY)
p.rect(403, 39, 90, 19, GROUND)
for band in (16, 9):
    p.rect(403, 39 - band, 90, band, SUN, 0.13)
p.circle(448, 27, 9, SUN)
p.rect(411, 38.25, 74, 1.5, INK)
p.rect(439, 46, 18, 2, SUN, rx=1)
p.save('wix-banner')
print('ok')
