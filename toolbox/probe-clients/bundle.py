#!/usr/bin/env python3
"""Assemble verified classpaths from Maven outputs for the desktop resources package."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tempfile

ROOT = Path(__file__).resolve().parent


def artifacts(module):
    target = module / "target"
    return [target / "probe.jar", *sorted((target / "lib").glob("*.jar"))]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--record", action="store_true", help="Record a reviewed release build in the source lock")
    opts = parser.parse_args()
    modules = sorted((ROOT / "modules").iterdir())
    lock = {}
    files = {}
    for module in modules:
        target = module / "target"
        files[module.name] = artifacts(module)
        if len(files[module.name]) < 2:
            raise SystemExit(f"Missing packaged dependencies: {module.name}")
        lock[module.name] = {}
        for file in files[module.name]:
            with file.open("rb") as f:
                digest = hashlib.sha256()
                for chunk in iter(lambda: f.read(65536), b""):
                    digest.update(chunk)
                lock[module.name][file.relative_to(target).as_posix()] = digest.hexdigest()
    path = ROOT / "artifacts.lock.json"
    if opts.record:
        path.write_text(json.dumps(lock, indent=2, sort_keys=True)+"\n")
    if json.loads(path.read_text()) != lock:
        raise SystemExit("Built artifacts differ from the reviewed classpath lock")
    dest = ROOT / "target"
    dest.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=dest) as temp:
        stage = Path(temp) / "bundle"
        for module in modules:
            for file in files[module.name]:
                out = stage / module.name / file.relative_to(module / "target")
                out.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(file, out)
                out.chmod(0o644)
        for directory in stage.rglob("*"):
            if directory.is_dir():
                directory.chmod(0o755)
        stage.chmod(0o755)
        final = dest / "bundle"
        previous = Path(temp) / "previous"
        if final.exists():
            final.rename(previous)
        try:
            stage.rename(final)
        except OSError:
            if previous.exists():
                previous.rename(final)
            raise
    print(f"Verified {sum(map(len,files.values()))} artifacts in {len(modules)} isolated classpaths")


if __name__ == "__main__":
    main()
