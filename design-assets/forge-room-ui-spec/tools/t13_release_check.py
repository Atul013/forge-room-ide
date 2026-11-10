"""Verify freshly downloaded T13 artifacts without executing their binaries."""
import hashlib
import json
import struct
import zipfile
from pathlib import Path

import pefile

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'design-assets/forge-room-ui-spec/measurements'
FOLDER = Path((OUT / 't13-release-folder.txt').read_text().strip())


def sha(data):
    return hashlib.sha256(data).hexdigest()


def ico_frames(data):
    result = []
    for i in range(struct.unpack_from('<H', data, 4)[0]):
        w, h, _, _, _, _, size, offset = struct.unpack_from('<BBBBHHII', data, 6 + 16*i)
        result.append({'size': [w or 256, h or 256], 'sha256': sha(data[offset:offset+size])})
    return result


def pe_icons(data):
    pe = pefile.PE(data=data)
    icons, groups = {}, []
    for typ in pe.DIRECTORY_ENTRY_RESOURCE.entries:
        if typ.id not in (3, 14):
            continue
        for entry in typ.directory.entries:
            for language in entry.directory.entries:
                r = language.data.struct
                blob = pe.get_data(r.OffsetToData, r.Size)
                if typ.id == 3:
                    icons[entry.id] = blob
                else:
                    groups.append((str(entry.name or entry.id), blob))
    result = []
    for name, group in groups:
        frames = []
        for i in range(struct.unpack_from('<H', group, 4)[0]):
            w, h, _, _, _, _, _, ident = struct.unpack_from('<BBBBHHIH', group, 6+14*i)
            frames.append({'size': [w or 256, h or 256], 'sha256': sha(icons[ident])})
        result.append({'group': name, 'frames': frames, 'matches_canonical': frames == canonical})
    return result


canonical = ico_frames((ROOT / 'design-assets/forge-room-brand/forge-room.ico').read_bytes())
report = {'folder': str(FOLDER), 'canonical_frames': canonical, 'hashes': [], 'pe_icons': {}}
for line in (FOLDER / 'SHA256SUMS.txt').read_text().splitlines():
    expected, name = line.split(maxsplit=1)
    name = name.lstrip('*')
    data = (FOLDER / name).read_bytes()
    actual = sha(data)
    report['hashes'].append({'file': name, 'expected': expected.lower(), 'actual': actual,
                             'matches': actual == expected.lower()})
    if name.endswith('.zip'):
        with zipfile.ZipFile(FOLDER / name) as z:
            report['zip_integrity_bad_member'] = z.testzip()
            report['zip_members'] = z.namelist()
            for member in z.namelist():
                if member.endswith('.exe'):
                    report['pe_icons'][member] = pe_icons(z.read(member))
    elif name.endswith('.exe'):
        report['pe_icons'][name] = pe_icons(data)
report['release'] = json.loads((FOLDER / 'release.json').read_text(encoding='utf-8-sig'))
report['metadata'] = {name: json.loads((FOLDER / name).read_text(encoding='utf-8-sig'))
                      for name in ('BUILD_METRICS.json', 'INSTALL_VERIFY.json')}
(OUT / 't13-release-check.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
