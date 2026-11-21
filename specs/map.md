# The map its people draw

The contract of cyberia.my's own cartography: what a layer and a feature are, how
they are changed, how a point or a boundary is verified by someone standing on it,
and how all of it moves onto soft3 without changing shape. The reference
implementation is `gate/layers.py` (the fold and its rights), `src/atlas.rs` (the
model in the browser) and `src/draw.rs` (the tools). What was observed is in
`audit/`; what is planned and in what order is `projects/map.md`.

## 1. What it is for

The valley's map was kept in Google My Maps. It holds ten layers, has no snapping,
no shared boundaries, no height, no history, no offline, no API, and nothing in it
is anyone's signed word. This replaces it with the same globe the site already
shows, drawn on by its people:

- a person draws on layers of their own — a point, a line, a plot, a building;
- a layer is shared by its owner with editors, or opened to everyone;
- a point or a boundary is confirmed or disputed by a person standing on it,
  with their phone's position, under their signature;
- a layer says how it paints itself by rules over its features' properties;
- nothing is stored but files and signed acts, so the map can be recomputed by
  anyone who holds them.

## 2. The model

| thing | what it is | identity |
|---|---|---|
| file | bytes | the hash of its bytes |
| feature | a point, a line, a plot or a building, with properties | a random id, stable for its life; each state of it is a version, the hash of its canonical bytes |
| changeset | a file: features put, features removed, on one layer | the hash of its bytes |
| layer | a set of features, with an owner, editors, a definition | a random id |
| definition | a file: the layer's colour and its rules | the hash of its bytes |
| act | a short text a wallet signs, naming files by hash | its own id, and its place in the order of commits |
| statement | an act: confirm or dispute, about one version of one feature | the act |

The state of the map is the fold of the acts in the order they were committed.
There is no database.

### 2.1 A feature

```json
{"id": "<uuid>", "kind": "plot", "geom": [[1150890444, -82973027], …], "props": {"name": "…"}, "prev": "<version>"}
```

- `kind`: `point` (one pair), `line` (two or more), `plot` and `building` (a ring of
  three or more, written without its closing pair).
- `geom`: pairs `[lon, lat]` in degrees × 10⁷ — whole numbers, about a centimetre.
  The frame is the earth's, WGS84. No float is ever hashed.
- `props`: short lowercase keys; a value is text or a whole number. A building
  carries `height_cm` and `base_cm`: it is a prism, a ring between two heights.
- `prev`: the version this state replaces; empty for a new feature.
- a version is `sha256` of the canonical bytes of `{id, kind, geom, props}`.
- canonical bytes: JSON with sorted keys, no spaces, UTF-8.

Limits: 5000 points in a feature, 60 props, 5000 features and 2 MiB in a changeset.

### 2.2 The acts

```
cyberia.my map
kind: change
id: <uuid>
at: 2026-11-21T10:00:00Z
layer: <uuid>
file: sha256:<hex>
put: 3
remove: 0
```

| act | who may sign it | what it does |
|---|---|---|
| `change` | the layer's owner or an editor; anyone on an open layer, for features they drew | applies one changeset. The first change of an unknown layer founds it (`name:` is required) and its signer owns it |
| `layer` | the owner | name, about, `public yes|no`, `open yes|no`, `file:` the definition |
| `grant` | the owner | `address`, `role editor|none` |
| `attest` | anyone who may see the layer | a statement: see §4 |

The wallet shows the text; the counts in it must be the file's, so what a person
reads is what they sign.

### 2.3 Editing together

- A put names the version it replaces. When that is no longer the feature's head
  the gate refuses the whole change, and the page says which feature moved. The
  person reloads, or signs their shape over the newer one on purpose. Nobody's
  edit is lost silently.
- A removed feature does not come back under the same id.
- Every version stays: the feature keeps its history (version, signer, time, file).
- The page is local first. What is drawn is a draft in the browser, shown at once,
  kept through reloads and with no network; SIGN & PUBLISH sends a layer's pending
  changes as one changeset under one signature.

### 2.4 Seeing

A layer is public unless its owner says `public: no`; then only the owner and the
editors are given it, by their session. The acts repository is private either way.
A private layer is hidden from readers, not encrypted.

## 3. A layer that paints itself

A definition is `{"color": "#rrggbb", "rules": [{"when": "<condition>", "color": "#rrggbb"}]}`.
The first rule whose condition holds paints the feature; otherwise the layer's colour.

A condition is one or more comparisons joined by `and`:

```
status == sold
confirm > 0
kind == plot and area_m2 > 1000
name ~ temple
```

