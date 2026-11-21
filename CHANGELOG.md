# changelog — cyberia.my

The product's versions, three numbers each (`~/cyber/AGENTS.md` § release train). The door states the
version it serves in `index.html` (`<meta name="version">`) and on `/menu`; the crate's `version` in
`Cargo.toml` is the same number, and `tests/version.rs` holds the three in step. A bump is one pull
request, `chore: cyberia.my <version>`.

## 0.2.0 — 2026-11-21

The console of 0.1.0 became the land console of cyberia: one map from the planet to a flat, a market
of leaseholds carried by signed acts, and the papers under them.

- the globe — a WebGL2 engine in the app: imagery tiles on a sphere, Natural Earth lines, cyberia
  layers at depth, cities as markers, one camera from planet to plot; the land console and the globe
  are one map, and on a phone the map is the page
- the earth under it — divisions as layers searched by name down to a village, imagery deeper than
  zoom 7 and to zoom 14 around a city from this year's Sentinel-2 scenes, the fog of war where the
  pictures do not know the land, the poles
- the land market — holders and their agents put flats on the map by signed acts, the valley's desk
  approves; the flat page made to be wanted, pick and pay in one line, land classes on the plan
- MY PLOTS — a person's flats as a collection of tokens over the valley
- the desk — `/admin` rebuilt from what the company controls: six menus, deals as signed acts, a
  register of two levels (the title, then the flats cut from it)
- the cabinet — the holder's room at `/cabinet`: steps, flats, identity, help, and how to get a
  wallet when the browser has none
- the library — the land and company documents on IPFS with their content hashes, every file a book
  with its cover, the signable pack as PDFs on the letterhead, one Hak Sewa instrument with Annexes
  A–G, the text in Bahasa Indonesia with fourteen second languages beside it, the content hash as a
  QR
- payment — premium or upfront to the Company's bank account, the ETH rail (ETH or USDT on mainnet),
  sent against received
- hard force — survey and media orders through `@cybervalley_hardforce_bot`: orders become tasks in
  the crew's group, results pinned to IPFS and published to the flat's data
- one crawlable page per flat — title, description, JSON-LD, `plan.svg`, ERC-721 metadata, sitemap
  and robots
- the adviser — the player sits in the middle of the header, the Atlas Shrugged set from the site
  itself; the lytics tracker on the landing
- the identity is the real stack — particles are hemera hashes, the neuron a mudra domain-scoped
  key, signals carry ADR-036 signatures, and `tests/domain_compat.rs` freezes the envelope

## 0.1.0 — 2026-08-10

The first console, never tagged: the cities catalog as the landing, the Gesing plots map with the
lease book and flat pages, the workers of hard force, the signal studio, the earth states list, and
TLS on the apex.
