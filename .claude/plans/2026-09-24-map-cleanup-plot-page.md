# map cleanup + plot page — 2026-09-24

Session goal (user, verbatim intent): big improvement pass on cyberia.my/map.
Work until done. Uncommitted work already in tree: render panel (real polygon,
worksite dot, GPS/tap), query sync, pointer-capture click fix.

## blocks

1. nav — keep YOU · CITIES · MAP visible. Hide world, studio, elements,
   products, genetics, services, orgs, calendar, robots, plots, places,
   domains, states → reachable at /menu/<name> (+ /menu index). Old routes stay.
2. fleets rail — delete workers witaya, lupus, surya, doplang, tika, sastra,
   darma, darsana, pande (both console.rs and robots.rs rosters). Delete
   MACHINES entirely. Then hide the whole left rail on /map (keep it for
   /domains board, it hosts the DOMAINS tab).
3. dock — delete BUY ROBOT, SPLIT LAND, MERGE LAND buttons + their sheets +
   split/merge helper code. LEASE LAND → navigate to /plot/<selected>/lease.
   If the flat is owned / in transfer → popup with creative copy.
4. right column (RENDER+INTENT) +20%: 270px → 324px.
5. land colours — stroke = district colour, fill = status colour (prysm
   emotion palette: owned #00fe00, transfer #fcf000, available #00acff),
   bright. Status source: name has "@handle" → owned; local deal not signed →
   transfer; deal signed → owned; else available. Legend updated.
6. plot page /plot/:id (new src/plot.rs + src/deals.rs), base = cyberia-map:
   - hero: name, zone, m², status, owner
   - LAND: 2 m grid on the real polygon; tools MEASURE (two taps → m),
     PRUNING (toggle cells), BUILD UNIT (2×2×2 m shower+toilet cube → commit)
   - other figures (CUBE-4, TUBE, PRYSM, PYRAMID, SPHERE) grey: placeable,
     build locked
   - DEAL: current status card first, history below; steps
     request → review → docs → transfer → signed; NEXT STEP / CANCEL (soft3)
   - DOCS: upload (localStorage, ≤1 MB data URL), download uploaded,
     download plot passport (JSON) + KML
   - /plot/:id/lease = same page, deal panel first, auto-creates request
   - link from map RENDER meta: "open flat →"

## status
- [x] 1 nav  - [x] 2 rail  - [x] 3 dock  - [x] 4 width  - [x] 5 colours  - [x] 6 plot page

All six blocks done 2026-09-24, verified headless (CDP): tools, deal steps,
LEASE nav + taken popup, docs upload/download, /menu/<name>. Not committed.
Files: src/{nav,deals,plot,plot_board,plot_land,plot_docs}.rs, console.rs,
robots.rs, app.rs, main.rs, style.css, Cargo.toml (web-sys File* features).

## follow-up 2026-09-24
- INTENT panel on /map replaced by FLAT panel: name, district·area, status,
  holder; AVAILABLE → LEASE THIS FLAT; not yours → REQUEST DOCUMENTS (due
  diligence, `cyberia.dd.v1`, logs a `docs` intent); yours → MANAGE link.
  "YOUR REQUESTS" log below (intents). ACTIONS/selected_action/next_id gone.
- Running dev server: only ONE `trunk serve` (killed the Sep-16 daemon);
  never run `trunk build` while it serves — the race leaves dist/ with
  integrity-mismatched index.html and the page renders empty.

## lease book 2026-09-24
- Sheet 1dJ_3rrDhc85y34wYbziVwjnRiFJLwzP4IW13oCfZzGo → `scripts/sync-plot-terms.py`
  → `src/plot_terms.json` (93 rows) → `src/terms.rs` (include_str).
- Status: live/signed local deal → transfer/owned; sheet owner (≠ cyber
  valley) → owned; private + price + no owner → available; else NOT FOR LEASE
  (gray #4b4b4d). KML `:@handle` in names is display noise → stripped.
- Result: 39 available (sinwood 30 + avalon-7..15), 2 owned (sinwood-24/30,
  dzin), 85 not for lease. Open question to user: avalon on offer? (memory
  says first wave = sinwood only).

## land use 2026-09-25
- User legend (the map's colours): green lease · orange joint venture /
  business · violet city commons · blue HGB sale of a whole district (bridge,
  $3,500/are) · yellow dual use (business + housing).
- Sync now reads My Maps plot fills (`LAND_USE` table in the script) +
  `DISTRICT_SALE` override for bridge; every map plot gets a `land_use`.
- Fill = land use (prysm hues), strength = status; owned → white hatch,
  in transfer → marching white dash; outline = district.
- Status: Available (lease/dual, priced, free) · OnRequest (venture, hgb) ·
  Transfer · Owned · Closed. Requests (jv / hgb) in `cyberia.jv.v1`,
  district key `district:<zone>`.
- Open: #c2185b (etherland-0/1/8, asgard-8) has no meaning yet → UNCLASSIFIED.

## rulings 2026-09-25 (user)
- #c2185b = special place (high energy) → land use `special`, red, pulsing,
  not on offer: "HIGH-ENERGY GROUND".
- bridge stays green/violet in My Maps: those colours are the buyer's marks of
  special-purpose places → `buyer_mark`, shown in the HGB card.
- sheet community/commons beats a lease/dual colour → recoloured to commons
  (sinwood-25/26/31/32/39), `map_use` keeps the original.
- orange in sinwood = construction business, not private (sinwood-2…5);
  sinwood-1 laba = the city's construction business, open to a JV.
- etherland-6/14 = commons (map colour already says so).
- lease/dual without a price → grey, "opens with the second wave".
