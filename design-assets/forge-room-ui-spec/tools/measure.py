"""Sample the real source PNG and export reproducible measurement evidence."""
from collections import Counter
from pathlib import Path
import csv
import hashlib
import json
import math
import cv2
import numpy as np
from PIL import Image
from layout import AREAS, RECTS, TEXT, ICONS, STYLES, TREE

ROOT = Path(__file__).resolve().parents[1]
IM = Image.open(ROOT / "source/reference.png").convert("RGB")
A = np.asarray(IM)
L = cv2.cvtColor(A, cv2.COLOR_RGB2GRAY).astype(float)
DETAIL = L - cv2.GaussianBlur(L, (0,0), 2)
DETAIL32 = DETAIL.astype(np.float32)

def hx(rgb): return "#" + "".join(f"{int(c):02X}" for c in rgb)

def sample(x,y):
    patch=A[max(0,y-2):min(941,y+3),max(0,x-2):min(1672,x+3)]
    rgb=A[y,x]
    dominant,count=Counter(map(tuple,patch.reshape(-1,3))).most_common(1)[0]
    return {"xy":[x,y],"hex":hx(rgb),"rgb":[int(c) for c in rgb],"raster_alpha":1,"source_layer_alpha":None,"patch_5x5_mode":hx(dominant),"patch_5x5_min":[int(c) for c in patch.min(axis=(0,1))],"patch_5x5_max":[int(c) for c in patch.max(axis=(0,1))]}

