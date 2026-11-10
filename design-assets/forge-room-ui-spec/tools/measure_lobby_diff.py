"""Reproduce T2 screenshot dimensions, landmark ratios, and direct RGB samples."""

from pathlib import Path
from collections import Counter
import hashlib
import json
import argparse
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
SOURCES = {
    "reference": Path(r"C:\Users\Icarus\.codex\generated_images\01a19bb3-cacd-7363-a6ab-0171bbb54092\exec-3b3cc3a1-3bd6-44ba-8fcd-aecf6e97295b.png"),
    "app": REPO / "artifacts/lobby-compare/lobby-current.png",
}
# Screenshot landmarks read at native image size; approximate AA edges are +/-2 px.
BOXES = {
    "reference": {
        "title_bar": [0, 0, 1672, 36], "rail": [0, 36, 115, 910],
        "pulse": [120, 40, 1668, 84], "logo": [144, 101, 255, 191],
        "enter_room": [1406, 121, 1584, 170], "countdown": [1104, 101, 1365, 190],
        "active_tab": [143, 205, 255, 245], "overview": [143, 260, 1146, 497],
        "goal": [630, 308, 1134, 486], "setup": [1159, 260, 1653, 488],
        "team": [145, 507, 623, 724], "agents": [634, 507, 1149, 724],
        "milestones": [146, 733, 1151, 886], "configuration": [1162, 497, 1655, 755],
        "invite": [1164, 766, 1658, 861], "status": [0, 910, 1672, 941],
        "active_rail_item": [4, 131, 110, 167], "avatar": [163, 558, 207, 602],
        "progress": [895, 776, 1139, 871],
    },
    "app": {
        "title_bar": [0, 0, 1550, 46], "rail": [0, 46, 132, 930],
        "pulse": [146, 47, 1536, 97], "logo": [146, 113, 238, 205],
        "enter_room": [1343, 113, 1483, 155], "countdown": [1067, 113, 1327, 201],
        "active_tab": [146, 217, 246, 257], "overview": [146, 269, 1010, 585],
        "goal": [585, 325, 995, 463], "setup": [1022, 269, 1536, 538],
        "team": [146, 597, 572, 744], "agents": [584, 597, 1010, 693],
        "milestones": [146, 756, 1010, 930], "configuration": [1022, 550, 1536, 863],
        "invite": [1022, 875, 1536, 930], "status": [0, 930, 1550, 975],
        "active_rail_item": [6, 141, 126, 179], "avatar": [161, 660, 205, 704],
        "progress": [775, 812, 995, 920],
    },
}
SAMPLES = {
    "reference": {
        "canvas": [130, 200], "rail": [50, 65], "card_body": [500, 470],
        "card_header": [900, 280], "input": [1480, 354], "border": [450, 260],
        "active_rail": [90, 150], "primary_button": [1480, 130],
        "logo_accent": [197, 137], "tab_fill": [166, 215],
        "countdown_stroke": [1313, 149],
    },
    "app": {
        "canvas": [135, 250], "rail": [60, 60], "card_body": [500, 570],
        "card_header": [900, 290], "input": [1460, 387], "border": [450, 269],
        "active_rail": [90, 150], "primary_button": [1400, 120],
        "logo_accent": [192, 145], "tab_fill": [166, 225],
        "countdown_stroke": [1107, 158],
    },
}


