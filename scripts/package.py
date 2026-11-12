#!/usr/bin/env python3
"""Create the delivery ZIP and verify it.

    python3 scripts/package.py build  [OUT]   # default dist/Natures-Thread.zip
    python3 scripts/package.py verify [ZIP]   # extract to a temp dir and check it

The archive contains the source tree under a top-level `Natures-Thread/`
folder. Secrets, caches and local build outputs are never included.
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOP = "Natures-Thread"

EXCLUDED_DIRS = {".git", "target", "node_modules", "dist", "certs", ".vite", ".idea", ".vscode", "__pycache__", ".claude"}
EXCLUDED_NAMES = {".DS_Store", "Thumbs.db"}
EXCLUDED_SUFFIXES = {".log", ".pem", ".key", ".crt", ".cer", ".zip", ".pyc", ".rs.bk", ".tmp"}

REQUIRED = [
    "README.md", "Makefile", ".env.example", ".gitignore", "Cargo.toml", "Cargo.lock",
    "package.json", "package-lock.json", "LICENSE", "THIRD_PARTY_NOTICES.md",
    "client/index.html", "client/src/main.ts", "client/vite.config.ts",
    "crates/game-core/src/lib.rs", "crates/game-core/src/solver.rs", "crates/game-server/src/main.rs",
    "database/migrations/0001_initial_schema.sql",
    "content/regions/forest_realm.json", "content/journal.json",
    "docs/GAME_DESIGN.md", "docs/RULE_BOOK.md", "docs/DATABASE.md", "docs/API.md",
    "scripts/make-certs.sh", "tests/e2e/room.mjs",
] + [f"content/levels/{n}" for n in (
    "01_missing_connection.json", "02_restore_the_forest.json", "03_who_belongs_here.json",
    "04_forest_in_balance.json", "05_missing_clue.json", "06_under_pressure.json",
    "07_ecosystem_puzzle.json", "08_nature_under_threat.json", "09_final_blueprint.json",
    "10_living_forest.json")]

SECRET_PATTERNS = [
    re.compile(rb"JWT_SECRET=(?!REPLACE_WITH_A_LONG_RANDOM_SECRET)[A-Za-z0-9+/=_-]{16,}"),
    # URL with an inline password (placeholders such as `<pw>`, `{}`, `pass` are allowed)
    re.compile(rb"postgres(?:ql)?://[A-Za-z0-9_.-]+:(?!pass@|password@)[^<\s@{}:/]{4,}@"),
    re.compile(rb"-----BEGIN (?:RSA |EC )?PRIVATE KEY-----"),
]


def included(rel: Path) -> bool:
    parts = rel.parts
    if any(p in EXCLUDED_DIRS for p in parts[:-1]) or parts[0] in EXCLUDED_DIRS:
        return False
    name = parts[-1]
    if name in EXCLUDED_NAMES or any(name.endswith(s) for s in EXCLUDED_SUFFIXES):
        return False
    if name == ".env" or (name.startswith(".env.") and name != ".env.example"):
        return False
    return True


def build(out: Path) -> Path:
    out.parent.mkdir(parents=True, exist_ok=True)
    files = sorted(p for p in ROOT.rglob("*") if p.is_file() and included(p.relative_to(ROOT)))
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for p in files:
            rel = p.relative_to(ROOT).as_posix()
            data = p.read_bytes()
            for pat in SECRET_PATTERNS:
                if pat.search(data):
                    sys.exit(f"refusing to package: {rel} looks like it contains a secret ({pat.pattern[:40]!r})")
            info = zipfile.ZipInfo(f"{TOP}/{rel}", date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o755 if os.access(p, os.X_OK) or rel.endswith(".sh") else 0o644) << 16
            z.writestr(info, data)
    print(f"✔ {out} — {len(files)} files, {out.stat().st_size / 1024:.0f} KiB")
    return out


def run(cmd: list[str], cwd: Path) -> None:
    print("  $ " + " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def verify(zip_path: Path, full: bool) -> None:
    if not zip_path.is_file():
        sys.exit(f"{zip_path} not found — run `make package` first")
    with tempfile.TemporaryDirectory(prefix="nt-verify-") as tmp:
        with zipfile.ZipFile(zip_path) as z:
            names = z.namelist()
            bad = [n for n in names if not included(Path(n).relative_to(TOP)) or ".." in Path(n).parts]
            if bad:
                sys.exit(f"archive contains excluded files: {bad[:5]}")
            z.extractall(tmp)
        root = Path(tmp) / TOP
        missing = [r for r in REQUIRED if not (root / r).is_file()]
        if missing:
            sys.exit("missing required files:\n  " + "\n  ".join(missing))
        print(f"✔ {len(names)} files, all {len(REQUIRED)} required files present, no secrets or caches")
        if full:
            print("▶ Building from the clean extraction")
            env_target = Path(tmp) / "target"
            os.environ["CARGO_TARGET_DIR"] = str(env_target)
            run(["cargo", "build", "--locked", "--workspace", "--all-targets"], root)
            run(["cargo", "test", "--locked", "-p", "natures-thread-core", "--lib"], root)
            run(["npm", "ci", "--no-audit", "--no-fund"], root)
            run(["npm", "run", "build"], root)
            run(["npm", "test"], root)
            print("✔ clean-extraction build and tests passed")


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "build"
    if cmd == "build":
        build(Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "dist" / f"{TOP}.zip")
    elif cmd == "verify":
        verify(Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "dist" / f"{TOP}.zip", full="--quick" not in sys.argv)
    else:
        sys.exit(__doc__)
