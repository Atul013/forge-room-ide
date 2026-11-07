"""Validate source constraints, sprite parity, rendered padding, and preview integrity."""

from pathlib import Path
import json
import hashlib
import re
import xml.etree.ElementTree as ET
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
RENDERS = ROOT.parents[1] / "artifacts/build-tools/icon-renderer/renders"
NS = "{http://www.w3.org/2000/svg}"
EXPECTED = "explorer search source-control run-debug extensions ai terminal room new-file refresh more close plus chevron-right chevron-down check arrow-up arrow-down folder-open file file-json file-ts commit-dropdown room-active".split()
EXPECTED += "code studio workflows models tests cloud settings accessibility-off pulse users user-plus user lock clock calendar eye bot target flag layers external-link github shield send warning bell rocket hourglass upload-box share panel-left panel-bottom panel-right window-maximize window-minimize circle-check circle-dot circle tag arrow-right".split()
EXPECTED += ["forge-mark"]
STYLE = {"fill": "none", "stroke": "currentColor", "stroke-width": "1.75", "stroke-linecap": "round", "stroke-linejoin": "round", "viewBox": "0 0 24 24"}


def validate():
    original = json.loads((ROOT / "tools/original-icon-hashes.json").read_text(encoding="utf-8"))
    assert len(original) == 24
    assert all(hashlib.sha256((ROOT / "svg" / name).read_bytes()).hexdigest() == digest for name, digest in original.items()), "original icons changed"
    files = sorted((ROOT / "svg").glob("*.svg"))
    assert {p.stem for p in files} == set(EXPECTED)
    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    assert [i["name"] for i in manifest["icons"]] == EXPECTED
    assert manifest["railOrder"] == EXPECTED[:8]
    sprite = ET.parse(ROOT / "forge-room-sprite.svg").getroot()
    assert sprite.tag == NS + "svg"
    assert len(sprite) == len(EXPECTED)
    assert [s.get("id") for s in sprite] == ["forge-" + name for name in EXPECTED]
    bounds = {}
    for name in EXPECTED:
        file = ROOT / "svg" / f"{name}.svg"
        source = file.read_text(encoding="utf-8")
        svg = ET.fromstring(source)
        symbol = next(s for s in sprite if s.get("id") == "forge-" + name)
        assert re.fullmatch(r"[a-z]+(?:-[a-z]+)*", name)
        assert svg.tag == NS + "svg" and symbol.tag == NS + "symbol"
        for k, value in STYLE.items():
            assert svg.get(k) == symbol.get(k) == value, (name, k)
        assert svg.get("width") == svg.get("height") == "24"
        assert [(p.tag, p.attrib) for p in svg] == [(p.tag, p.attrib) for p in symbol], name
        assert all(p.tag == NS + "path" and set(p.attrib) == {"d"} for p in svg), name
        assert all(not (p.text or "").strip() and not (p.tail or "").strip() for p in svg.iter())
        assert not re.search(r"#[0-9a-fA-F]{3,8}\b|data:|<text|<image|<style|<script|<foreignObject|<filter|<linearGradient|<radialGradient|font", source)
        for size in (24, 48):
            with Image.open(RENDERS / f"{name}-{size}.png") as a, Image.open(RENDERS / f"{name}-{size}-sprite.png") as b:
                assert a.size == b.size == (size, size)
                assert not any(ImageChops.difference(a, b).getextrema()[i][1] for i in range(4)), (name, size, "sprite differs")
                assert a.getchannel("A").getbbox(), (name, "blank")
        with Image.open(RENDERS / f"{name}-bounds.png") as image:
            bbox = image.getchannel("A").getbbox()
            bounds[name] = [v / 16 for v in bbox]
            assert bbox[0] >= 31 and bbox[1] >= 31 and bbox[2] <= 353 and bbox[3] <= 353, (name, bounds[name], "padding")
    assert (ROOT / "svg/room.svg").read_bytes() == (ROOT / "svg/room-active.svg").read_bytes()
    assert (ROOT / "svg/chevron-down.svg").read_bytes() == (ROOT / "svg/commit-dropdown.svg").read_bytes()
    with Image.open(ROOT / "preview.png") as preview:
        assert preview.size == (1376, 680 + ((len(EXPECTED) - 8 + 3) // 4) * 154)
        assert preview.getpixel((0, 0)) == (18, 19, 20)
    with Image.open(ROOT / "preview.png") as preview:
        preview.verify()
    result = {
        "status": "passed",
        "standaloneIcons": len(EXPECTED),
        "spriteSymbols": len(sprite),
        "exactSpriteRasterComparisons": len(EXPECTED) * 2,
        "geometryAliasesMatch": True,
        "originalIconsByteIdentical": len(original),
        "allStrokesCurrentColor": True,
        "strokeWidth": 1.75,
        "svgElements": ["svg", "symbol", "path"],
        "previewSize": [1376, 680 + ((len(EXPECTED) - 8 + 3) // 4) * 154],
        "paintedBoundsAt16x": bounds,
    }
    (ROOT / "validation.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"PASS: {len(EXPECTED)} path-only SVGs; {len(sprite)} symbols; {len(EXPECTED) * 2} pixel-identical sprite comparisons; currentColor, 1.75 px strokes, rounded caps/joins, padding, aliases, and PNG integrity.")


if __name__ == "__main__":
    validate()