def line_ridge(bounds,side):
    x,y,w,h=bounds
    values=[]
    if side in ("left","right"):
        seed=x if side=="left" else x+w-1
        yy0,yy1=y+min(12,h//3),y+h-min(12,h//3)
        for xx in range(max(3,seed-3),min(1669,seed+4)):
            center=L[yy0:yy1,xx]
            neighbor=(L[yy0:yy1,xx-3]+L[yy0:yy1,xx+3])/2
            values.append((xx,float(np.median(center-neighbor))))
    else:
        seed=y if side=="top" else y+h-1
        xx0,xx1=x+min(12,w//3),x+w-min(12,w//3)
        for yy in range(max(3,seed-3),min(938,seed+4)):
            center=L[yy,xx0:xx1]
            neighbor=(L[yy-3,xx0:xx1]+L[yy+3,xx0:xx1])/2
            values.append((yy,float(np.median(center-neighbor))))
    if not values: return None
    best=max(values,key=lambda v:v[1])
    above=[p for p,v in values if v>=best[1]/2] if best[1]>0 else []
    span=(max(above)-min(above)+1) if above else 0
    return {"peak_coordinate":best[0],"contrast_luma":round(best[1],3),"half_peak_pixel_band":span,"profile":[[p,round(v,3)] for p,v in values]}

def radius_fit(bounds,search):
    if search==[0,0]: return {"radius":0,"status":"square or not rounded","original_radius":None}
    x,y,w,h=bounds
    x1=x+w-1;y1=y+h-1
    candidates=[]
    for r in np.arange(search[0],min(search[1],w/2,h/2)+.01,.5):
        vals=[]
        for cx,cy,start in [(x+r,y+r,180),(x1-r,y+r,270),(x1-r,y1-r,0),(x+r,y1-r,90)]:
            for angle in np.linspace(start+8,start+82,20):
                t=math.radians(angle)
                xx=cx+r*math.cos(t);yy=cy+r*math.sin(t)
                vals.append(float(cv2.getRectSubPix(DETAIL32,(1,1),(xx,yy))[0,0]))
        candidates.append((float(r),float(np.mean(vals))))
    candidates.sort(key=lambda p:-p[1])
    return {"radius":candidates[0][0],"status":"raster ridge fit; original vector radius unknown","search_step_px_pt":.5,"search_boundary_hit":candidates[0][0] in (float(search[0]),float(search[1])),"top_candidates":[[r,round(score,4)] for r,score in candidates[:4]],"original_radius":None}

def mask_for(crop,kind):
    p=crop.astype(float)
    lum=p[:,:,0]*.2126+p[:,:,1]*.7152+p[:,:,2]*.0722
    if kind=="red": mask=(p[:,:,0]>100)&(p[:,:,0]>1.35*np.maximum(p[:,:,1],p[:,:,2]))
    elif kind=="green": mask=(p[:,:,1]>100)&(p[:,:,1]>1.15*np.maximum(p[:,:,0],p[:,:,2]))
    elif kind=="blue": mask=(p[:,:,2]>120)&(p[:,:,2]>p[:,:,0]*1.12)
    elif kind=="line-number": mask=lum>75
    else: mask=lum>110
    return mask.astype(np.uint8)

def ink(roi,kind="neutral"):
    x,y,w,h=roi
    crop=A[y:y+h,x:x+w]
    mask=mask_for(crop,kind)
    n,labels,stats,_=cv2.connectedComponentsWithStats(mask)
    cleaned=np.zeros(mask.shape,np.uint8)
    for i in range(1,n):
        if stats[i,cv2.CC_STAT_AREA]>=2:
            cleaned[labels==i]=1
    yy,xx=np.where(cleaned)
    if not len(xx): return None, None, cleaned
    bbox=[x+int(xx.min()),y+int(yy.min()),int(xx.max()-xx.min()+1),int(yy.max()-yy.min()+1)]
    pixels=crop[cleaned.astype(bool)]
    luma=pixels[:,0]*.2126+pixels[:,1]*.7152+pixels[:,2]*.0722
    index=int(np.argsort(luma)[round((len(luma)-1)*.9)])
    # The representative is an actual foreground pixel, with its exact source coordinate.
    representative=sample(int(x+xx[index]),int(y+yy[index]))
    return bbox,representative,cleaned

def paths_from_mask(mask):
    enlarged=cv2.resize(mask,None,fx=4,fy=4,interpolation=cv2.INTER_NEAREST)
    contours,_=cv2.findContours(enlarged,cv2.RETR_TREE,cv2.CHAIN_APPROX_SIMPLE)
    paths=[]
    for contour in contours:
        if len(contour)<3: continue
        poly=cv2.approxPolyDP(contour,1.4,True).reshape(-1,2)
        if len(poly)<3: continue
        # Integer pixels represent unit squares; use half-pixel contour placement.
        commands=[f"M{(poly[0,0]+.5)/4:g} {(poly[0,1]+.5)/4:g}"]
        commands += [f"L{(p[0]+.5)/4:g} {(p[1]+.5)/4:g}" for p in poly[1:]]
        paths.append(" ".join(commands)+"Z")
    return " ".join(paths)

def main():
    rects=[]
    for source in RECTS:
        item=source.copy();x,y,w,h=item["bounds"]
        point=item.pop("sample") or (x+min(5,w-1),y+min(5,h-1))
        item["background"]=sample(*point) if item["fill"] else None
        item["radius_fit"]=radius_fit(item["bounds"],item["radius_search"])
        item["edge_ridges"]={side:line_ridge(item["bounds"],side) for side in ("left","right","top","bottom")} if item["border"] else {}
        item["border_sample"]=sample(x+w//2,y) if item["border"] else None
        item["bounds_status"]="observed contour selection; ridge profiles retained; allow 1 px for antialiasing, 2 px at soft boundaries"
        rects.append(item)
    texts=[]
    for source in TEXT:
        item=source.copy()
        style=item["style"]
        kind="green" if style=="diff-green" else "red" if style in ("diff-red","badge-red","line-number-active") else "blue" if style in ("terminal-link","badge-blue") else "line-number" if style=="line-number" else "neutral"
        bbox,color,_=ink(item["roi"],kind)
        item.update(ink_bounds=bbox,foreground=color,mask_kind=kind)
        item["font_reconstruction"]={"family":STYLES[style][0],"size_px_pt":STYLES[style][1],"weight":STYLES[style][2],"original_family_size_weight":"unrecoverable from PNG; these are explicit reconstruction choices"}
        texts.append(item)
    icons=[]
    for source in ICONS:
        item=source.copy();x,y,w,h=item["roi"]
        bbox,color,_=ink(item["roi"],"neutral" if item["color"]=="multi" else item["color"])
        layers=[]
        for kind in (["neutral","red","green"] if item["color"]=="multi" else [item["color"]]):
            b,c,m=ink(item["roi"],kind)
            if c:
                d=paths_from_mask(m)
                if d: layers.append({"color":c["hex"],"source_sample_xy":c["xy"],"path":d,"threshold":kind})
        if item["color"]=="multi" and layers:
            occupied=[]
            for kind in ["neutral","red","green"]:
                b,c,m=ink(item["roi"],kind)
                if b: occupied.append(b)
            if occupied:
                lx=min(b[0] for b in occupied);ty=min(b[1] for b in occupied)
                rx=max(b[0]+b[2] for b in occupied);by=max(b[1]+b[3] for b in occupied)
                bbox=[lx,ty,rx-lx,by-ty]
        item.update(ink_bounds=bbox,foreground=color,vector_layers=layers)
        icons.append(item)
    counts=Counter(IM.getdata())
    out=ROOT / "measurements"
    out.mkdir(exist_ok=True)
    area_map=np.zeros((941,1672),np.uint8)
    for index,(_,_,(x,y,w,h)) in enumerate(AREAS,1): area_map[y:y+h,x:x+w]=index
    locations={}
    for area in range(10):
        colors=np.unique(A[area_map==area],axis=0)
        for rgb in colors: locations.setdefault(tuple(rgb),[]).append("desktop" if area==0 else AREAS[area-1][0])
    with (out / "all-colors.csv").open("w",encoding="utf-8",newline="") as handle:
        writer=csv.writer(handle);writer.writerow(["hex","red","green","blue","pixel_count","raster_alpha","observed_in"])
        for rgb,n in sorted(counts.items(),key=lambda p:(-p[1],p[0])):writer.writerow([hx(rgb),*rgb,n,1,";".join(locations[rgb])])
    excess=A[:,:,0].astype(float)-(A[:,:,1].astype(float)+A[:,:,2].astype(float))/2
    mask=((excess>12)&(area_map==0)).astype(np.uint8)
    n,labels,stats,centroids=cv2.connectedComponentsWithStats(mask)
    warm=[]
    for i in range(1,n):
        if stats[i,4]<50: continue
        yy,xx=np.where(labels==i);peak=int(np.argmax(excess[yy,xx]))
        warm.append({"bounds_px_pt":[int(v) for v in stats[i,:4]],"area_pixels":int(stats[i,4]),"centroid_px_pt":[round(float(v),3) for v in centroids[i]],"peak":sample(int(xx[peak]),int(yy[peak])),"radius":None,"original_opacity":None})
    (out/"warm-fields.json").write_text(json.dumps({"criterion":"outside window, R-(G+B)/2 > 12; 8-connected components with area >=50 pixels","components":warm,"original_glow_shader_or_blur":"not recoverable"},indent=2)+"\n",encoding="utf-8")
    result={"source":{"file":"source/reference.png","sha256":hashlib.sha256((ROOT/'source/reference.png').read_bytes()).hexdigest(),"size_px":[1672,941],"pixels_per_point_assumption":1,"unique_rgb_colors":len(counts),"alpha_channel":False},"areas":[{"id":id,"name":name,"bounds_px":list(b),"bounds_logical_pt":list(b)} for id,name,b in AREAS],"rectangles":rects,"texts":texts,"icons":icons,"styles":{name:{"family_reconstruction":v[0],"size_px_pt_reconstruction":v[1],"weight_reconstruction":v[2],"fit_sample":v[3]} for name,v in STYLES.items()}}
    (out/"measurements.json").write_text(json.dumps(result,indent=2)+"\n",encoding="utf-8")
    blanks=[p["id"] for p in texts+icons if not p["ink_bounds"]]
    print(f"Measured {len(rects)} rectangles, {len(texts)} text selections, {len(icons)} icons; exhaustive palette {len(counts)} RGB colors.")
    print("Empty ink selections:",blanks)
    print("Rounded contour fits:",[(r["id"],r["radius_fit"]["radius"]) for r in rects if r["radius_search"]!=[0,0]])

if __name__=="__main__":main()
