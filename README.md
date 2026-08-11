# cyberia.my — fleets & flats

Gesing land console: plots map, hard-force workers, intents (buy robot, lease / split / merge land).

Soft3-aligned experimental surface: open map data, local intent queue, no closed backend.

## Dev

```bash
trunk serve
# http://127.0.0.1:8090
```

## Deploy

```bash
nu scripts/deploy.nu
```

Rsyncs `dist/` → `cyberproxy:/var/www/html/cyberia.my/`.

### Domain

1. **DNS** — apex only: `cyberia.my` **A** → `167.235.28.94` (no www)
2. **nginx + TLS** — already on cyberproxy (`/etc/nginx/sites-enabled/cyberia.my`, certbot)
3. Public: **https://cyberia.my/**

## Earth states list

`/states` mirrors terrestrial countries from `../cyberstates/states/*.toml`
(non-aggregate, non–solar-system), sorted by capital (`money_supply_b_usd`).

Refresh snapshot after cyberstates market updates:

```bash
python3 scripts/sync-earth-states.py
# then rebuild / deploy
```

## Origin

Extracted from `cyberstates` `/cyberia` experimental page. Map data from Cyber Valley Gesing KML.
