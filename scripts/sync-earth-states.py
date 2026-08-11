#!/usr/bin/env python3
"""Export terrestrial cyberstates → cyberia-my/src/earth_states.json

Earth countries only: not Solar System / Oceans / Terra Nullius, not
continent aggregates (OCNA/AFRI/EURA/AMER). Sorted by capital (money
supply B USD) descending — same default board as cyberstates.net.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CYBER = ROOT.parent / "cyberstates"
STATES_DIR = CYBER / "states"
OUT = ROOT / "src" / "earth_states.json"

AGG = {"OCNA", "AFRI", "EURA", "AMER"}
NON_TERR = {"Oceans", "Terra Nullius", "Solar System"}


def get_str(text: str, key: str, default: str = "") -> str:
    m = re.search(rf"^{re.escape(key)}\s*=\s*\"(.*)\"\s*$", text, re.M)
    return m.group(1) if m else default


def get_num(text: str, key: str, default: float = 0.0) -> float:
    m = re.search(rf"^{re.escape(key)}\s*=\s*([0-9.eE+-]+)\s*$", text, re.M)
    if not m:
        return default
    try:
        return float(m.group(1))
    except ValueError:
        return default


def main() -> int:
    if not STATES_DIR.is_dir():
        print(f"missing {STATES_DIR}", file=sys.stderr)
        return 1

    rows = []
    for p in sorted(STATES_DIR.glob("*.toml")):
        t = p.read_text(encoding="utf-8")
        code = get_str(t, "code")
        region = get_str(t, "region")
        if code in AGG or region in NON_TERR:
            continue
        rows.append(
            {
                "name": get_str(t, "name"),
                "code": code,
                "slug": get_str(t, "slug"),
                "flag": get_str(t, "flag"),
                "region": region,
                "population": int(get_num(t, "population", 0)),
                "land_area_km2": int(get_num(t, "land_area_km2", 0)),
                "currency_code": get_str(t, "currency_code"),
                "currency_name": get_str(t, "currency_name"),
                "money_supply_b_usd": get_num(t, "money_supply_b_usd", 0),
                "money_supply_b_usd_prev": get_num(t, "money_supply_b_usd_prev", 0),
                "token_price_usd": get_num(t, "token_price_usd", 0),
            }
        )

    rows.sort(key=lambda r: -r["money_supply_b_usd"])
    payload = {
        "source": "cyberstates states/*.toml",
        "filter": "earth countries — terrestrial, non-aggregate",
        "sort": "capital money_supply_b_usd desc",
        "count": len(rows),
        "states": rows,
    }
    OUT.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
        encoding="utf-8",
    )
    print(f"wrote {OUT} · {len(rows)} earth states")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
