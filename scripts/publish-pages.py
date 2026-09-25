#!/usr/bin/env python3
"""Ship the SPA through the cyberia.my pages repo — the path that needs no
SSH to cyberproxy.

The host cron copies every top-level directory of cyberia-to/cyberia.my into
the docroot once a minute (cybernode/sites/cyberia.my/sync.sh). This builds
the app with its assets under /app/ and gives each route of this release its
own directory whose index.html links to app/index.html; nginx serves
`/map` → `/map/` → that index, and the router takes it from there.

    python3 scripts/publish-pages.py ~/cyber/cyberia.my
    # then commit + push the pages repo

The site root keeps whatever the last SSH deploy put there; the routes below
carry this build. Once SSH works again, `nu scripts/deploy.nu` replaces the
root — then drop the generated directories from the pages repo (listed in
its `.spa-routes`), or they keep shadowing those paths.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP = "app"
MANIFEST = ".spa-routes"
# top-level paths this release owns; flats are added from the map
ROUTES = ["map", "menu", "cities"]


def run(cmd, **kw):
    print("→", " ".join(cmd))
    subprocess.run(cmd, check=True, **kw)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    pages = Path(sys.argv[1]).expanduser().resolve()
    if not (pages / ".git").exists():
        print(f"{pages} is not a git clone of cyberia-to/cyberia.my")
        return 1

    env = dict(os.environ, PATH=f"{Path.home()}/.cargo/bin:{os.environ['PATH']}")
    with tempfile.TemporaryDirectory() as tmp:
        run(
            ["trunk", "build", "--release", "--public-url", f"/{APP}/", "--dist", tmp],
            cwd=ROOT,
            env=env,
        )
        app = pages / APP
        shutil.rmtree(app, ignore_errors=True)
        shutil.copytree(tmp, app)

    # clear the previous release's route directories
    manifest = pages / MANIFEST
    if manifest.exists():
        for rel in manifest.read_text().split():
            p = pages / rel
            if p.is_symlink() or p.is_file():
                p.unlink()
    for top in {r.split("/")[0] for r in (manifest.read_text().split() if manifest.exists() else [])}:
        d = pages / top
        if d.is_dir():
            # remove only emptied directories — never a hand-made page
            for sub in sorted(d.rglob("*"), key=lambda x: -len(x.parts)):
                if sub.is_dir() and not any(sub.iterdir()):
                    sub.rmdir()
            if d.is_dir() and not any(d.iterdir()):
                d.rmdir()

    plots = json.load(open(ROOT / "src" / "cyberia_map.json"))["phase0"]
    routes = list(ROUTES)
    for p in plots:
        routes += [f"plot/{p['id']}", f"plot/{p['id']}/lease"]

    written = []
    for r in routes:
        d = pages / r
        d.mkdir(parents=True, exist_ok=True)
        link = d / "index.html"
        if link.exists() or link.is_symlink():
            link.unlink()
        target = os.path.relpath(pages / APP / "index.html", d)
        link.symlink_to(target)
        written.append(f"{r}/index.html")

    manifest.write_text("\n".join(written) + "\n")
    print(f"{len(written)} routes → {APP}/index.html · build in {APP}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