def main():
    parser = argparse.ArgumentParser()
    variants = parser.add_mutually_exclusive_group()
    variants.add_argument("--second", action="store_true", help="Measure the T3 populated-room screenshot instead of T2")
    variants.add_argument("--third", action="store_true", help="Measure the T5 final populated-room screenshot")
    args = parser.parse_args()
    if args.second:
        SOURCES["app"] = REPO / "artifacts/lobby-compare/lobby-v2.png"
        BOXES["app"] = {
            "title_bar": [0, 0, 1550, 36], "rail": [0, 36, 126, 936],
            "pulse": [140, 37, 1536, 75], "logo": [140, 89, 243, 173],
            "enter_room": [1337, 89, 1477, 134], "countdown": [1073, 89, 1315, 172],
            "active_tab": [140, 185, 233, 223], "overview": [140, 236, 1067, 516],
            "goal": [611, 282, 1054, 507], "setup": [1079, 236, 1536, 444],
            "team": [140, 528, 579, 751], "agents": [591, 528, 1067, 698],
            "milestones": [140, 763, 1067, 914], "configuration": [1079, 456, 1536, 709],
            "invite": [1079, 721, 1550, 821], "status": [0, 936, 1550, 975],
            "active_rail_item": [6, 125, 120, 160], "avatar": [153, 583, 197, 627],
            "progress": [834, 809, 1054, 901],
        }
        SAMPLES["app"] = {
            "canvas": [135, 220], "rail": [60, 50], "card_body": [500, 500],
            "card_header": [900, 255], "input": [1400, 325], "border": [450, 236],
            "active_rail": [90, 140], "primary_button": [1400, 96],
            "logo_accent": [191, 110], "tab_fill": [166, 190],
        }
    if args.third:
        SOURCES["app"] = REPO / "artifacts/lobby-compare/lobby-v3.png"
        BOXES["app"] = {
            "title_bar": [0, 0, 1550, 36], "rail": [0, 36, 118, 949],
            "pulse": [123, 40, 1545, 81], "logo": [144, 101, 247, 185],
            "enter_room": [1305, 101, 1470, 146], "countdown": [1041, 101, 1283, 184],
            "active_tab": [144, 197, 248, 236], "overview": [144, 248, 1066, 482],
            "goal": [609, 293, 1053, 473], "setup": [1078, 248, 1532, 455],
            "team": [144, 494, 581, 710], "agents": [593, 494, 1066, 710],
            "milestones": [144, 722, 1066, 868], "configuration": [1078, 467, 1532, 719],
            "invite": [1078, 731, 1532, 816], "status": [0, 949, 1550, 975],
            "active_rail_item": [6, 125, 112, 160], "avatar": [157, 539, 198, 580],
            "progress": [833, 767, 1053, 859],
        }
        SAMPLES["app"] = {
            "canvas": [135, 220], "rail": [60, 50], "card_body": [500, 470],
            "card_header": [900, 265], "border": [450, 248],
            "active_rail": [90, 140], "primary_button": [1400, 107],
            "tab_fill": [166, 205],
        }
    result = {}
    for name, path in SOURCES.items():
        image = Image.open(path).convert("RGB")
        samples = {}
        for role, (x, y) in SAMPLES[name].items():
            pixel = image.getpixel((x, y))
            patch = image.crop((x - 2, y - 2, x + 3, y + 3))
            mode, count = Counter(patch.getdata()).most_common(1)[0]
            samples[role] = {"point": [x, y], "pixel": "#%02X%02X%02X" % pixel,
                             "patchMode": "#%02X%02X%02X" % mode, "modeCountOf25": count}
        boxes = {key: {"xyxy": box, "size": [box[2]-box[0], box[3]-box[1]],
                        "normalized": [round(v / image.size[i % 2], 6) for i, v in enumerate(box)]}
                 for key, box in BOXES[name].items()}
        result[name] = {"source": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                        "size": list(image.size), "boxes": boxes, "samples": samples}
    result["method"] = "Native screenshot landmarks +/-2px; direct RGB samples; font family/weight cannot be recovered exactly from pixels."
    filename = "lobby-diff-3.json" if args.third else "lobby-diff-2.json" if args.second else "lobby-diff.json"
    destination = ROOT / "measurements" / filename
    destination.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({name: {"size": result[name]["size"], "samples": result[name]["samples"]} for name in SOURCES}, indent=2))


if __name__ == "__main__":
    main()
