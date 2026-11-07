from collections import Counter
from pathlib import Path
import hashlib
import json
import shutil
import sys
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path(sys.argv[1]) if len(sys.argv)>1 else ROOT / "source/reference.png"
ROOT.joinpath("source").mkdir(parents=True, exist_ok=True)
if SOURCE.resolve() != (ROOT / "source/reference.png").resolve():
    shutil.copyfile(SOURCE, ROOT / "source/reference.png")
im = Image.open(SOURCE).convert("RGB")
a = np.asarray(im).astype(float)
assert im.size == (1672, 941)

def peaks(axis, fixed, first, last):
    line = a[fixed, first:last] if axis == "x" else a[first:last, fixed]
    d = np.max(np.abs(np.diff(line, axis=0)), axis=1)
    ranked = sorted(range(len(d)), key=lambda i: -d[i])[:24]
    return [(first + i + 1, round(float(d[i]), 1)) for i in sorted(ranked)]

report = {
    "sha256": hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
    "size": list(im.size),
    "unique_rgb_colors": len(Counter(im.getdata())),
    "horizontal_scans": {str(y): peaks("x", y, 20, 1655) for y in [25, 75, 76, 78, 124, 125, 200, 530, 625, 669, 860, 861, 903, 904]},
    "vertical_scans": {str(x): peaks("y", x, 15, 915) for x in [28, 29, 50, 104, 105, 200, 386, 387, 500, 1000, 1217, 1218, 1220, 1400, 1646, 1647]},
}
ROOT.joinpath("measurements").mkdir(exist_ok=True)
(ROOT / "measurements/source-inspection.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(f"Inspected {im.width}x{im.height}; {report['unique_rgb_colors']} colors; SHA256 {report['sha256']}")

for name, rect in {
    "window-top-left": (15, 10, 160, 160),
    "title-bar": (25, 20, 1650, 78),
    "explorer": (104, 76, 387, 532),
    "source-control": (104, 530, 387, 862),
    "editor-top": (386, 76, 1219, 453),
    "editor-floats": (730, 350, 1210, 609),
    "bottom-panel": (386, 624, 1219, 862),
    "right-dock": (1218, 76, 1648, 862),
    "status-bar": (28, 861, 1648, 905),
}.items():
    folder = ROOT / "measurements/crops"
    folder.mkdir(exist_ok=True)
    im.crop(rect).resize(((rect[2]-rect[0])*2, (rect[3]-rect[1])*2)).save(folder / f"{name}-2x.png")
