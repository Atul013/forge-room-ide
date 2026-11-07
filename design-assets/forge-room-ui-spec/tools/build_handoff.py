"""Build Markdown, source-based annotations, native constants, and a vector redraw."""
from pathlib import Path
import csv
import html
import json
import math
import re
import textwrap
import xml.etree.ElementTree as ET
import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from layout import AREAS, TREE, STYLES
from notes import INTRO, AREA_NOTES, BUILD_NOTES

ROOT=Path(__file__).resolve().parents[1]
DATA=json.loads((ROOT/"measurements/measurements.json").read_text(encoding="utf-8"))
IMAGE=Image.open(ROOT/"source/reference.png").convert("RGB")
A=np.asarray(IMAGE)
RECT={r["id"]:r for r in DATA["rectangles"]}
TEXT={r["id"]:r for r in DATA["texts"]}
ICON={r["id"]:r for r in DATA["icons"]}
FONTS=Path("C:/Windows/Fonts")
COLORS=[]

def font(size,bold=False):return ImageFont.truetype(str(FONTS/("seguisb.ttf" if bold else "segoeui.ttf")),size)
def hexrgb(rgb):return "#"+"".join(f"{int(v):02X}" for v in rgb)
def bounds(b):return "("+", ".join(f"{v:g}" for v in b)+")"
def pair(v):return f"{v:g} px / {v:g} pt"
def esc(s):return str(s).replace("|","\\|").replace("\n"," ")
def token(role,source,where):
    if source:
        COLORS.append({"role":role,"hex":source["hex"],"sample_xy":source["xy"],"raster_alpha":1,"original_layer_alpha":None,"where":where})
def source_sample(x,y):return {"hex":hexrgb(A[y,x]),"xy":[x,y]}

