# hard force bot

Orders from a flat's page on cyberia.my become tasks in the crew's Telegram
group; what the crew sends back lands on the page. One Python process on
cyberproxy, standard library only, writes `plot-data/<plot>.json` straight
into the nginx docroot — the app fetches it, no rebuild.

## flow

| step | who | what |
|---|---|---|
| ORDER SURVEY · ORDER PHOTO · DRONE VIDEO | Holder, on the page | opens `t.me/<bot>?start=survey_<plot>` |
| TASK | bot | posts the task to the hard force group, answers the Holder |
| reply to the TASK | crew | photos, video, the survey sheet; `m2 612.4` for the figure |
| publish | bot | pins every file to IPFS (Pinata), writes `plot-data/<plot>.json`, tells the group and the Holder |

## install on cyberproxy

```sh
sudo mkdir -p /opt/hardforce && sudo cp bot/hardforce.py /opt/hardforce/
sudo cp bot/hardforce.service /etc/systemd/system/
sudo install -m 600 /dev/null /etc/hardforce.env   # fill the five lines below
sudo systemctl enable --now hardforce
journalctl -u hardforce -f
```

`/etc/hardforce.env`:

```
BOT_TOKEN=...
HARDFORCE_CHAT_ID=-100...
PINATA_JWT=...
DOCROOT=/var/www/html/cyberia.my
SITE=https://cyberia.my
```

The group id: add the bot to the group, send `/id` there, copy the number.
The service runs as `www-data` so it can write under the docroot.

## switch the site to the bot

`src/orders.rs` — set `HARDFORCE_BOT` to the bot's username and deploy;
until then the ORDER buttons open a direct message to support with the
order text filled in.
