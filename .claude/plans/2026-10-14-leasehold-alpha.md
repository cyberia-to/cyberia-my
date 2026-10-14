# leasehold alpha — 2026-10-14

Goal (user): launch the alpha of the land product by the end of the week —
cyberia.my/map + plot pages + the lease pack + graph pages. The user's
critical list is the spine of this plan; items are ordered by dependency.

## where things stand (from the transcripts of 09-25, 09-28, 10-12, 10-13)

- map: 127 plots, prysm land use, lease book from the sheet, trails, friends'
  sales, no popups; `/plot/:id` = hero · LAND board · DEAL track (local) ·
  DOCS (passport JSON, KML, uploads in localStorage).
- pack: `cve/legal/10 hak sewa/` — template ID/EN, akta simple, Annexes A–G,
  reading pack EN-RU (PDF). Public copies on cyberia.blog under
  `cyber-valley/products/leasehold/` (template, simple, perjanjian, design
  code). Policies, referral system, rates, century index — all live.
  Open in the pack: bracket values (fees, terms, garden/pond/water norms),
  the KEPMEN number on HGB 00048 page 1, notary read of akta + agreement.
- player: Atlas Shrugged set already on every page (cyberia.to/player.js,
  download at /music/atlas.shrugged.set.mp3). A new track swaps in when sent.
- gaps against the user's list: every plot URL serves one generic <title>;
  robots.txt / sitemap.xml fall through to index.html; no survey / media
  per plot and no ORDER button; no Telegram flow; /me is a mock economy
  dashboard, not a client cabinet; no document library; reading-pack PDF
  link is dead for outsiders (cve went private 09-28); `company licences`
  and `office practice` pages exist only in private cve; no wallet connect;
  no NFT side of the plot page; nothing on IPFS.

## task pool

1. SEO per plot — static `plot/<id>/index.html` with its own title,
   description (district · land use · size in ares · m² in plan as on the
   Google map · surveyed m² when known · price), canonical, og:*, JSON-LD;
   `sitemap.xml`, `robots.txt`; the same shell loads the SPA. Script
   `scripts/render-plot-pages.py`, called by `deploy.nu` after the build.
2. survey + media on the plot page — `measured_m2`/`measured_on` already in
   the book; a SURVEY card (figure + date, or ORDER SURVEY) and a MEDIA card
   (photos / drone video, or ORDER PHOTO · DRONE). Media comes from
   `/plot-data/<id>.json` fetched at runtime so the bot can publish without
   a rebuild. ORDER = Telegram deep link `t.me/<bot>?start=<kind>_<plot>`;
   until the bot runs, a DM to @st_joy with the text prefilled.
3. client cabinet `/cabinet` — every status in one place: deal track, due
   diligence, KYC, wallet, survey, media, documents; orders log; support
   links (DM @st_joy, group). All state in the browser, nothing on our server.
4. document library `/docs` — templates and the pack (graph links), rules
   pages, century index, rates; land + company documents for the deal as
   IPFS links (Pinata) once pinned; the reading-pack PDF pinned to IPFS.
5. wallet connect — EIP-1193 (`eth_requestAccounts`), address shown in the
   cabinet and on the plot page; CYB neuron already exists (mudra key).
6. NFT side of the plot page — `plot/<id>/metadata.json` (ERC-721 metadata:
   name, description, image, attributes) from the same renderer; ON-CHAIN
   card on the page (contract + token id once deployed). Contract and chain
   need the user's answer (chain, deployer, DAO address).
7. hardforce bot — one process (Python std lib, long polling): `/start`
   payload → task posted to the hardforce group → media replied in the group
   → pinned to Pinata → `plot-data/<id>.json` pushed to the pages repo →
   live within a minute. Needs: bot token, group id, Pinata JWT.
8. pack — pin the EN-RU reading pack; publish `company licences` and
   `office practice` to the public graph (company data is public by the
   user's rule); the bracket values stay the user's decisions.

## needs from the user

- Telegram: bot token (BotFather), the hardforce group (add the bot, send
  its id), the support handle to show (default @st_joy).
- Pinata JWT for pinning the pack and the deal documents.
- NFT: chain (Ethereum mainnet / Base / testnet), who holds the deployer
  key, the DAO address to own the collection.
- the music track to replace or join the Atlas Shrugged set.
- bracket values in the pack; the KEPMEN number from HGB 00048 page 1.

## status

- [ ] 1 seo  - [ ] 2 survey/media  - [ ] 3 cabinet  - [ ] 4 docs
- [ ] 5 wallet  - [ ] 6 nft metadata  - [ ] 7 bot  - [ ] 8 pack
