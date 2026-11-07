"""Build the approved architectural mark and an outlined native-font wordmark."""

from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

ROOT = Path(__file__).resolve().parent
SHAPES = [
    ("#E8E6DD", "M22 68 128 12 226 63 190 82 128 48 57 86v90l-35 20Z"),
    ("#D9837F", "m80 129 158-69v38l-124 55v71l-18 10-16-9Z"),
    ("#D67A3F", "m146 179 36-17v60l-36-18Z"),
]


def build():
    paths = [f'  <path fill="{color}" d="{shape}"/>' for color, shape in SHAPES]
    header = '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">'
    (ROOT / "forge-mark-color.svg").write_text(header + "\n" + "\n".join(paths) + "\n</svg>\n", encoding="utf-8")
    mono = [f'  <path fill="currentColor" d="{shape}"/>' for _, shape in SHAPES]
    (ROOT / "forge-mark-mono.svg").write_text(header + "\n" + "\n".join(mono) + "\n</svg>\n", encoding="utf-8")

    font = TTFont("C:/Windows/Fonts/seguisb.ttf")
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    scale = 110 / font["head"].unitsPerEm
    x = 290.0
    for character in "Forge Room":
        name = cmap[ord(character)]
        pen = SVGPathPen(glyphs)
        glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, x, 164)))
        if commands := pen.getCommands():
            paths.append(f'  <path fill="#E8E6DD" d="{commands}"/>')
        x += font["hmtx"][name][0] * scale
    width = round(x + 20)
    header = f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="256" viewBox="0 0 {width} 256">'
    (ROOT / "forge-room-logo.svg").write_text(header + "\n" + "\n".join(paths) + "\n</svg>\n", encoding="utf-8")
    print(f"Built two 256-square symbols and {width}x256 path-only wordmark lockup.")


if __name__ == "__main__":
    build()
