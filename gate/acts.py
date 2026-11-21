#!/usr/bin/env python3
"""the acts of cyberia.my's land market and the state they fold into (projects/listings.md).

an act is a short text a wallet signs with personal_sign (EIP-191). the gate checks who signed it and whether that
address may do it, commits it to the acts repository as one file, and the state of the market is nothing but the fold
of those files in order: who holds a flat, whom a holder gave a mandate, what is listed, what the valley approved.
nothing is stored anywhere else, so anyone holding the repository recomputes the same market.

    python3 gate/acts.py verify <acts-dir> <root-address> [<map.json>]   every act's signature and right, in order; exit 1 on any failure
    python3 gate/acts.py public <acts-dir> <root-address>     the public listings as json

the act, as the wallet shows it:

    cyberia.my act
    kind: listing
    id: 6f1c…-uuid
    at: 2026-11-20T10:00:00Z
    plot: sinwood-7
    deal: sale
    price_usd: 42000
    …

kinds and who may sign them:

    role      the root, or an admin for `agent`      address · role admin|agent|none · name
    profile   anyone, for themselves                 name · contact
    claim     anyone                                 plot · name · contact          "this flat is mine — bind my address"
    holder    an admin                               plot · address · name          binds a flat to its holder's address
    mandate   the flat's holder                      plot · agent · scope sale|lease|both · commission · until
    revoke    the holder, the agent, or an admin     mandate
    listing   the holder, or an agent with a live    plot · deal sale|lease · price_usd · rent_usd_year · term_years ·
              mandate that covers the deal           words · contact
    approve   an admin                               listing
    reject    an admin                               target (a listing or a claim) · reason
    withdraw  the listing's signer, the flat's       listing
              holder, or an admin
"""
import json, re, sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sign  # noqa: E402

HEAD = "cyberia.my act"
KINDS = ("role", "profile", "claim", "holder", "mandate", "revoke", "listing", "approve", "reject", "withdraw")
UUID = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
ADDR = re.compile(r"^0x[0-9a-fA-F]{40}$")
PLOT = re.compile(r"^[a-z0-9][a-z0-9-]{1,40}$")
DAY = re.compile(r"^\d{4}-\d{2}-\d{2}$")
MAX_LINE = 700


def fields(message):
    """the act's fields from its signed text; ValueError when it is not an act."""
    lines = message.split("\n")
    if not lines or lines[0] != HEAD: raise ValueError("not an act of cyberia.my")
    out = {}
    for l in lines[1:]:
        if not l.strip(): continue
        if ": " not in l and not l.endswith(":"): raise ValueError(f"not a field: {l[:60]}")
        k, _, v = l.partition(":")
        if len(l) > MAX_LINE: raise ValueError(f"{k} is too long")
        if k in out: raise ValueError(f"{k} is written twice")
        out[k] = v.strip()
    if out.get("kind") not in KINDS: raise ValueError(f"unknown kind {out.get('kind')}")
    if not UUID.match(out.get("id", "")): raise ValueError("the id is not a uuid")
    datetime.strptime(out.get("at", ""), "%Y-%m-%dT%H:%M:%SZ")
    return out


def render(f):
    """an act's text from its fields — what the page hands to the wallet. kind, id and at lead, the rest follows."""
    head = [f"{k}: {f[k]}" for k in ("kind", "id", "at")]
    rest = [f"{k}: {str(v).replace(chr(10), ' ').strip()}" for k, v in f.items() if k not in ("kind", "id", "at") and str(v).strip() != ""]
    return "\n".join([HEAD] + head + rest)


# the flats that exist: the ids of the site's map. None on the bench of the tests; the gate and CI load the real ones
FLATS = None


def load_flats(map_json):
    """the valley's flat ids from the site's map (src/cyberia_map.json): a claim or a binding names one of them."""
    global FLATS
    FLATS = {f["id"] for f in json.loads(Path(map_json).read_text())["phase0"]}
    return FLATS


def empty():
    return {"roles": {}, "profiles": {}, "holders": {}, "claims": {}, "mandates": {}, "listings": {}, "ids": []}


def role_of(state, addr, root):
    if addr.lower() == root.lower(): return "root"
    return state["roles"].get(addr.lower(), {}).get("role", "")


