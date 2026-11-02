"""Compare local fonts to isolated screenshot glyphs; matches are not font identification."""
from pathlib import Path
import json
import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
image = np.asarray(Image.open(ROOT / "source/reference.png").convert("RGB"))
samples = [
    ("brand", "Forge Room", [89, 39, 108, 23]),
    ("panel-heading", "Explorer", [124, 92, 74, 22]),
    ("tab", "Room.tsx", [430, 92, 72, 21]),
    ("tree", "desktop", [194, 184, 60, 20]),
    ("section-heading", "Source Control", [128, 545, 124, 22]),
    ("card-heading", "How can I help you today?", [1249, 224, 224, 24]),
    ("chat-body", "Update the Room component to show", [1301, 384, 305, 22]),
    ("breadcrumb", "apps", [408, 133, 32, 20]),
    ("code", "import", [473, 169, 54, 19]),
    ("terminal", "ready in 412 ms", [553, 766, 134, 20]),
    ("status", "feature/room-collab", [80, 874, 132, 20]),
    ("small-helper", "Agent", [1147, 490, 40, 18]),
]
fonts = [
    ("Segoe UI", "400", "segoeui.ttf"),
    ("Segoe UI", "600", "seguisb.ttf"),
    ("Segoe UI", "700", "segoeuib.ttf"),
    ("Arial", "400", "arial.ttf"),
    ("Arial", "700", "arialbd.ttf"),
    ("Calibri", "400", "calibri.ttf"),
    ("Calibri", "700", "calibrib.ttf"),
    ("Consolas", "400", "consola.ttf"),
    ("Consolas", "700", "consolab.ttf"),
    ("Courier New", "400", "cour.ttf"),
]

def trim(mask):
    yy, xx = np.where(mask > 0)
    if not len(xx):
        return None, None
    bounds = [int(xx.min()), int(yy.min()), int(xx.max()+1), int(yy.max()+1)]
    return mask[bounds[1]:bounds[3], bounds[0]:bounds[2]], bounds

def glyph_mask(font_path, text, size):
    font = ImageFont.truetype(str(font_path), round(size * 4))
    canvas = Image.new("L", (1600, 180), 0)
    ImageDraw.Draw(canvas).text((8, 8), text, font=font, fill=255)
    canvas = canvas.resize((400, 45), Image.Resampling.LANCZOS)
    return trim((np.asarray(canvas) > 110).astype(np.uint8))[0]

results=[]
for style, text, rect in samples:
    x,y,w,h=rect
    crop=image[y:y+h, x:x+w]
    lum=(crop[:,:,0]*0.2126+crop[:,:,1]*0.7152+crop[:,:,2]*0.0722)
    target, bb=trim((lum > 110).astype(np.uint8))
    assert bb is not None, style
    candidates=[]
    for family, weight, file in fonts:
        if style in ("code", "terminal") and family not in ("Consolas", "Courier New"):
            continue
        if style not in ("code", "terminal") and family in ("Consolas", "Courier New"):
            continue
        for step in range(44,97):
            size=step/4
            mask=glyph_mask(Path("C:/Windows/Fonts") / file, text, size)
            if mask is None: continue
            dimension_error=abs(mask.shape[1]-target.shape[1])/target.shape[1] + abs(mask.shape[0]-target.shape[0])/target.shape[0]
            if dimension_error > 0.65: continue
            normalized=cv2.resize(mask.astype(float),(target.shape[1],target.shape[0]), interpolation=cv2.INTER_AREA)>.45
            t=target.astype(bool)
            intersection=(normalized&t).sum()
            union=(normalized|t).sum()
            iou=float(intersection/union)
            score=iou-1.0*dimension_error
            candidates.append({"family":family,"weight":int(weight),"size_px_pt":size,"ink_width":mask.shape[1],"ink_height":mask.shape[0],"normalized_shape_iou":round(iou,4),"dimension_error":round(dimension_error,4),"score":round(score,4)})
    candidates.sort(key=lambda c:-c["score"])
    ink=[x+bb[0],y+bb[1],bb[2]-bb[0],bb[3]-bb[1]]
    results.append({"style":style,"sample":text,"sample_rect":rect,"measured_ink_bounds_luminance_gt_110":ink,"candidates":candidates[:3],"original_font_identity":"not recoverable from raster"})
(ROOT / "measurements/typography-fit.json").write_text(json.dumps(results,indent=2)+"\n",encoding="utf-8")
for r in results:
    print(r["style"],r["measured_ink_bounds_luminance_gt_110"],r["candidates"][0])
