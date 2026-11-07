"""Verify measured evidence and deliverable integrity; record redraw differences honestly."""
from collections import Counter
from pathlib import Path
import csv
import hashlib
import json
import re
import xml.etree.ElementTree as ET
import numpy as np
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
SOURCE_SHA="4a57ade63d634e88e429d6b24971b0328d4d2575efc767992cd5e8af90356c9c"
NS="{http://www.w3.org/2000/svg}"
INKSCAPE="{http://www.inkscape.org/namespaces/inkscape}"

def main():
    source_path=ROOT/"source/reference.png"
    assert hashlib.sha256(source_path.read_bytes()).hexdigest()==SOURCE_SHA
    source=Image.open(source_path).convert("RGB")
    assert source.size==(1672,941)
    a=np.asarray(source)
    data=json.loads((ROOT/"measurements/measurements.json").read_text(encoding="utf-8"))
    assert data["source"]["sha256"]==SOURCE_SHA
    assert data["source"]["pixels_per_point_assumption"]==1
    assert len(data["areas"])==9
    assert len(data["rectangles"])==71
    assert len(data["texts"])==149
    assert len(data["icons"])==108
    for r in data["areas"]:assert r["bounds_px"]==r["bounds_logical_pt"]
    samples=0
    def verify_sample(s):
        nonlocal samples
        if s is None:return
        x,y=s["xy"];actual="#"+"".join(f"{int(c):02X}" for c in a[y,x])
        assert actual==s["hex"],(s,actual)
        assert s["source_layer_alpha"] is None
        assert s["raster_alpha"]==1
        samples+=1
    for r in data["rectangles"]:
        x,y,w,h=r["bounds"]
        assert x>=0 and y>=0 and w>0 and h>0 and x+w<=1672 and y+h<=941,r["id"]
        verify_sample(r["background"]);verify_sample(r["border_sample"])
        assert r["radius_fit"]["original_radius"] is None
    for t in data["texts"]:
        assert t["ink_bounds"] and t["foreground"],t["id"]
        x,y,w,h=t["ink_bounds"];rx,ry,rw,rh=t["roi"]
        assert x>=rx and y>=ry and x+w<=rx+rw and y+h<=ry+rh
        verify_sample(t["foreground"])
    for i in data["icons"]:
        assert i["ink_bounds"] and i["vector_layers"],i["id"]
        for layer in i["vector_layers"]:
            assert layer["path"].startswith("M") and layer["path"].endswith("Z")
            x,y=layer["source_sample_xy"]
            assert layer["color"]=="#"+"".join(f"{int(c):02X}" for c in a[y,x]),i["id"]
    palette=Counter()
    with (ROOT/"measurements/all-colors.csv").open(encoding="utf-8",newline="") as f:
        for row in csv.DictReader(f):
            rgb=tuple(int(row[c]) for c in ("red","green","blue"))
            assert row["hex"]=="#"+"".join(f"{v:02X}" for v in rgb)
            assert row["observed_in"]
            assert rgb not in palette
            palette[rgb]=int(row["pixel_count"])
    assert palette==Counter(source.getdata())
    assert len(palette)==66627 and sum(palette.values())==1672*941
    tokens=json.loads((ROOT/"tokens.json").read_text(encoding="utf-8"))
    for c in tokens["colors"]:
        x,y=c["sample_xy"]
        assert c["hex"]=="#"+"".join(f"{int(v):02X}" for v in a[y,x]),c["role"]
        assert c["original_layer_alpha"] is None
    svg_path=ROOT/"forge-room-layered.svg"
    tree=ET.parse(svg_path).getroot()
    assert tree.get("viewBox")=="0 0 1672 941"
    assert tree.get("width")=="1672" and tree.get("height")=="941"
    forbidden={"image","script","foreignObject","font","font-face"}
    elements=list(tree.iter())
    assert not any(e.tag.split("}")[-1] in forbidden for e in elements)
    assert "data:image" not in svg_path.read_text(encoding="utf-8")
    ids=[e.get("id") for e in elements if e.get("id")]
    assert len(ids)==len(set(ids)),"duplicate SVG IDs"
    assert all(item["id"] in ids for item in data["texts"]+data["icons"])
    layers=[e for e in elements if e.get(INKSCAPE+"groupmode")=="layer"]
    assert len(layers)==11
    for area in data["areas"]:assert area["id"] in ids
    for e in elements:
        for v in e.attrib.values():
            for ref in re.findall(r"url\(#([^)]*)\)",v):assert ref in ids,ref
    annotation_files=sorted((ROOT/"annotations").glob("*.png"))
    assert len(annotation_files)==11
    for path in annotation_files:
        with Image.open(path) as im:im.verify()
    with Image.open(ROOT/"redraw.png") as im:
        assert im.size==(1672,941)
        rendered=np.asarray(im.convert("RGB")).astype(float)
    with Image.open(ROOT/"redraw.png") as im:im.verify()
    delta=np.abs(a.astype(float)-rendered)
    metrics={"whole_image_rgb_mae_0_to_255":round(float(delta.mean()),4),"fraction_pixels_with_any_channel_error_gt_32":round(float((delta.max(axis=2)>32).mean()),6),"pixel_identical":bool(not delta.any()),"interpretation":"A fitted vector redraw, not original font/blur/alpha recovery or pixel-equivalent reconstruction."}
    per_area={}
    for area in data["areas"]:
        x,y,w,h=area["bounds_px"]
        per_area[area["id"]]=round(float(delta[y:y+h,x:x+w].mean()),4)
    metrics["area_rgb_mae_0_to_255"]=per_area
    # Difference image is evidence, not part of any source measurement.
    heat=np.clip(delta.max(axis=2)*3,0,255).astype(np.uint8)
    heat_rgb=np.stack((heat,np.zeros_like(heat),np.minimum(heat.astype(int)//4,255).astype(np.uint8)),axis=2)
    Image.fromarray(heat_rgb).save(ROOT/"measurements/redraw-difference.png",optimize=True)
    result={"status":"passed artifact/evidence validation; redraw is not pixel identical","source_sha256":SOURCE_SHA,"reference_size_px":[1672,941],"pixels_per_logical_point":1,"areas":9,"component_rectangles":71,"text_ink_selections":149,"icon_ink_selections":108,"exact_source_samples_checked":samples,"named_role_samples_checked":len(tokens["colors"]),"exhaustive_rgb_colors":len(palette),"palette_pixels_accounted_for":sum(palette.values()),"annotated_pngs_including_overview_and_token_board":11,"named_svg_layers":len(layers),"svg_embedded_rasters":0,"svg_embedded_fonts":0,"svg_text_nodes":sum(e.tag==NS+"text" for e in elements),"svg_paths":sum(e.tag==NS+"path" for e in elements),"redraw_comparison":metrics,"unknown_metadata_explicitly_retained":["original font family/size/weight","original source-layer alpha","source background image","source blur kernel/radius","original stroke/radius parameters","hidden states and behavior"]}
    (ROOT/"validation.json").write_text(json.dumps(result,indent=2)+"\n",encoding="utf-8",newline="\n")
    print(f"PASS: source hash, {samples} source samples + {len(tokens['colors'])} role samples, 66,627-color exhaustive palette, 71 rectangles, 149 text selections, 108 icons, 11 annotations, 11 SVG layers, no embedded images/fonts.")
    print("Redraw comparison (not pixel equivalence):",metrics)

if __name__=="__main__":main()