def is_admin(state, addr, root): return role_of(state, addr, root) in ("root", "admin")


def _need(f, *keys):
    for k in keys:
        if not f.get(k): raise ValueError(f"{f['kind']} needs {k}")


def _money(v, what):
    if not re.fullmatch(r"\d{1,9}(\.\d{1,2})?", v or ""): raise ValueError(f"{what} is a number of dollars")
    return float(v)


def live_mandate(state, plot, agent, deal, day):
    """the mandate that lets `agent` list `plot` for `deal` on `day`, or None."""
    for mid, m in state["mandates"].items():
        if m["status"] == "live" and m["plot"] == plot and m["agent"].lower() == agent.lower() and m["scope"] in (deal, "both") and m["until"] >= day:
            if state["holders"].get(plot, {}).get("address", "").lower() == m["holder"].lower(): return mid
    return None


def _end_listings(state, plot, why, only_by=None):
    for l in state["listings"].values():
        if l["plot"] == plot and l["status"] in ("pending", "live") and (only_by is None or l["by"].lower() == only_by.lower()):
            l["status"] = "withdrawn"; l["reason"] = why


def apply(state, f, signer, root):
    """fold one act into the state, or ValueError when the signer may not do it. the state is changed only on success."""
    kind, at, me = f["kind"], f["at"], signer.lower()
    day = at[:10]
    if f["id"] in state["ids"]: raise ValueError("the id is used")
    if kind == "role":
        _need(f, "address", "role")
        if not ADDR.match(f["address"]): raise ValueError("not an address")
        if f["role"] not in ("admin", "agent", "none"): raise ValueError("a role is admin, agent or none")
        mine = role_of(state, signer, root)
        if f["role"] == "admin" or state["roles"].get(f["address"].lower(), {}).get("role") == "admin":
            if mine != "root": raise ValueError("only the root names and removes admins")
        elif mine not in ("root", "admin"): raise ValueError("only an admin accredits agents")
        if f["address"].lower() == root.lower(): raise ValueError("the root is not in the book")
        state["roles"][f["address"].lower()] = {"address": f["address"], "role": f["role"], "name": f.get("name", ""), "by": signer, "at": at}
    elif kind == "profile":
        _need(f, "name")
        state["profiles"][me] = {"address": signer, "name": f["name"][:80], "contact": f.get("contact", "")[:80], "at": at}
    elif kind == "claim":
        _need(f, "plot", "name")
        if not PLOT.match(f["plot"]): raise ValueError("not a flat")
        if FLATS is not None and f["plot"] not in FLATS: raise ValueError("no such flat in the valley")
        if state["holders"].get(f["plot"], {}).get("address", "").lower() == me: raise ValueError("this flat is bound to you already")
        state["claims"][f["id"]] = {"id": f["id"], "plot": f["plot"], "from": signer, "name": f["name"][:80], "contact": f.get("contact", "")[:80], "status": "pending", "at": at}
    elif kind == "holder":
        _need(f, "plot", "address")
        if not is_admin(state, signer, root): raise ValueError("only an admin binds a flat to its holder")
        if not PLOT.match(f["plot"]) or not ADDR.match(f["address"]): raise ValueError("a flat and an address")
        if FLATS is not None and f["plot"] not in FLATS: raise ValueError("no such flat in the valley")
        was = state["holders"].get(f["plot"])
        if was and was["address"].lower() != f["address"].lower():
            # the flat changed hands: what the old holder gave and listed ends with them
            for m in state["mandates"].values():
                if m["plot"] == f["plot"] and m["status"] == "live": m["status"] = "ended"
            _end_listings(state, f["plot"], "the flat changed hands")
        state["holders"][f["plot"]] = {"plot": f["plot"], "address": f["address"], "name": f.get("name", "")[:80], "by": signer, "at": at}
        for c in state["claims"].values():
            if c["plot"] == f["plot"] and c["status"] == "pending":
                c["status"] = "bound" if c["from"].lower() == f["address"].lower() else "rejected"
    elif kind == "mandate":
        _need(f, "plot", "agent", "scope", "until")
        if state["holders"].get(f["plot"], {}).get("address", "").lower() != me: raise ValueError("only the flat's holder gives a mandate")
        if not ADDR.match(f["agent"]) or f["agent"].lower() == me: raise ValueError("the agent is another address")
        if f["scope"] not in ("sale", "lease", "both"): raise ValueError("a mandate is for sale, lease or both")
        if not DAY.match(f["until"]) or f["until"] < day: raise ValueError("until is a day ahead")
        com = f.get("commission", "0")
        if not re.fullmatch(r"\d{1,2}(\.\d)?", com): raise ValueError("commission is a percent, 0 to 99.9")
        for m in state["mandates"].values():   # one live mandate per flat and agent: the new one replaces it
            if m["status"] == "live" and m["plot"] == f["plot"] and m["agent"].lower() == f["agent"].lower(): m["status"] = "replaced"
        state["mandates"][f["id"]] = {"id": f["id"], "plot": f["plot"], "holder": signer, "agent": f["agent"], "scope": f["scope"],
                                      "commission": float(com), "until": f["until"], "status": "live", "at": at}
    elif kind == "revoke":
        _need(f, "mandate")
        m = state["mandates"].get(f["mandate"])
        if not m: raise ValueError("no such mandate")
        if me not in (m["holder"].lower(), m["agent"].lower()) and not is_admin(state, signer, root): raise ValueError("the holder, the agent or an admin revokes")
        if m["status"] != "live": raise ValueError("the mandate is not live")
        m["status"] = "revoked"
        _end_listings(state, m["plot"], "the mandate was revoked", only_by=m["agent"])
    elif kind == "listing":
        _need(f, "plot", "deal", "price_usd")
        if f["deal"] not in ("sale", "lease"): raise ValueError("a listing is a sale or a lease")
        holder = state["holders"].get(f["plot"], {}).get("address", "").lower()
        if not holder: raise ValueError("the flat has no holder bound yet — claim it first")
        mandate = None
        if me != holder:
            mandate = live_mandate(state, f["plot"], signer, f["deal"], day)
            if not mandate: raise ValueError("only the holder lists the flat, or an agent holding a live mandate for this deal")
        price = _money(f["price_usd"], "the price")
        rent = _money(f["rent_usd_year"], "the rent") if f.get("rent_usd_year") else None
        term = f.get("term_years", "")
        if term and not re.fullmatch(r"\d{1,2}", term): raise ValueError("the term is a number of years")
        for l in state["listings"].values():   # one offer per flat and deal: the new one takes the old one's place
            if l["plot"] == f["plot"] and l["deal"] == f["deal"] and l["status"] in ("pending", "live"): l["status"] = "superseded"
        state["listings"][f["id"]] = {"id": f["id"], "plot": f["plot"], "deal": f["deal"], "price_usd": price, "rent_usd_year": rent,
                                      "term_years": int(term) if term else None, "words": f.get("words", "")[:600], "contact": f.get("contact", "")[:80],
                                      "by": signer, "as": "agent" if mandate else "holder", "mandate": mandate, "status": "pending", "at": at, "reason": ""}
    elif kind == "approve":
        _need(f, "listing")
        if not is_admin(state, signer, root): raise ValueError("only an admin approves")
        l = state["listings"].get(f["listing"])
        if not l or l["status"] != "pending": raise ValueError("no such listing waits")
        l["status"] = "live"; l["approved_by"] = signer; l["approved_at"] = at
    elif kind == "reject":
        _need(f, "target")
        if not is_admin(state, signer, root): raise ValueError("only an admin rejects")
        t = state["listings"].get(f["target"]) or state["claims"].get(f["target"])
        if not t or t["status"] != "pending": raise ValueError("nothing waits under this id")
        t["status"] = "rejected"; t["reason"] = f.get("reason", "")[:300]
    elif kind == "withdraw":
        _need(f, "listing")
        l = state["listings"].get(f["listing"])
        if not l or l["status"] not in ("pending", "live"): raise ValueError("no such listing is open")
        holder = state["holders"].get(l["plot"], {}).get("address", "").lower()
        if me not in (l["by"].lower(), holder) and not is_admin(state, signer, root): raise ValueError("the signer, the holder or an admin withdraws")
        l["status"] = "withdrawn"; l["reason"] = f.get("reason", "")[:300]
    state["ids"].append(f["id"])
    return state