Operators: `==` `!=` `>` `<` `>=` `<=` `~` (contains); a bare name holds when the
value is present and not zero. A name is a prop, or one of what the map knows of a
feature: `kind`, `area_m2`, `length_m`, `height_m`, `confirm`, `dispute`, `on_spot`.

Conditions are pure: the same feature and the same definition give the same colour
everywhere. This is the first rung of programmable layers; the next ones are in
`projects/map.md` (computed props, filters, layers derived from other layers).

## 4. Verification on the spot

A statement is an act about one version of one feature:

| field | meaning |
|---|---|
| `feature`, `version` | what is spoken of; the version must be the head |
| `statement` | `confirm` or `dispute` |
| `role` | `holder`, `neighbour`, `witness`, `surveyor` — the signer's own word |
| `method` | `phone` (the browser's position), `rtk`, `manual` (no position) |
| `lat_e7`, `lon_e7`, `accuracy_cm` | the position read, and the accuracy the device reported |
| `samples`, `spread_cm` | how many readings were averaged and how far apart they lay |
| `vertex` | the corner the signer stands at, when they stand at one |
| `note` | free words |

The gate computes `distance_cm` itself: from the position to the named corner, or
to the feature's nearest corner. A statement counts as made on the spot when the
position lies within twice the reported accuracy (at least 6 m) of what it speaks of.

- A signer has one word per feature and corner; a new one replaces it.
- Statements hold for the version they saw. When a boundary moves, it stands
  unverified again.
- The page reads the position for six seconds, keeps the better half of the
  readings, and refuses a confirmation coarser than 25 m. A dispute can always be
  signed.

What this proves and what it does not. A phone's browser attests that this key
signed this reading, with this reported accuracy, at this time. It does not prove
the person was there: a position can be spoofed. The standing of a boundary
therefore comes from several signers and from who they are — the holder, the
neighbours across each line, a surveyor — and never from one reading. Stronger
evidence is planned and named in `projects/map.md`: a photograph hashed into the
statement, a receiver with RTK, a neighbour's co-signature of a shared line.

## 5. The same shape on soft3

The prototype runs on git and wallet signatures. Each piece has its place on the
stack, and the data is laid out so that the move renames identities and changes
nothing else.

| here | on soft3 | what the move changes |
|---|---|---|
| a file named `sha256:…` | a file: `(particle, data)`, the particle being the hemera hash of its bytes | the name; the bytes stay. Names carry their scheme so the two never mix |
| canonical JSON, whole numbers | the canonical value encoding (`soft3/specs/types.md`), fixed-point integers | the encoding; the values are already integers at a declared scale |
| an act: one signer, one step, files named by hash | a signal: a bundle of cyberlinks a neuron commits in one step with one proof | the envelope: network, height, the previous signal |
| a wallet's EIP-191 signature (secp256k1) | a neuron: the hemera hash of the compressed public key, signing by mudra | the same key signs; its identity is derived differently |
| the order of commits | the order of signals | — |
| "feature F is on layer L" | a cyberlink `L → F` | — |
| "version V replaces U" | a cyberlink `U → V` | — |
| a feature removed | the same link with valence −1; nothing is erased | — |
| confirm, dispute | a cyberlink from the version to the observation's file, valence +1 or −1 | — |
| a relation's meaning ("on layer", "replaces") | a link from the pair's axon to a relation word (`neural/specs/relation.md`) | no type field is ever put on a link |
| a building: a ring between two heights | the Prism of `cyberia/protocol/space-accounting.md` | — |

Rules the prototype already keeps because the stack requires them: append only;
identity from bytes, never from a name or a place in a list; no float in hashed
bytes; who and when belong to the envelope, not to the data; the page is a view and
has no authority.

What the prototype does not do and the stack will ask for: exclusive plots
(disjoint, children inside the parent, challengeable claims) — a drawn plot here is
a drawing, not a title; and the frame, which is implicit (the earth, WGS84).

## 6. The gate

```
GET  /api/map/layers          the layers this address may see
GET  /api/map/layer/<id>      one layer whole: features, their standing, the statements
POST /api/map/act             {message, signature, file} → {ok, layer}
```

Acts are committed as `map/acts/<seq:08d>-<kind>-<id8>.md` and files as
`map/files/<aa>/<hex>.json` in the acts repository. `python3 gate/layers.py verify <dir>`
recomputes the map from them: every signature, every right, every file's name.

## 7. Files in and out

In: KML (every folder becomes a layer — the way My Maps writes its layers) and
GeoJSON (Point, LineString, Polygon and their Multi forms). A polygon's holes are
not carried yet; a KMZ is a zip and must be unpacked first. Out: GeoJSON and KML,
per layer.
