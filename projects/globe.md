---
tags: cyberia, project, cyberia-my, map
alias: globe, the globe, cyberia.my globe, the map engine
crystal-type: plan
crystal-domain: cyberia
status: executed
date: 2026-11-17
owner: master
serves: land
---

# the globe

the owner, 2026-11-17: the home of cyberia.my becomes a globe, and the map is the primary way to use the site. a simplified vector earth like the one nodes are drawn on, detailed inside: the land divided into its administrative districts, a plain satellite bitmap that shows something when you zoom, and at the deepest zoom what cyberia.my/map shows today — plots, trails, places. everything served from our own machines, no external API.

| field | value |
|---|---|
| delivery | cyberia.my/ is the globe; zoom carries one camera from the planet to the plots of a city |
| date | — |
| stage | proposed |
| repository | cyberia-to/cyberia-my (engine and app), build scripts for the data in the same repository |

## data, all open, all ours to serve

| layer | source | licence | measured / estimated |
|---|---|---|---|
| land, coast, oceans | Natural Earth 1:10m | public domain | ~30 MB |
| districts | geoBoundaries gbOpen, all levels | CC BY, CC BY-SA, ODbL, PD by country | measured 2026-11-17: 952,093 units — ADM0 230, ADM1 3,281, ADM2 49,363, ADM3 105,170, ADM4 94,251, ADM5 699,798 |
| where geoBoundaries stops early | OpenStreetMap administrative boundaries | ODbL | Indonesia is ADM2 only (519 kabupaten) in geoBoundaries; the desa of Bali come from OSM |
| imagery, planet | NASA Blue Marble Next Generation, 500 m | public domain | zoom 0–7, a few GB |
| imagery, land detail | Sentinel-2 cloudless 2016 (EOX) | CC BY 4.0 | estimated: zoom 12 ~50 GB, zoom 13 ~200 GB (land tiles, WebP) |
| a city | our own drone orthophoto | ours | Cyber Valley first |
| cyberia | the cities, plots, trails, places of the land console | ours | today's `cyberia_map.json`, `cyberia_trails.json` |

GADM is left out: its licence forbids redistribution. the attribution panel names every source per layer, as the licences require.

## serving

every layer is one PMTiles file: a single archive of tiles read by HTTP range requests, so nginx on front-primary serves it as a static file — no tile server, no API, nothing to keep alive. vector tiles (land, districts, cyberia) are built with tippecanoe, imagery with GDAL; the build runs on build-host (96 cores, 853 GB free), the archives go to front-primary (820 GB free).

## the engine

Rust to wasm, WebGL2, in the same trunk build as the site — no node, no map library.

- one camera, three projections, continuous: a globe from the whole earth to about zoom 5, morphing into Web Mercator, and from about zoom 16 the local metric plane of a site that the land console already uses (to the 10 cm tile). zoom is one number from planet to plot.
- tiles: a quadtree of visible tiles per frame, fetched from the PMTiles archives by range, decoded in a worker, cached on the GPU with an LRU; the parent tile drawn while a child loads, so the screen is never blank.
- vector: polygons triangulated once per tile, strokes as screen-width lines, fills by level of the district hierarchy; the selected district highlighted on the GPU, no redraw of geometry.
- labels: a glyph atlas and collision on the CPU, few per screen, ranked by district level and size.
- cyberia layers draw over the same camera; the console's plot rendering becomes the last zoom range of the engine instead of a separate SVG page.

## navigation

- drag rotates the globe and pans the map; wheel and pinch zoom around the pointer; double click flies to the district under it.
- click selects a district: a side card with its name, its chain of parents, the cities and plots inside, and actions (go to the city, found a city here).
- search: a static index of district names, sharded by prefix, answers name → box, flies there.
- the address is the view: `/@lat,lon,zoom` and `/d/<district>`; every district gets a particle (hemera of its source id), so it can be linked in the [[cybergraph]].
- the list of cities becomes a panel on the globe; their markers are the first thing seen on the planet.

## order, one change per pull request

1. the engine with the globe, Natural Earth and Blue Marble, and the city markers; `/` is the globe.
2. districts from geoBoundaries and OSM, selection, search, the address.
3. Sentinel-2 imagery to the chosen zoom.
4. the land console as the deepest zoom range; /map becomes a view of the globe.

## built, 2026-11-17

live at https://cyberia.my/ — pull requests cyberia-my #6, #7, #8. the four steps above landed as one map:

