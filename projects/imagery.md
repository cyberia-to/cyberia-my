---
tags: cyberia, project, cyberia-my, map, imagery
alias: imagery, the imagery resource, satellite tiles
crystal-type: plan
crystal-domain: cyberia
status: proposed
date: 2026-11-18
owner: master
serves: land
---

# imagery: a resource of our own for satellite pictures

the owner, 2026-11-18: "maybe we think about a resource for serving satellite imagery?" — this page is that thought, for a decision.

## where the pictures live today

on the front's disk, beside every site it serves: 98 GB in all, 4 cores, 8 GiB. measured on 2026-11-18, the Sentinel-2 cloudless 2016 mosaic by EOX (CC BY 4.0), fetched by `scripts/fetch-imagery.py`:

| zoom | metres a pixel | tiles kept | on disk |
|---|---|---|---|
| 8 | 611 | 52,532 | 0.4 GB |
| 9 | 306 | 206,121 | 1.5 GB |
| 10 | 153 | 247,828 | 3.6 GB |
| 11 | 76 | 944,264 | 12.9 GB |

the land of the earth stops at zoom 11; Bali and Armenia go to zoom 14, Indonesia is being added to zoom 14. the source's own depth is zoom 14, ten metres a pixel.

## what stands in the way of the whole earth at ten metres

projected from the growth between zooms 10 and 11 (3.8 times the tiles, about 14 KB a tile) — a projection, to be measured:

| zoom | metres a pixel | tiles | disk | fetch at 50 tiles a second |
|---|---|---|---|---|
| 12 | 38 | ~3.6 million | ~50 GB | ~20 hours |
| 13 | 19 | ~14 million | ~190 GB | ~3 days |
| 14 | 10 | ~52 million | ~700 GB | ~12 days |

- disk: about a terabyte for the whole earth to zoom 14. the front has 63 GB free.
- the fetch: the source gives about fifty tiles a second to one address whatever the threads (measured), so the whole earth is some sixteen days of polite fetching.
- the mosaic's year: 2016 is the one year under CC BY; 2018 and later are CC BY-NC-SA, closed to a site that sells land.

## the resource

three parts, each useful without the next:

1. a machine for tiles: its own disk of two terabytes or more, nginx over static files, nothing else on it. the front keeps `/tiles/` as the address every page asks (no request leaves our domains) and passes it to the machine with a cache on its own disk for what is asked often. imagery, the polar pyramid and the divisions move there; the front gets its disk back. a second copy on the secondary front's side when there is one.
2. the earth to zoom 14 from the same mosaic, fetched onto that machine zoom by zoom, the states where the franchise is open first.
3. a mosaic of our own: the Sentinel-2 scenes themselves are open for any use, commerce included, under the Copernicus terms, and are published as open data. a mosaic built from them is ours to licence, of the year we choose, renewed when we choose. this is the heavy part — terabytes of scenes, cloud masking, colour — and the one that makes the resource more than a mirror: other cyberstates and anyone else could be served from it.

## open for the owner

- the machine: lease one for tiles (two terabytes, modest cores), or put a disk under the build host's lease. the build host has 831 GB free today, but it is the release train's and has been closed once for an empty escrow; the default here is a machine of its own.
- the depth of step 2: zoom 12 for the earth and zoom 14 for the franchise states (about 60 GB more), or the whole earth to zoom 14 (about a terabyte). the default here is the first, the second after the machine stands.
- whether the resource is open to others from the start (a public address such as tiles.cyberia.my with the attribution the licence asks), or only behind our own pages. the default here is behind our pages until step 3.

## acceptance

- every tile a page of ours asks comes from our own machines, at the same address as today;
- the front's disk holds no pyramid, only the cache;
- any land on earth shows ten metres a pixel, with the source and licence named on the map.

related: [[cyberia-my/projects/globe|the globe]]
