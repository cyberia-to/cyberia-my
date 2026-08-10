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

1. **DNS** — point `cyberia.my` (+ optional `www`) **A** to cyberproxy: `167.235.28.94`
2. **nginx** — install `scripts/nginx-cyberia.my.conf` on cyberproxy (see comments in file)
3. **TLS** — `sudo certbot --nginx -d cyberia.my -d www.cyberia.my`

## Origin

Extracted from `cyberstates` `/cyberia` experimental page. Map data from Cyber Valley Gesing KML.