# the files

def file_text(message, signature, signer):
    return f"---\nfrom: {signer}\nscheme: eip191\nsignature: {signature}\n---\n{message}\n"


def parse_file(text):
    m = re.match(r"^---\nfrom: (0x[0-9a-fA-F]{40})\nscheme: eip191\nsignature: (0x[0-9a-f]{130})\n---\n(.*)\n$", text, re.S)
    if not m: raise ValueError("not an act file")
    return m.group(3), m.group(2), m.group(1)


def fold(directory, root):
    """(state, errors) from every act file under `directory`, in the order of their names. the gate numbers a file as it
    commits it (`00000042-listing-6f1c2a9e.md`), so the order of the fold is the order of the commits and never the
    signers' clocks: two acts signed in the same second, or by clocks minutes apart, still fold the way they were taken."""
    state, errors = empty(), []
    for p in sorted(Path(directory).rglob("*.md"), key=lambda p: p.name):
        try:
            message, signature, signer = parse_file(p.read_text())
            f = fields(message)
            who = sign.recover(message, signature)
            if who.lower() != signer.lower(): raise ValueError(f"signed by {who}, filed as {signer}")
            apply(state, f, who, root)
        except Exception as e: errors.append(f"{p}: {e}")
    return state, errors


def seller(state, l):
    """who offers a listing, as the public sees it."""
    prof = state["profiles"].get(l["by"].lower(), {})
    name = prof.get("name") or (state["holders"].get(l["plot"], {}).get("name") if l["as"] == "holder" else "") or f"{l['by'][:6]}…{l['by'][-4:]}"
    return {"as": l["as"], "name": name, "contact": l["contact"] or prof.get("contact", ""),
            "accredited": state["roles"].get(l["by"].lower(), {}).get("role") == "agent"}