def color_tokens():
    for role,p,where in [
        ("desktop.warm",(10,10),"Top-left exterior"),
        ("desktop.dark",(1000,925),"Bottom exterior"),
        ("frame.border",(800,22),"Top frame ridge"),
        ("divider.sidebar",(386,200),"Explorer/editor divider"),
        ("divider.dock",(1216,200),"Editor/right-dock divider"),
        ("divider.panel",(700,625),"Editor/bottom-panel divider"),
        ("surface.explorer",(270,320),"Tree background"),
        ("surface.editor",(1180,250),"Empty editor region"),
        ("surface.dock",(1230,600),"Dock gutter background"),
        ("room.active.border",(40,460),"Left red border of active Room tile"),
    ]:token(role,source_sample(*p),where)
    for id in ["window","title-band","rail-band","command-palette","room-pill","room-active-tile","selected-tree-row","scm-panel","badge-room","badge-agents","commit-button","sync-button","open-pr-button","room-tab","room-tab-underline","active-line","caret","floating-toolbar","agent-dropdown","agent-selected-row","terminal-tab","terminal-tab-underline","problem-count-pill","terminal-run-button","current-branch-card","create-pr-button","ai-tab","ai-tab-underline","quick-actions-card","quick-action-implement","user-message-bubble","edited-summary","filename-chip-room","composer","send-button","status-band","review-needed-pill"]:
        token(id+".sample",RECT[id]["background"],id)
    for style in STYLES:
        t=next((t for t in DATA["texts"] if t["style"]==style and t["foreground"]),None)
        if t:token("text."+style,t["foreground"],t["id"]+"; other instances can vary")
    for id in ["notification-dot","title-room-people","terminal-prompt-dot","status-icon-members","ai-tab-sparkle","edited-check-room"]:
        item=ICON[id]
        for index,layer in enumerate(item["vector_layers"]):
            token("icon."+id+f".{index}",{"hex":layer["color"],"xy":layer["source_sample_xy"]},id+"; representative mask foreground")
    # Syntax samples are selected from actual high-chroma code pixels, not a theme file.
    crop=A[169:605,473:1148]
    hsv=cv2.cvtColor(crop,cv2.COLOR_RGB2HSV)
    hue=hsv[:,:,0];sat=hsv[:,:,1];value=hsv[:,:,2]
    for role,low,high in [("syntax.keyword-purple",125,165),("syntax.identifier-cyan",75,105),("syntax.string-orange",7,23),("syntax.function-yellow",23,40),("syntax.tag-red",0,7),("syntax.blue",100,122)]:
        mask=(hue>=low)&(hue<high)&(sat>50)&(value>160)
        yy,xx=np.where(mask)
        if not len(xx):continue
        pixels=crop[yy,xx]
        q=(pixels//8).astype(int)
        unique,counts=np.unique(q,axis=0,return_counts=True)
        target=unique[int(np.argmax(counts))]*8+4
        i=int(np.argmin(((pixels.astype(float)-target)**2).sum(axis=1)))
        token(role,source_sample(int(xx[i])+473,int(yy[i])+169),"Editor source glyphs; sampled color cluster")
    mask=(sat<35)&(value>170)
    yy,xx=np.where(mask)
    if len(xx):
        i=int(np.argsort(crop[yy,xx].mean(axis=1))[round((len(xx)-1)*.65)])
        token("syntax.punctuation",source_sample(int(xx[i])+473,int(yy[i])+169),"Low-chroma editor source glyphs")
    for role,roi in [("terminal.ready-green",(442,767,94,17)),("terminal.ready-text",(555,767,130,17))]:
        x,y,w,h=roi;pixels=A[y:y+h,x:x+w];lum=pixels.mean(axis=2)
        mask=(pixels[:,:,1]>pixels[:,:,2]*1.2)&(lum>110) if "green" in role else lum>160
        yy,xx=np.where(mask)
        if len(xx):
            i=len(xx)//2;token(role,source_sample(int(xx[i])+x,int(yy[i])+y),"Terminal ready line")
    (ROOT/"tokens.json").write_text(json.dumps({"provenance":"M = observed composite; R = reconstruction; original alpha/font metadata unavailable","scale":{"physical_px_per_logical_pt":1},"colors":COLORS,"typography":DATA["styles"]},indent=2)+"\n",encoding="utf-8")
    with (ROOT/"measurements/color-roles.csv").open("w",encoding="utf-8",newline="") as f:
        writer=csv.writer(f);writer.writerow(["role","observed_hex","sample_x","sample_y","raster_alpha","source_layer_alpha","where"])
        for c in COLORS:writer.writerow([c["role"],c["hex"],*c["sample_xy"],1,"unrecoverable",c["where"]])

def make_markdown():
    rows=[INTRO,"\n## Primary measured envelopes\n", "| Area | Screenshot bounds, px (M) | Logical bounds at 100%, pt | Window-local logical bounds, pt |", "|---|---|---|---|"]
    for id,name,b in AREAS:
        local=[b[0]-28,b[1]-22,b[2],b[3]]
        rows.append(f"| {name} | {bounds(b)} | {bounds(b)} | {bounds(local)} |")
    rows += ["\nShared divider evidence: the source has bright/antialiased bands near x=104–105,385–386,1215–1218 px / pt and y=75–76,124–125,624–626,668–669,861–862 px / pt. Keep the sampled ridge widths separate from an assumed original one-pixel stroke.\n"]
    for area,(id,name,_) in enumerate(AREAS,1):
        rows += [f"\n## {area}. {name}\n", f"[Annotated PNG](annotations/{id}.png)\n", AREA_NOTES[area]]
        rr=[r for r in DATA["rectangles"] if r["area"]==area]
        if rr:
            rows += ["\n### Container, border, and content-offset measurements\n", "Color samples below are opaque composite pixels (raster alpha 1); **original layer alpha is U for every row**. Radius is F, not a recovered original token. A width band reports half-peak ridge pixels (L/R/T/B), not the original stroke parameter. The SVG uses a disclosed R outline width of **1 px / 1 pt** for selected bordered containers; this is not asserted as the original width. Content offsets are observations/selections and are not proof of internal widget padding.\n", "| Element | Bounds px (M) | Bounds logical pt | Background composite (M) | Border composite / ridge band px = pt (M) | Fitted radius px / pt (F) | Visible content offsets / notes |", "|---|---|---|---|---|---|---|"]
            for r in rr:
                bg=r["background"]["hex"] if r["background"] else "no separate fill selected"
                if r["border"]:
                    band="/".join(str(r["edge_ridges"][side]["half_peak_pixel_band"]) for side in ("left","right","top","bottom"))
                    border=f'{r["border_sample"]["hex"]}; {band} px = {band} pt'
                else:border="no distinct outline selected; U source width"
                fit=r["radius_fit"]
                radius=pair(fit["radius"])+( "; boundary-limited" if fit.get("search_boundary_hit") else "")
                rows.append(f'| `{r["id"]}` | {bounds(r["bounds"])} | {bounds(r["bounds"])} | {bg} | {border} | {radius} | {esc(r["padding"]+". "+r["note"])} |')
        tt=[t for t in DATA["texts"] if t["area"]==area]
        if tt:
            rows += ["\n### Text instances\n", "Use the style key in the typography table for the disclosed reconstruction family/size/weight. These bounds are thresholded **ink**, not text-layout allocation. Every foreground hex is an actual source sample; colors can differ slightly between instances. Mask thresholds and sample coordinates are in measurements.json.\n", "| Text / instance | Style | Ink bounds px (M) | Ink bounds logical pt | Foreground composite (M) |", "|---|---|---|---|---|"]
            for t in tt:
                rows.append(f'| {esc(t["content"])} · `{t["id"]}` | `{t["style"]}` | {bounds(t["ink_bounds"])} | {bounds(t["ink_bounds"])} | {t["foreground"]["hex"]} |')
        ii=[i for i in DATA["icons"] if i["area"]==area]
        if ii:
            rows += ["\n### Icon instances\n", "These are painted-mask sizes. Original icon viewBoxes, exact source stroke widths, and hit targets are U. SVG paths are traced from the selected masks at subpixel contour placement; the original vectors are not recovered.\n", "| Icon | Painted bounds px (M) | Painted bounds logical pt | Painted size px / pt | Foreground composite color(s) | Notes |", "|---|---|---|---|---|---|"]
            for i in ii:
                b=i["ink_bounds"];cs=", ".join(dict.fromkeys(l["color"] for l in i["vector_layers"]))
                rows.append(f'| `{i["id"]}` | {bounds(b)} | {bounds(b)} | {b[2]}×{b[3]} px / {b[2]}×{b[3]} pt | {cs} | {esc(i["note"])} |')
        if area==4:
            rows += ["\n### Tree anchors and row-center cadence\n", "| Item | Depth label | Selected center y px | Center y logical pt | Label anchor x px | Label anchor x logical pt | Marker |", "|---|---|---|---|---|---|---|"]
            for label,level,cy,tx,kind,chev in TREE:rows.append(f"| {label} | {level} | {cy} | {cy} | {tx} | {tx} | {kind or 'none'}; {chev or 'no chevron'} |")
    rows += ["\n## 10. Color table, typography, and spacing\n", "[Annotated token board](annotations/10-colors-typography-spacing.png)\n",f"The image contains **{DATA['source']['unique_rgb_colors']:,} distinct RGB values**. The complete table is [all-colors.csv](measurements/all-colors.csv), including pixel counts and every observed area. A useful implementation palette is below. These role names are assigned to observed samples; they do not imply that the original image had one uniform token per role. Source-layer opacity is U for every color; stored raster alpha is 1.\n", "| Role | Observed hex (M) | Sample (x,y), px | Sample (x,y), logical pt | Where |", "|---|---|---|---|---|"]
    for c in COLORS:rows.append(f'| `{c["role"]}` | {c["hex"]} | {bounds(c["sample_xy"])} | {bounds(c["sample_xy"])} | {esc(c["where"])} |')
    rows += ["\n### Typography table\n", "**Original family, nominal font size, and weight are U for all styles.** The following native/SVG values are R: practical reconstruction settings, not measurements masquerading as original tokens. Per-instance ink bounds and colors above remain M. The SVG additionally fits each text run to its measured ink envelope.\n", "| Style | Family (R) | Size px (R) | Size logical pt (R) | Weight (R) | Observed ink-height range px / pt (M) | Color evidence |", "|---|---|---|---|---|---|---|"]
    for name,s in DATA["styles"].items():
        instances=[t for t in DATA["texts"] if t["style"]==name]
        heights=[t["ink_bounds"][3] for t in instances]
        colors=", ".join(dict.fromkeys(t["foreground"]["hex"] for t in instances))
        if heights:rows.append(f'| `{name}` | {s["family_reconstruction"]} | {s["size_px_pt_reconstruction"]:g} | {s["size_px_pt_reconstruction"]:g} | {s["weight_reconstruction"]} | {min(heights)}–{max(heights)} px / {min(heights)}–{max(heights)} pt | {colors} |')
    fits=json.loads((ROOT/"measurements/typography-fit.json").read_text())
    rows += ["\n### Font comparison evidence (F, not identification)\n", "Local Segoe UI, Arial, Calibri, Consolas and Courier New files were compared at 0.25 px / pt size steps. Binary glyph masks use luminance >110; candidates are ranked by normalized shape IoU minus relative width/height error. Even the best candidate is not a recovered font. In particular the Explorer-heading sample's best candidate differs from the reconstruction choice, and short samples do not uniquely identify a family.\n", "| Sample | Measured ink w×h px / pt | Best local candidate (F) | Candidate size px / pt | Normalized shape IoU | Relative size error |", "|---|---|---|---|---|---|"]
    for f in fits:
        c=f["candidates"][0];b=f["measured_ink_bounds_luminance_gt_110"]
        rows.append(f'| {f["style"]}: {f["sample"]} | {b[2]}×{b[3]} px / {b[2]}×{b[3]} pt | {c["family"]} {c["weight"]} | {pair(c["size_px_pt"])} | {c["normalized_shape_iou"]:.4f} | {c["dimension_error"]:.4f} |')
    rows += ["\n### Measured spacing inventory\n", "This is an inventory of observed distances, not an invented 4/8-point design system. Original spacing tokens and flex/layout constraints are U.\n", "| Observation | px (M) | logical pt | Evidence |", "|---|---|---|---|"]
    spacing=[("Frame left/top/right/bottom margins","28 / 22 / 24 / 37","source vs frame envelope"),("Rail width","76","rail envelope"),("Sidebar width","281","Explorer/SCM envelope"),("Editor width","832","editor envelope"),("Right dock width","428","dock envelope"),("Title band / editor tab band / bottom tab band","54 / 49 / 44","selected band envelopes"),("Selected tree row height","26","selected-tree-row"),("Code row-center deltas","21 then 20","line 1→2, then following rows"),("Quick-action column / row gaps","11 / 6","button envelopes"),("Commit→Sync / Sync→Open PR gaps","9 / 9","button envelopes"),("Toolbar horizontal button gaps","10 / 11 / 9 / 11","individual button selections"),("Popup left inset / selected row height","6 / 27","agent popup and selected row"),("Active underline bright core height","3","room-tab, Terminal, AI tab"),("Selected Room tile w×h","56 × 46","active rail envelope"),("Composer send button w×h","42 × 36","send-button envelope")]
    rail=[i for i in DATA["icons"] if i["area"]==3]
    centers=[(i["ink_bounds"][0]+i["ink_bounds"][2]/2,i["ink_bounds"][1]+i["ink_bounds"][3]/2) for i in rail]
    gaps=[round(centers[i+1][1]-centers[i][1],2) for i in range(7)]
    spacing.append(("Rail painted-center vertical deltas"," / ".join(f"{g:g}" for g in gaps),"eight isolated ink envelopes"))
    spacing.append(("Tree selected-center deltas"," / ".join(str(TREE[i+1][2]-TREE[i][2]) for i in range(len(TREE)-1)),"tree anchors above"))
    for label,value,evidence in spacing:rows.append(f"| {label} | {value} | {value} | {evidence} |")
    rows += ["\n### Exterior warm-field measurements\n", "Criterion: outside the framed window, R − (G+B)/2 >12, 8-connected regions of at least 50 pixels. The visible field can be truncated by the window. No source glow center/radius, shader color, or alpha is inferred from these regions.\n", "| Visible red-excess region | Bounds px | Bounds logical pt | Peak observed RGB | Peak position px / pt | Original radius / opacity |", "|---|---|---|---|---|---|"]
    warm=json.loads((ROOT/"measurements/warm-fields.json").read_text())
    for n,c in enumerate(warm["components"],1):rows.append(f'| {n} | {bounds(c["bounds_px_pt"])} | {bounds(c["bounds_px_pt"])} | {c["peak"]["hex"]} | {bounds(c["peak"]["xy"])} / {bounds(c["peak"]["xy"])} | U / U |')
    rows += [BUILD_NOTES,f"\n## Source integrity\n\nSource SHA-256: `{DATA['source']['sha256']}`. The stored copy has no PNG metadata. All measurements use that exact file, not an upscaled chat preview. See validation.json for current artifact checks and renderer-comparison limits.\n"]
    (ROOT/"SPEC.md").write_text("\n".join(rows)+"\n",encoding="utf-8",newline="\n")

def write_wrapped(draw,xy,text,font_value,fill,width,line=18):
    x,y=xy
    words=text.split();current="";lines=[]
    for word in words:
        trial=(current+" "+word).strip()
        if draw.textlength(trial,font=font_value)>width and current:lines.append(current);current=word
        else:current=trial
    if current:lines.append(current)
    for s in lines:draw.text((x,y),s,font=font_value,fill=fill);y+=line
    return y

def annotate():
    out=ROOT/"annotations";out.mkdir(exist_ok=True)
    overview=IMAGE.copy();d=ImageDraw.Draw(overview)
    accents=["#FFC857","#4EC9FF","#FF7E9B","#87E2A3","#C99CFF","#4EC9FF","#FFC857","#87E2A3","#FF7E9B"]
    for n,((id,name,b),accent) in enumerate(zip(AREAS,accents),1):
        x,y,w,h=b;d.rectangle((x,y,x+w-1,y+h-1),outline=accent,width=2)
        label=f"{n}. {name}  {bounds(b)} px = pt"
        tw=d.textlength(label,font=font(13,True))+12
        ly=y+3 if n not in (3,4,5) else y+4
        d.rectangle((x+3,ly,x+3+tw,ly+24),fill="#11151D")
        d.text((x+9,ly+2),label,font=font(13,True),fill=accent)
    overview.save(out/"00-overview.png")
    extra={6:{"code-line-01","line-number-14","code-line-14","agent-name-builder"},7:{"branch-card-name","terminal-local-url"},8:{"composer-model","user-message-line-1","edited-plus-room","edited-minus-room"},9:{"status-branch","status-sync","status-pull-request","status-room","status-members","status-position","status-encoding","status-line-ending","status-model","status-home"}}
    for area,(id,name,b) in enumerate(AREAS,1):
        selected=[]
        for category,group in [("rect",DATA["rectangles"]),("text",DATA["texts"]),("icon",DATA["icons"])]:
            for item in group:
                if item["area"]!=area:continue
                use=item.get("annotate",False) or item["id"] in extra.get(area,set())
                if area==9:use=True
                if not use:continue
                bb=item.get("bounds") if category=="rect" else item["ink_bounds"]
                if bb:selected.append((category,item,bb))
        if area==1:
            warm=json.loads((ROOT/"measurements/warm-fields.json").read_text())
            for n,c in enumerate(warm["components"],1):
                selected.append(("warm",{"id":f"exterior-red-excess-{n}","foreground":c["peak"]},c["bounds_px_pt"]))
        # Frame sheet retains the surrounding desktop, because its blur/glow live outside the frame.
        cropbox=(0,0,1672,941) if area==1 else (max(0,b[0]-3),max(0,b[1]-3),min(1672,b[0]+b[2]+3),min(941,b[1]+b[3]+3))
        crop=IMAGE.crop(cropbox)
        legend_w=620;left=28;top=110
        legend_x=left+crop.width+28
        width=max(1200,legend_x+legend_w+28)
        row_height=76
        height=max(top+crop.height+60,top+len(selected)*row_height+55)
        sheet=Image.new("RGB",(width,height),"#151922");d=ImageDraw.Draw(sheet)
        d.text((28,20),f"{area:02d}  {name}",font=font(27,True),fill="#F1F3F7")
        d.text((28,58),"Source crop at 1:1. Global coordinates; right/bottom excluded. 1 px = 1 logical pt at 100%.",font=font(16),fill="#B8C5D8")
        sheet.paste(crop,(left,top))
        for n,(category,item,bb) in enumerate(selected,1):
            accent=accents[(n-1)%len(accents)]
            x,y,w,h=bb
            lx=left+x-cropbox[0];ly=top+y-cropbox[1]
            d.rectangle((lx,ly,lx+max(1,w)-1,ly+max(1,h)-1),outline=accent,width=1)
            # A numbered marker links source geometry to the full-size legend.
            mx=max(left+8,min(left+crop.width-10,lx+8));my=max(top+8,min(top+crop.height-10,ly+8))
            d.ellipse((mx-9,my-9,mx+9,my+9),fill="#11151D",outline=accent)
            d.text((mx,my),str(n),font=font(10,True),fill=accent,anchor="mm")
            ry=top+(n-1)*row_height
            d.text((legend_x,ry),f"{n:02d}  {item['id']}",font=font(16,True),fill=accent)
            d.text((legend_x,ry+23),f"{bounds(bb)} px  /  {bounds(bb)} pt",font=font(14),fill="#EEF1F7")
            if category=="rect":
                bg=item["background"]["hex"] if item["background"] else "no selected fill"
                summary=f"BG {bg}; radius fit {item['radius_fit']['radius']:g}px/{item['radius_fit']['radius']:g}pt; alpha U"
            elif category=="text":summary=f"{item['style']}; ink {item['foreground']['hex']}; original font U"
            elif category=="icon":summary="ink "+", ".join(dict.fromkeys(l["color"] for l in item["vector_layers"]))+"; original stroke U"
            else:summary=f"peak {item['foreground']['hex']}; threshold R-(G+B)/2>12; radius/alpha U"
            d.text((legend_x,ry+45),summary,font=font(13),fill="#B8C5D8")
        d.text((28,height-30),"M: source observations. F: fitted corner contours. U: unrecoverable source metadata. Full values and uncertainty: SPEC.md.",font=font(14),fill="#B8C5D8")
        sheet.save(out/f"{id}.png",optimize=True)
    token_board(out)

def token_board(out):
    columns=4;cell=96;rows=math.ceil(len(COLORS)/columns)
    height=180+rows*cell+490
    sheet=Image.new("RGB",(1672,height),"#151922");d=ImageDraw.Draw(sheet)
    d.text((36,26),"10  Colors, typography & spacing",font=font(29,True),fill="#F1F3F7")
    d.text((36,73),"Actual composite RGB samples. Original alpha and font metadata are unavailable.",font=font(18),fill="#B8C5D8")
    d.text((36,105),"66,627 distinct RGB colors: full palette + locations in measurements/all-colors.csv.",font=font(16),fill="#B8C5D8")
    for n,c in enumerate(COLORS):
        row,col=divmod(n,columns);x=36+col*405;y=155+row*cell
        d.rounded_rectangle((x,y+5,x+50,y+55),radius=5,fill=c["hex"],outline="#8491A4")
        d.text((x+62,y+2),c["role"][:38],font=font(13,True),fill="#F1F3F7")
        d.text((x+62,y+25),c["hex"],font=font(17,True),fill="#F1F3F7")
        d.text((x+62,y+51),f"sample {bounds(c['sample_xy'])} px = pt",font=font(12),fill="#B8C5D8")
    y=175+rows*cell
    d.text((36,y),"Typography: measured ink height and disclosed reconstruction settings",font=font(22,True),fill="#F1F3F7")
    fits=json.loads((ROOT/"measurements/typography-fit.json").read_text())
    for n,f in enumerate(fits):
        col=n//6;row=n%6;x=36+col*810;yy=y+48+row*45
        c=f["candidates"][0];b=f["measured_ink_bounds_luminance_gt_110"]
        label=f"{f['style']}: {b[3]}px/{b[3]}pt ink; fit {c['family']} {c['size_px_pt']:g}px/pt, {c['weight']}"
        d.text((x,yy),label,font=font(15),fill="#EEF1F7")
        d.text((x,yy+21),f"Shape IoU {c['normalized_shape_iou']:.3f}; candidate, not font identification.",font=font(12),fill="#B8C5D8")
    yy=y+350
    d.text((36,yy),"Measured spacing",font=font(21,True),fill="#F1F3F7")
    d.text((36,yy+36),"3px/pt underline · 6px/pt popup inset · 11px/pt quick-action gap · 20px/pt code cadence",font=font(17),fill="#EEF1F7")
    d.text((36,yy+67),"Tree rows and rail centers vary. These are observations, not a recovered 4/8-point scale.",font=font(16),fill="#B8C5D8")
    sheet.save(out/"10-colors-typography-spacing.png",optimize=True)

def fnum(v):return f"{float(v):g}"

def svg_text(item):
    x,y,w,h=item["ink_bounds"]
    spec=item["font_reconstruction"];family=spec["family"];size=spec["size_px_pt"];weight=spec["weight"]
    file="consolab.ttf" if family=="Consolas" and weight>=600 else "consola.ttf" if family=="Consolas" else "seguisb.ttf" if weight>=600 else "segoeui.ttf"
    ff=ImageFont.truetype(str(FONTS/file),round(size*4))
    box=ff.getbbox(item["content"],anchor="ls");bx,by,bw,bh=box[0]/4,box[1]/4,(box[2]-box[0])/4,(box[3]-box[1])/4
    sx=w/max(.1,bw);sy=h/max(.1,bh)
    color=item["foreground"]["hex"]
    attr=f'font-family="{family}" font-size="{size:g}" font-weight="{weight}" fill="{color}" xml:space="preserve"'
    content=html.escape(item["content"])
    rolecolor={c["role"]:c["hex"] for c in COLORS}
    if item["style"]=="code":
        parts=re.split(r"('[^']*'|\"[^\"]*\"|\b\w+\b)",item["content"])
        spans=[]
        for p in parts:
            if not p:continue
            if p.startswith(("'",'"')):role="syntax.function-yellow" if "builder" in p else "syntax.string-orange"
            elif p in ("import","from","export","const","return"):role="syntax.keyword-purple"
            elif p=="function":role="syntax.blue"
            elif p in ("Room","useRoom","useAgents","startSession","openInvite"):role="syntax.function-yellow"
            elif p=="div":role="syntax.tag-red"
            elif re.fullmatch(r"\w+",p):role="syntax.identifier-cyan"
            else:role="syntax.punctuation"
            spans.append(f'<tspan fill="{rolecolor.get(role,color)}">{html.escape(p)}</tspan>')
        content="".join(spans)
    elif item["id"]=="terminal-prompt":
        cyan=rolecolor.get("syntax.identifier-cyan",color);red=rolecolor.get("syntax.tag-red",color);yellow=rolecolor.get("syntax.function-yellow",color)
        white=rolecolor.get("syntax.punctuation",color)
        content=f'<tspan fill="{cyan}">forge-room-app git:(</tspan><tspan fill="{red}">feature/room-collab</tspan><tspan fill="{cyan}">)</tspan><tspan fill="{yellow}"> ✗ </tspan><tspan fill="{white}">npm run dev</tspan>'
    elif item["id"]=="terminal-ready":
        green=rolecolor.get("terminal.ready-green",color);white=rolecolor.get("terminal.ready-text",color)
        content=f'<tspan fill="{green}">VITE v5.4.2</tspan><tspan fill="{white}">   ready in 412 ms</tspan>'
    decoration=' text-decoration="underline"' if item["style"]=="terminal-link" else ""
    return f'<g id="{item["id"]}" data-evidence="M ink-envelope; R font" transform="translate({x} {y}) scale({sx:.6f} {sy:.6f})"><text x="{-bx:g}" y="{-by:g}" {attr}{decoration}>{content}</text></g>'

def svg_rect(item):
    x,y,w,h=item["bounds"]
    if not item["fill"] and not item["border"]:return ""
    color=item["background"]["hex"] if item["background"] else "none"
    if item["id"] in ("commit-button","send-button","room-active-tile","agent-selected-row"):
        color=f'url(#{item["id"]}-sampled-fill)'
    r=item["radius_fit"]["radius"]
    stroke=f' stroke="{item["border_sample"]["hex"]}" stroke-width="1"' if item["border"] else ""
    # One-pixel reconstruction outlines are deliberate settings; ridge evidence is in the spec.
    return f'<rect id="{item["id"]}" x="{x}" y="{y}" width="{w}" height="{h}" rx="{r:g}" fill="{color}"{stroke} data-evidence="M envelope/sample; F radius; R border"/>'

def desktop_svg():
    definitions=[];rectangles=[]
    # Sample only visible exterior pixels. Several strips preserve a 2D blurred color field.
    strips=[]
    for y0,y1 in [(0,8),(8,16),(16,22),(904,916),(916,928),(928,941)]:strips.append(("horizontal",0,y0,1672,y1-y0))
    for x0,x1 in [(0,7),(7,14),(14,21),(21,28),(1648,1654),(1654,1660),(1660,1666),(1666,1672)]:strips.append(("vertical",x0,22,x1-x0,882))
    for index,(direction,x,y,w,h) in enumerate(strips):
        id=f"desktop-sampled-field-{index}"
        stops=[]
        count=55 if direction=="horizontal" else 34
        for n in range(count):
            t=n/(count-1)
            xx=round(x+t*(w-1)) if direction=="horizontal" else x+w//2
            yy=y+h//2 if direction=="horizontal" else round(y+t*(h-1))
            patch=A[max(0,yy-1):min(941,yy+2),max(0,xx-1):min(1672,xx+2)]
            color=hexrgb(np.median(patch,axis=(0,1)))
            stops.append(f'<stop offset="{t:.5f}" stop-color="{color}"/>')
        end='x2="1" y2="0"' if direction=="horizontal" else 'x2="0" y2="1"'
        definitions.append(f'<linearGradient id="{id}" x1="0" y1="0" {end}>{"".join(stops)}</linearGradient>')
        rectangles.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="url(#{id})"/>')
    return definitions,rectangles

def layered_svg():
    defs,desktop=desktop_svg()
    for id in ("commit-button","send-button","room-active-tile","agent-selected-row"):
        item=RECT[id];x,y,w,h=item["bounds"]
        yy=y+4 if id in ("room-active-tile","agent-selected-row") else y+h-5
        stops=[]
        for n in range(9):
            t=n/8;xx=round(x+2+t*(w-5));color=hexrgb(A[yy,xx])
            stops.append(f'<stop offset="{t:g}" stop-color="{color}"/>')
        defs.append(f'<linearGradient id="{id}-sampled-fill" x1="0" y1="0" x2="1" y2="0">'+"".join(stops)+'</linearGradient>')
    glow_fields={6:(391,115,151,17,470),7:(406,660,80,17,450),8:(1227,115,149,17,1320)}
    for area,(x,y,w,h,sample_x) in glow_fields.items():
        stops=[]
        for dy in range(h):stops.append(f'<stop offset="{dy/(h-1):.6f}" stop-color="{hexrgb(A[y+dy,sample_x])}"/>')
        defs.append(f'<linearGradient id="underline-observed-field-{area}" x1="0" y1="0" x2="0" y2="1">'+"".join(stops)+'</linearGradient>')
    radius=RECT["window"]["radius_fit"]["radius"]
    defs.append(f'<clipPath id="window-clip"><rect x="28" y="22" width="1620" height="882" rx="{radius:g}"/></clipPath>')
    layers=['<g id="00-desktop-field" inkscape:groupmode="layer" inkscape:label="Desktop field" data-evidence="R interpolation of M exterior RGB samples"><rect width="1672" height="941" fill="'+hexrgb(A[925,1000])+'"/>',*desktop,'</g>']
    frame=svg_rect(RECT["window"])
    layers.append('<g id="01-window-frame" inkscape:groupmode="layer" inkscape:label="Window frame">'+frame+'</g>')
    layers.append('<g id="application-content" inkscape:groupmode="layer" inkscape:label="Application content" clip-path="url(#window-clip)">')
    bgpoints={2:(1050,50),3:(40,300),4:(270,320),5:(120,710),6:(1180,250),7:(800,840),8:(1230,600),9:(1100,892)}
    for area,(id,name,b) in enumerate(AREAS,1):
        if area==1:continue
        x,y,w,h=b
        bg=hexrgb(A[bgpoints[area][1],bgpoints[area][0]])
        layers.append(f'<g id="{id}" inkscape:groupmode="layer" inkscape:label="{html.escape(name)}" data-name="{html.escape(name)}">')
        layers.append(f'<rect id="{id}-background" x="{x}" y="{y}" width="{w}" height="{h}" fill="{bg}"/>')
        for item in DATA["rectangles"]:
            if item["area"]==area:layers.append(svg_rect(item))
        if area in glow_fields:
            gx,gy,gw,gh,_=glow_fields[area]
            layers.append(f'<rect id="active-underline-observed-field-{area}" x="{gx}" y="{gy}" width="{gw}" height="{gh}" fill="url(#underline-observed-field-{area})" data-evidence="R interpolation of measured vertical RGB profile"/>')
        if area==4:
            for gx,y0,y1 in [(143,175,516),(160,201,451),(177,225,425),(195,250,350)]:layers.append(f'<path d="M{gx} {y0}V{y1}" fill="none" stroke="{hexrgb(A[y0+2,gx])}" stroke-width="1"/>')
        if area==6:
            for gx,y0,y1 in [(475,266,597),(495,287,580),(516,347,560),(536,368,538),(556,389,520)]:layers.append(f'<path d="M{gx} {y0}V{y1}" fill="none" stroke="{hexrgb(A[min(605,y0+7),gx])}" stroke-width="1"/>')
            layers.append('<path id="agent-popup-pointer" d="M908 452 916 446 924 452Z" fill="'+hexrgb(A[451,916])+'" data-evidence="R contour interpolation"/>')
        if area==8:
            layers.append('<path id="user-bubble-pointer" d="M1286 396 1277 403 1286 409Z" fill="'+hexrgb(A[403,1285])+'" data-evidence="R contour interpolation"/>')
            layers.append('<circle id="avatar-background" cx="1257" cy="402" r="17" fill="'+hexrgb(A[400,1243])+'"/>')
        for item in DATA["texts"]:
            if item["area"]==area:layers.append(svg_text(item))
        for item in DATA["icons"]:
            if item["area"]!=area:continue
            x,y,w,h=item["roi"]
            paths="".join(f'<path d="{layer["path"]}" fill="{layer["color"]}" fill-rule="evenodd"/>' for layer in item["vector_layers"])
            layers.append(f'<g id="{item["id"]}" transform="translate({x} {y})" data-evidence="M threshold-mask trace">{paths}</g>')
        layers.append('</g>')
    # Shared separators were measured from the source's multi-pixel ridge bands.
    layers.append('<g id="shared-divider-ridges" fill="'+hexrgb(A[625,700])+'"><rect x="104" y="77" width="2" height="784"/><rect x="385" y="77" width="2" height="784"/><rect x="1216" y="77" width="2" height="784"/><rect x="29" y="75" width="1618" height="2"/><rect x="546" y="124" width="676" height="2"/><rect x="1380" y="124" width="267" height="2"/><rect x="387" y="624" width="832" height="2"/><rect x="387" y="668" width="832" height="2"/><rect x="29" y="861" width="1618" height="1"/></g>')
    layers.append('</g>')
    svg='<?xml version="1.0" encoding="UTF-8"?>\n<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="1672" height="941" viewBox="0 0 1672 941">\n<title>Forge Room IDE — measured vector reconstruction</title>\n<desc>Observed raster geometry and sampled composite colors; reconstructed fonts and contours. Original font metadata, blur, opacity and vector paths are not recoverable from this PNG. See SPEC.md.</desc>\n<defs>'+"\n".join(defs)+'</defs>\n'+"\n".join(layers)+'\n</svg>\n'
    (ROOT/"forge-room-layered.svg").write_text(svg,encoding="utf-8",newline="\n")

def native_constants():
    lines=["#[derive(Clone, Copy, Debug, PartialEq)]","pub struct MeasuredRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }","impl MeasuredRect {", "    pub const fn logical(self, pixels_per_point: f32) -> Self {", "        Self { x: self.x / pixels_per_point, y: self.y / pixels_per_point, w: self.w / pixels_per_point, h: self.h / pixels_per_point }", "    }", "    pub const fn window_local_px(self) -> Self { Self { x: self.x - 28.0, y: self.y - 22.0, w: self.w, h: self.h } }", "    pub const fn window_logical(self, pixels_per_point: f32) -> Self { self.window_local_px().logical(pixels_per_point) }", "}","pub const REFERENCE_SIZE_PX: (f32, f32) = (1672.0, 941.0);","pub const REFERENCE_PIXELS_PER_POINT: f32 = 1.0;","pub const WINDOW_ORIGIN_PX: (f32, f32) = (28.0, 22.0);", "pub const AREAS: &[(&str, MeasuredRect)] = &["]
    for id,name,b in AREAS:lines.append(f'    ("{id}", MeasuredRect {{ x: {b[0]}.0, y: {b[1]}.0, w: {b[2]}.0, h: {b[3]}.0 }}),')
    lines += ["];","pub const COMPONENTS: &[(&str, MeasuredRect)] = &["]
    for r in DATA["rectangles"]:
        x,y,w,h=r["bounds"];lines.append(f'    ("{r["id"]}", MeasuredRect {{ x: {x}.0, y: {y}.0, w: {w}.0, h: {h}.0 }}),')
    lines += ["];",""]
    (ROOT/"layout.rs").write_text("\n".join(lines),encoding="utf-8",newline="\n")

def main():
    color_tokens();make_markdown();annotate();layered_svg();native_constants()
    print(f"Built SPEC.md, {len(COLORS)} role samples, 11 annotation PNGs, layered SVG, tokens.json, and Rust layout constants.")

if __name__=="__main__":main()
