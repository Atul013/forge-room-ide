"""Build the standalone Forge Room SVG assets and symbol sprite."""

from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[1]
ICONS = {
    "explorer": [
        "M3 7V6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z",
    ],
    "search": [
        "M18.5 10.75a7.75 7.75 0 1 1-15.5 0 7.75 7.75 0 0 1 15.5 0Z",
        "m16.25 16.25 4.75 4.75",
    ],
    "source-control": [
        "M6 7v10m0-2h6a6 6 0 0 0 6-6",
        "M8 5a2 2 0 1 1-4 0 2 2 0 0 1 4 0Zm0 14a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM20 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
    ],
    "run-debug": [
        "M6 4.75q0-1.5 1.3-.75l12 7a1.15 1.15 0 0 1 0 2l-12 7q-1.3.75-1.3-.75Z",
    ],
    "extensions": [
        "M8 6H5a2 2 0 0 0-2 2v2.5h1.25a2.25 2.25 0 0 1 0 4.5H3v3a2 2 0 0 0 2 2h3.5v-1.25a2.25 2.25 0 0 1 4.5 0V20h3a2 2 0 0 0 2-2v-3h.75a2.25 2.25 0 0 0 0-4.5H18V8a2 2 0 0 0-2-2h-3.5v-.75a2.25 2.25 0 0 0-4.5 0Z",
    ],
    "ai": [
        "M12 3c.7 6.3 2.7 8.3 9 9-6.3.7-8.3 2.7-9 9-.7-6.3-2.7-8.3-9-9 6.3-.7 8.3-2.7 9-9Z",
    ],
    "terminal": [
        "M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z",
        "m7 9 3 3-3 3m6 0h4",
    ],
    "room": [
        "M11.75 8a3.25 3.25 0 1 1-6.5 0 3.25 3.25 0 0 1 6.5 0ZM19.5 8.25a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Z",
        "M3 20v-1.5a5.5 5.5 0 0 1 11 0V20Zm13.75-5.75H17a4 4 0 0 1 4 4V20h-3.75",
    ],
    "new-file": [
        "M11 21H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8l6 6v3M14 3v4a2 2 0 0 0 2 2h4",
        "M18 15v6m-3-3h6",
    ],
    "refresh": [
        "M20.25 8.4a9 9 0 1 0 0 7.2M20.25 4v4.4h-4.4",
    ],
    "more": [
        "M5 12h.01M12 12h.01M19 12h.01",
    ],
    "close": [
        "m5 5 14 14M19 5 5 19",
    ],
    "plus": [
        "M12 4v16M4 12h16",
    ],
    "chevron-right": [
        "m9 5 7 7-7 7",
    ],
    "chevron-down": [
        "m5 9 7 7 7-7",
    ],
    "check": [
        "m4 12.5 5.5 5.5L20 6.5",
    ],
    "arrow-up": [
        "M12 21V3M5.5 9.5 12 3l6.5 6.5",
    ],
    "arrow-down": [
        "M12 3v18m-6.5-6.5L12 21l6.5-6.5",
    ],
    "folder-open": [
        "M3 18V6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2",
        "m3 18.5 2.05-6.15Q5.5 11 7 11h12.75q1.5 0 1.05 1.5l-2.05 6.15Q18.3 20 16.8 20H4.5q-2 0-1.5-1.5Z",
    ],
    "file": [
        "M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9ZM14 3v4a2 2 0 0 0 2 2h4",
    ],
    "file-json": [
        "M8.5 3.5H7a2 2 0 0 0-2 2v3.75Q5 12 3 12q2 0 2 2.75v3.75a2 2 0 0 0 2 2h1.5m7-17H17a2 2 0 0 1 2 2v3.75Q19 12 21 12q-2 0-2 2.75v3.75a2 2 0 0 1-2 2h-1.5",
    ],
    "file-ts": [
        "M3 5h8M7 5v14",
        "M20.5 7C20.5 5.6 19.2 5 17.5 5H16a3.25 3.25 0 0 0 0 6.5h1.25a3.75 3.75 0 0 1 0 7.5H16c-1.8 0-3-.8-3-2.25",
    ],
    "commit-dropdown": [
        "m5 9 7 7 7-7",
    ],
}
ICONS["room-active"] = ICONS["room"].copy()

STYLE = 'fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"'


def build():
    directory = ROOT / "svg"
    directory.mkdir(parents=True, exist_ok=True)
    symbols = []
    for name, paths in ICONS.items():
        body = "\n".join(f'  <path d="{path}"/>' for path in paths)
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" {STYLE}>\n{body}\n</svg>\n'
        (directory / f"{name}.svg").write_text(svg, encoding="utf-8", newline="\n")
        symbols.append(
            f'  <symbol id="forge-{name}" viewBox="0 0 24 24" {STYLE}>\n'
            + "\n".join(f'    <path d="{path}"/>' for path in paths)
            + "\n  </symbol>"
        )
    sprite = '<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" viewBox="0 0 24 24" fill="none">\n' + "\n".join(symbols) + "\n</svg>\n"
    (ROOT / "forge-room-sprite.svg").write_text(sprite, encoding="utf-8", newline="\n")
    manifest = {
        "viewBox": [0, 0, 24, 24],
        "strokeWidth": 1.75,
        "railOrder": list(ICONS)[:8],
        "icons": [{"name": name, "file": f"svg/{name}.svg", "symbol": f"forge-{name}"} for name in ICONS],
        "aliases": {"room-active": "room", "commit-dropdown": "chevron-down"},
    }
    (ROOT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Built {len(ICONS)} standalone SVGs and {len(symbols)} sprite symbols.")


if __name__ == "__main__":
    build()