def public(state):
    """what the map reads: the live listings, with the seller's chosen name and contact and no address."""
    rows = [{"id": l["id"], "plot": l["plot"], "deal": l["deal"], "price_usd": l["price_usd"], "rent_usd_year": l["rent_usd_year"],
             "term_years": l["term_years"], "words": l["words"], "seller": seller(state, l), "at": l["at"]}
            for l in state["listings"].values() if l["status"] == "live"]
    return {"listings": sorted(rows, key=lambda r: r["price_usd"]), "updated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}


def view(state, addr, root):
    """what one address sees of the market: everything for an admin, its own flats, mandates, listings and claims otherwise."""
    me = addr.lower(); role = role_of(state, addr, root)
    out = {"address": addr, "role": role, "profile": state["profiles"].get(me)}
    if role in ("root", "admin"):
        out.update({k: list(state[k].values()) for k in ("holders", "claims", "mandates", "listings")})
        out["roles"] = list(state["roles"].values()); out["profiles"] = list(state["profiles"].values())
        return out
    held = {p for p, h in state["holders"].items() if h["address"].lower() == me}
    out["holders"] = [h for p, h in state["holders"].items() if p in held]
    out["claims"] = [c for c in state["claims"].values() if c["from"].lower() == me]
    out["mandates"] = [m for m in state["mandates"].values() if me in (m["holder"].lower(), m["agent"].lower())]
    out["listings"] = [l for l in state["listings"].values() if l["by"].lower() == me or l["plot"] in held]
    # the people behind my mandates, by the name they chose
    people = {m["agent"].lower() for m in out["mandates"]} | {m["holder"].lower() for m in out["mandates"]}
    out["profiles"] = [p for a, p in state["profiles"].items() if a in people]
    out["roles"] = [r for a, r in state["roles"].items() if a in people or a == me]
    return out


if __name__ == "__main__":
    if len(sys.argv) not in (4, 5) or sys.argv[1] not in ("verify", "public"):
        print(__doc__); sys.exit(2)
    if len(sys.argv) == 5: load_flats(sys.argv[4])
    st, errs = fold(sys.argv[2], sys.argv[3])
    if sys.argv[1] == "public":
        print(json.dumps(public(st), indent=1))
    else:
        for e in errs: print("FAIL", e)
        print(f"{len(st['ids'])} acts verified, {len(errs)} failed")
    sys.exit(1 if errs else 0)
