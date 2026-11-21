---
tags: cyberia, project, cyberia-my, map
alias: the map, cartography, layers, drawing, my maps
crystal-type: plan
crystal-domain: cyberia
status: proposed
date: 2026-11-21
owner: master
serves: land
lane: wide
---

# cartography of our own

the owner, 2026-11-21: Google My Maps is cut down in every sense; the map we make must let people draw on layers of their own — a point, a plot, a building in 3D — with programmable layers and with verification of points and boundaries signed on a phone on the spot; a collaboration on the map of the world, and on our own layers what cyber valley and citadel genesis need. a prototype on git and signatures now, soft3 rails later: design soft3 first.

| field | value |
|---|---|
| delivery | on cyberia.my a person draws points, lines, plots and buildings on their own layers over the one globe, opens a My Maps KML as layers, shares a layer, verifies a boundary standing on it, and publishes by one signature |
| contract | `specs/map.md` |
| stage | proposed · the first slice below is built and green on a bench with throwaway keys (2026-11-21) |

## what the survey said

the tools read: Google My Maps, Felt, geojson.io, Placemark, uMap, Mapbox and MapLibre with Terra Draw, the OpenStreetMap editors (iD, JOSM, Rapid), QGIS with QField and Mergin Maps, ArcGIS Field Maps, Kart, Overture, and the proof-of-location projects.

| what people praise | where it comes from | here |
|---|---|---|
| any file dropped in becomes layers | Felt | slice 1: KML and GeoJSON |
| drawing with corners that can be moved, added, removed | Terra Draw, Placemark | slice 1 |
| per-feature history, nothing lost on a concurrent edit | OpenStreetMap, Kart | slice 1: every put names the version it replaces |
| style by a property, in the layer itself | My Maps, Felt, MapLibre expressions | slice 1: rules |
| offline first, with what is not synced in plain sight | QField, Mergin Maps | slice 1: drafts in the browser, a count of what is not signed |
| an accuracy reading that gates capture | QField, Field Maps | slice 1: six seconds, the better half, 25 m gate |
| snapping to corners and edges, right angles | Terra Draw, iD | slice 2 |
| shared corners: neighbours' boundaries move together | OpenStreetMap | slice 2 |
| the map, a table and the raw data in step; bulk edit | geojson.io, Placemark | slice 3 |
| live cursors, comments pinned to the map | Felt | slice 4 |
| one expression language for style, labels, forms, computed fields | ArcGIS Arcade, QGIS | slice 3 |
| layers that are queries over other layers | Felt, CARTO | slice 5 |

what none of them has, and this does: every feature and every change is a file named by its hash under a signature, and a boundary carries the signed word of the people who stood on it.

## slices

| # | slice | what a person can do after it | state |
|---|---|---|---|
| 1 | layers, drawing, signing | draw a point, a line, a plot, a building; move corners; props; open a KML or GeoJSON as layers; save a layer as a file; colour and rules; share with editors or open to all; private layers; verify on the spot; sign and publish | built · this pull request |
| 2 | precision | snap to corners and edges of any visible layer; right-angle and parallel assist; shared corners; undo and redo; a no-overlap check; holes | next |
| 3 | the table | a layer as a table beside the map; a form per layer (fields with types); bulk edit; computed props; labels by rule | |
| 4 | together, live | who is on the layer now, their cursors; comments pinned to a feature; a link that opens a feature | |
| 5 | derived layers | a layer that is a query over others (filter, join by containment, buffer, a sum per cell); the valley's own: open flats, titles, deals, straight from the acts of the market | |
| 6 | stronger evidence | a photograph hashed into a statement; an RTK receiver over Bluetooth; a neighbour co-signing a shared line; a statement asked for by a link sent to a phone | |
| 7 | the valley on it | the valley's map layer by layer from My Maps into layers the company owns; `cyberia_map.json` generated from them; citadel genesis: its zones, flats, roads, buildings | needs the owner's KML and a word on who owns the layers |
| 8 | soft3 rails | files by particle, acts as signals, links as §5 of the contract says | after the stack's signal lands |

## open

| question | where it is decided |
|---|---|
| who owns the valley's layers: the owner's wallet, or a company address with editors | the owner |
| whether a drawn plot on a company layer becomes a flat of the market (the acts of `projects/listings.md` name flats by the ids of `cyberia_map.json`) | slice 7 |
| a private layer is hidden, not encrypted; the acts repository's readers see it | slice 8, or sooner when asked |
| the preview host of a pull request has no gate behind it, so a wallet cannot publish there; drawing, files and drafts work | apparatus: `scripts/patch.py` |
