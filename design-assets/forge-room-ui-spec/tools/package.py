"""Create a verified ZIP of the deliverables, excluding enlarged scratch crops."""
from pathlib import Path
import hashlib
import zipfile

ROOT=Path(__file__).resolve().parents[1]
DEST=ROOT.parents[1]/"artifacts/forge-room-ui-spec.zip"

def main():
    files=sorted(p for p in ROOT.rglob("*") if p.is_file() and "__pycache__" not in p.parts and "crops" not in p.relative_to(ROOT).parts)
    assert (ROOT/"validation.json").exists()
    DEST.parent.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(DEST,"w",zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        for p in files:z.write(p,Path("forge-room-ui-spec")/p.relative_to(ROOT))
    with zipfile.ZipFile(DEST) as z:
        assert z.testzip() is None
        assert len(z.namelist())==len(files)
        for p in files:
            name=(Path("forge-room-ui-spec")/p.relative_to(ROOT)).as_posix()
            assert z.read(name)==p.read_bytes(),name
    print(f"ZIP verified against source bytes: {len(files)} files, {DEST.stat().st_size:,} bytes.")
    print("SHA256:",hashlib.sha256(DEST.read_bytes()).hexdigest())
    print(DEST)

if __name__=="__main__":main()
