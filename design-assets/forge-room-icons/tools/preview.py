"""Compose the labeled PNG from actual SVG renders, at exact 24 and 48 pixel sizes."""

from pathlib import Path
import html
import json
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
RENDERS = ROOT.parents[1] / "artifacts/build-tools/icon-renderer/renders"
ICONS = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))["icons"]
BACKGROUND = "#121314"
INACTIVE = "#A0A1A5"
ACTIVE = "#EFEFEE"
ACCENT = "#C74C45"
FONT_DIR = Path("C:/Windows/Fonts")


def font(size, bold=False):
    return ImageFont.truetype(str(FONT_DIR / ("segoeuib.ttf" if bold else "segoeui.ttf")), size)


def mix(top, bottom, opacity):
    a = tuple(int(top[i:i + 2], 16) for i in (1, 3, 5))
    b = tuple(int(bottom[i:i + 2], 16) for i in (1, 3, 5))
    return tuple(round(x * opacity + y * (1 - opacity)) for x, y in zip(a, b))


def preview():
    sheet = Image.new("RGB", (1376, 1296), BACKGROUND)
    draw = ImageDraw.Draw(sheet)
    draw.text((48, 32), "Forge Room", font=font(36, True), fill=ACTIVE)
    draw.text((48, 83), "Navigation & sidebar icons", font=font(21), fill=INACTIVE)
    draw.text((1328, 46), "24 px / 48 px", font=font(20), fill=ACTIVE, anchor="ra")
    draw.text((1328, 80), "1.75 px stroke at 24 px", font=font(16), fill=INACTIVE, anchor="ra")
    line = mix(INACTIVE, BACKGROUND, 0.22)
    draw.line((48, 130, 1328, 130), fill=line, width=1)
    draw.text((48, 150), "Navigation rail", font=font(20, True), fill=ACTIVE)
    draw.text((1328, 155), "In reference order", font=font(15), fill=INACTIVE, anchor="ra")
    draw.text((48, 538), "Sidebar & controls", font=font(20, True), fill=ACTIVE)

    for index, icon in enumerate(ICONS):
        if index < 8:
            row, column = divmod(index, 4)
            top = 190 + row * 154
        else:
            row, column = divmod(index - 8, 4)
            top = 578 + row * 154
        left = 48 + column * 320
        draw.text((left + 18, top + 8), icon["name"], font=font(17), fill=ACTIVE)
        for size, cx in ((24, left + 86), (48, left + 222)):
            cy = top + 79
            if icon["name"] == "room-active":
                side = size * 4 // 3
                box = (cx - side // 2, cy - side // 2, cx + side // 2 - 1, cy + side // 2 - 1)
                draw.rounded_rectangle(box, radius=size // 4, fill=mix(ACCENT, BACKGROUND, 0.16), outline=mix(ACCENT, BACKGROUND, 0.52), width=max(1, size // 24))
            with Image.open(RENDERS / f'{icon["name"]}-{size}.png') as im:
                assert im.size == (size, size)
                sheet.paste(im, (cx - size // 2, cy - size // 2), im)
            draw.text((cx, top + 119), f"{size} px", font=font(13), fill=INACTIVE, anchor="ma")
        draw.line((left + 18, top + 150, left + 302, top + 150), fill=line, width=1)

    draw.text((48, 1230), "24 icons  /  currentColor  /  round caps & joins", font=font(16), fill=INACTIVE)
    draw.text((48, 1257), "Room active: identical geometry on a red tinted square. Preview colors only.", font=font(14), fill=INACTIVE)
    sheet.save(ROOT / "preview.png", optimize=True)
    print(f"Saved preview.png: {sheet.width} x {sheet.height}, exact 24 px and 48 px icon rasters.")


def html_preview():
    sections = []
    for title, group in (("Navigation rail", ICONS[:8]), ("Sidebar & controls", ICONS[8:])):
        cards = []
        for icon in group:
            source = (ROOT / icon["file"]).read_text(encoding="utf-8").strip()
            active = " active" if icon["name"] == "room-active" else ""
            samples = []
            for size in (24, 48):
                svg = source.replace('width="24" height="24"', f'width="{size}" height="{size}"')
                svg = svg.replace("<svg ", '<svg aria-hidden="true" focusable="false" ')
                samples.append(f'<figure><span class="tile{active}" style="--size:{size}px">{svg}</span><figcaption>{size} px</figcaption></figure>')
            cards.append(f'<article><h3>{html.escape(icon["name"])}</h3><div class="samples">{"".join(samples)}</div></article>')
        sections.append(f'<section><h2>{title}</h2><div class="grid">{"".join(cards)}</div></section>')
    page = '''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Forge Room icon preview</title><style>
:root{color-scheme:dark;background:#121314;color:#EFEFEE;font:16px/1.5 "Segoe UI",sans-serif}
*{box-sizing:border-box}body{margin:0 auto;padding:32px;max-width:1440px}header{border-bottom:1px solid #A0A1A538;padding-bottom:24px}
h1{font-size:36px;margin:0}header p,figcaption,footer{color:#A0A1A5}h2{font-size:20px;margin:32px 0 12px}h3{font-size:17px;font-weight:400;margin:0}
.grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:16px}article{padding:16px;border-bottom:1px solid #A0A1A538}
.samples{display:flex;justify-content:space-around;align-items:center}figure{margin:12px 0;text-align:center;width:50%}
.tile{display:inline-flex;align-items:center;justify-content:center;color:#A0A1A5;width:calc(var(--size)*4/3);height:calc(var(--size)*4/3);vertical-align:middle}
.active{color:#EFEFEE;background:#C74C4529;border:1px solid #C74C4585;border-radius:calc(var(--size)/4)}figcaption{font-size:13px;margin-top:8px}footer{margin-top:32px}
@media(max-width:900px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:480px){body{padding:20px}.grid{grid-template-columns:1fr}}
</style></head><body><header><h1>Forge Room</h1><p>Navigation &amp; sidebar icons. 1.75 px stroke on a 24 &times; 24 viewBox.</p></header>
'''
    page += "\n".join(sections)
    page += '<footer>24 icons. Room active uses identical geometry on a red tinted square. All SVG strokes inherit currentColor.</footer></body></html>\n'
    (ROOT / "preview.html").write_text(page, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    preview()
    html_preview()
