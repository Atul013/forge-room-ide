"""Exercise the actual merge resolver using disposable Git indexes, without any commits."""

from pathlib import Path
import json
import subprocess
import contextlib
import shutil
import stat
import uuid

ROOT = Path(__file__).resolve().parents[3]
GIT_SOURCE = ROOT / "native/forge-room-app/src/git.rs"
DOCUMENT_SOURCE = ROOT / "native/forge-room-app/src/document.rs"


def execute(args, cwd=None, data=None, check=True):
    result = subprocess.run(args, cwd=cwd, input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if check and result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", "replace"))
    return result


@contextlib.contextmanager
def disposable_fixture():
    scratch = ROOT / "design-assets/forge-room-ui-spec/.review-temp"
    scratch.mkdir(exist_ok=True)
    temporary = scratch / ("merge-" + uuid.uuid4().hex)
    temporary.mkdir()
    try:
        yield temporary
    finally:
        assert temporary.resolve().parent == scratch.resolve()
        assert temporary.name.startswith("merge-")
        def remove_readonly(function, path, error):
            Path(path).chmod(stat.S_IWRITE)
            function(path)
        shutil.rmtree(temporary, onexc=remove_readonly)
        scratch.rmdir()


def main():
    results = []
    with disposable_fixture() as temporary:
        helper = temporary / "resolver.exe"
        source = f'''#![allow(dead_code)]
#[path = "{GIT_SOURCE.as_posix()}"] mod git;
#[path = "{DOCUMENT_SOURCE.as_posix()}"] mod document;
fn main() {{
    let args: Vec<String> = std::env::args().collect();
    let root = std::path::Path::new(&args[1]);
    let side = if args[3] == "current" {{ git::Side::Current }} else {{ git::Side::Incoming }};
    let mut doc = document::DocumentSession::default();
    if args.len() > 4 {{ doc.open(root.join(&args[2])).unwrap(); *doc.text_mut()="dirty editor content\\n".into(); }}
    match git::resolve_conflict(root, &args[2], side) {{
        Ok(()) => println!("resolved"),
        Err(error) => {{ eprintln!("{{error}}"); std::process::exit(1); }}
    }}
    if args.len() > 4 {{
        assert!(doc.is_dirty());
        assert!(matches!(doc.save(),Err(document::DocumentError::ChangedOnDisk)));
        println!("dirty buffer retained; normal save refuses to erase chosen side");
    }}
}}
'''
        execute(["rustc", "--edition=2024", "--crate-name", "forge_merge_review", "-", "-o", str(helper)], data=source.encode())

        def scenario(name, paths, side="incoming", dirty=False, binary=False):
            repo = temporary / name
            repo.mkdir()
            execute(["git", "init", "-q"], cwd=repo)
            lines = []
            for filename, stages in paths.items():
                (repo / filename).write_bytes(b"working copy\n")
                for stage in stages:
                    content = (b"\x00" if binary else b"") + f"stage {stage}\n".encode()
                    oid = execute(["git", "hash-object", "-w", "--stdin"], cwd=repo, data=content).stdout.strip().decode()
                    lines.append(f"100644 {oid} {stage}\t{filename}\0".encode())
            execute(["git", "update-index", "-z", "--index-info"], cwd=repo, data=b"".join(lines))
            filename = next(iter(paths))
            args = [str(helper), str(repo), filename, side] + (["dirty"] if dirty else [])
            resolved = execute(args, check=False)
            entries = execute(["git", "ls-files", "--stage", "-z"], cwd=repo).stdout.decode().split("\0")
            entry_list = [entry for entry in entries if entry]
            row = {"case": name, "path": filename, "side": side, "exitCode": resolved.returncode,
                   "stdout": resolved.stdout.decode("utf-8", "replace").strip(),
                   "stderr": resolved.stderr.decode("utf-8", "replace").strip(), "indexEntries": entry_list,
                   "worktreeHex": {p: (repo / p).read_bytes().hex() for p in paths if (repo / p).exists()}}
            results.append(row)

        scenario("spaces", {"a file.txt": [1, 2, 3]})
        scenario("leading-dash", {"-notes.txt": [1, 2, 3]})
        scenario("binary", {"blob.bin": [1, 2, 3]}, binary=True)
        scenario("incoming-deleted", {"deleted.txt": [1, 2]})
        scenario("current-deleted", {"deleted.txt": [1, 3]}, side="current")
        scenario("pathspec-brackets", {"a[1].txt": [1, 2, 3], "a1.txt": [1, 2, 3]})
        scenario("dirty-buffer", {"open.txt": [1, 2, 3]}, dirty=True)
        # Recursive cleanup is confined to the verified disposable workspace fixture.
    destination = ROOT / "design-assets/forge-room-ui-spec/measurements/merge-review-probes.json"
    destination.write_text(json.dumps({"noCommitsCreated": True, "scenarios": results}, indent=2) + "\n", encoding="utf-8")
    for row in results:
        print(row["case"], "exit", row["exitCode"], row["stdout"], row["stderr"].splitlines()[:1])
    print("Evidence:", destination)


if __name__ == "__main__":
    main()