- the engine is `src/globe.rs`: WebGL2 through web-sys, no map library. one camera from the planet to two metres across; the globe is a ball in the hand — any heading at any zoom, the point under the finger stays under it, a thrown ball keeps turning; keys are held, eased, read by their place on the keyboard, and written on the buttons of one pad.
- the land console is the deepest zoom range: its plan lies over the imagery and shares the camera; /, /cities, /map, /domains and `/@lat,lon,zoom[,heading]` are the same map.
- imagery from our own disk: the Sentinel-2 mosaic to zoom 7 for the earth and to zoom 14 for Bali and Armenia; the poles have their own pyramid (north the same mosaic, south of 60°S Blue Marble).
- the divisions: 1,040,641 units in six levels. geoBoundaries gbOpen for the earth; gbHumanitarian for Indonesia's kecamatan and desa (in place of OpenStreetMap, which the plan named); Natural Earth's breakaway areas and Iraq's northern governorates for the five states cyberstates names. a tap gives the point and the unit under it at every level; each level is selected by its row; the right panel follows the plot panel's plan and shows the area; the lines of the level the zoom reads are drawn; a place is found by name.

where the build left the plan:

- files, not PMTiles: imagery tiles, unit files, index cells, line cells and search shards are plain static files under `/tiles/`. the engine reads them directly, so tippecanoe and vector tiles were not needed.
- no parent links are stored: every level is asked for the unit under the point.
- Natural Earth is no longer drawn; states are named as cyberstates names them.

not built yet: names on the map (the glyph atlas), the address `/d/<district>` and a particle per unit, imagery deeper than zoom 11 outside Bali and Armenia, a measured page check of the acceptance list below.

### deep imagery and layers, 2026-11-18

pull requests cyberia-my #17, #18.

- the world's land to zoom 11 (76 m a pixel at the equator, sixteen times the old zoom 7). `scripts/fetch-imagery.py` descends by content: every tile with a picture to zoom 9, beyond it the children of a rich tile (a JPEG over 2,600 bytes, which open sea is not). the source answers with a blank white tile where it has nothing — deep sea, the ice sheets; those are not kept. measured: 52,532 tiles at zoom 8, 206,121 at 9, 247,828 at 10, 944,264 at 11; 1,497,617 asked, 15.6 GB kept, 19 GB for the whole pyramid on the front. 26 tiles the source refuses (http 403); the engine shows their parents.
- `have/{z}.rle` says which tiles of a deep zoom exist — run lengths over the zoom's tiles, 47 KB for zoom 11 — so the engine asks for nothing in vain and falls back to the parent. `manifest.json` names the span as `deep`.
- the source gives about fifty tiles a second to one address whatever the threads, so the fetch took eleven hours. it ran on the build host; the tiles went to the front by rsync straight between the two machines, with a key made for the push, bound on the front to that one directory, and removed after.
- layers: a tree on the pad (key L). earth: satellite, grid; borders: states, regions, subregions, local; cyberia: cities, franchise, city outline, plots. a group switches its leaves together; the choice is kept in the browser. the grid is drawn by the engine at a step that follows the zoom.
- the source stays Sentinel-2 cloudless 2016 (CC BY 4.0): the later years are CC BY-NC-SA.

the data is built by `scripts/build-divisions.py` on the build host and copied to the front — never built on the front, which has 4 cores and 8 GiB: on 2026-11-17 a 48-job run there got its container killed and re-created, about two minutes of outage for every site on it.

## open for the owner

- the imagery budget: the land of the earth is at zoom 11 in 19 GB. zoom 12 would be about four times that and does not fit the front's disk (98 GB) beside the rest; a state at a time fits — Indonesia to zoom 14 is the next candidate, the franchise being open there.
- the share-alike layers (ODbL, CC BY-SA countries) oblige the tiles built from them to carry the same licence; the default here is to accept that and say so in the attribution panel.

## acceptance

- a cold load of `/` shows the globe in under two seconds on a phone, with no request leaving our domains (a page check lists every request);
- zooming from the planet to a plot in Cyber Valley passes through the district, the imagery and the plots without a blank frame;
- clicking any district on earth shows its name and parents; search finds it by name;
- 60 fps on an M-series laptop and above 30 on a mid phone at every zoom.

related: [[cyberia/projects/cyberia-my|the land console]] · [[cybergraph/projects/studio|studio]]
