#!/bin/bash
# front.sh — the gate of cyberia.my's land market on front-primary (projects/listings.md).
# from the dev machine:  ssh front-primary 'bash -s key'              < gate/front.sh   the deploy key (prints its public half)
#                        ssh front-primary 'bash -s install [branch]' < gate/front.sh   the clones, the secret, the root, runit, nginx
#                        ssh front-primary 'bash -s update [branch]'  < gate/front.sh   deploy: move the served tree, restart
# one key: deploy_key — write on cyberia-to/cyberia-my-acts only (the log of acts). the code is the public repository,
# cloned over https; a compromised front can write acts the build refuses, never the code and never the root.
# layout (/srv/edge, the persistent volume): market/{repo,acts,secret,root,deploy_key}; the public listings are written
# to the docroot as /var/www/html/cyberia.my-market/listings.json and served at /api/acts/public by the gate itself.
# the root is the owner's address, the same one the apparatus pins: only it names admins.
set -euo pipefail
M=/srv/edge/market; ROOT=0xEA43e87071E7fC452BFc69948bB75Ac4e02eeEBd; BRANCH="${2:-main}"
as_cyber() { sudo -u cyber -H "$@"; }
case "${1:-}" in
key)
  mkdir -p $M; chown -R cyber:cyber $M
  [ -f $M/deploy_key ] || as_cyber ssh-keygen -q -t ed25519 -N "" -C "cyberia.my market gate on front-primary" -f $M/deploy_key
  echo "deploy_key: $(cat $M/deploy_key.pub)" ;;
install)
  [ -f $M/secret ] || { head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > $M/secret; chown cyber:cyber $M/secret; chmod 600 $M/secret; }
  printf '%s\n' "$ROOT" > $M/root; chown cyber:cyber $M/root
  [ -d $M/repo/.git ] || as_cyber git clone -q https://github.com/cyberia-to/cyberia-my.git $M/repo
  as_cyber git -C $M/repo fetch -q origin "$BRANCH" && as_cyber git -C $M/repo checkout -q --detach FETCH_HEAD
  SSH="ssh -i $M/deploy_key -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new"
  [ -d $M/acts/.git ] || as_cyber env GIT_SSH_COMMAND="$SSH" git clone -q git@github.com:cyberia-to/cyberia-my-acts.git $M/acts
  as_cyber git -C $M/acts config core.sshCommand "$SSH"
  mkdir -p /var/www/html/cyberia.my-market; chown cyber:cyber /var/www/html/cyberia.my-market
  mkdir -p /srv/edge/sv/market/log
  cat > /srv/edge/sv/market/run <<EOF
#!/bin/sh
exec 2>&1
exec chpst -u cyber env HOME=/home/cyber python3 $M/repo/gate/gate.py --repo $M/acts --root $M/root --secret $M/secret --publish /var/www/html/cyberia.my-market/listings.json --flats $M/repo/src/cyberia_map.json --bind 127.0.0.1 --port 8770
EOF
  printf '#!/bin/sh\nmkdir -p /var/log/market\nexec svlogd -tt /var/log/market\n' > /srv/edge/sv/market/log/run
  chmod +x /srv/edge/sv/market/run /srv/edge/sv/market/log/run
  [ -e /etc/service/market ] || ln -s /srv/edge/sv/market /etc/service/market
  conf=/etc/nginx/sites-enabled/cyberia.my
  if ! grep -q "location /api/acts/" $conf; then
    cp $conf /srv/edge/backup/cyberia.my.nginx.$(date -u +%Y%m%dT%H%M%SZ)
    # the gate sits before the site's catch-all; everything else on the site is untouched
    sed -i '0,/^    location \/ {/s##    # the land market'"'"'s gate: signed acts in, commits out (cyberia-my gate/gate.py, projects/listings.md)\n    location /api/acts/ { client_max_body_size 16k; proxy_pass http://127.0.0.1:8770; proxy_set_header Host $host; proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for; proxy_read_timeout 60s; }\n    location / {#' $conf
    nginx -t && nginx -s reload
  fi
  sv restart market 2>/dev/null || true; sleep 2; sv status market ;;
update)
  as_cyber git -C $M/repo fetch -q origin "$BRANCH" && as_cyber git -C $M/repo checkout -q --detach FETCH_HEAD && sv restart market && sleep 1 && sv status market ;;
*) sed -n 2,10p "$0" 2>/dev/null || true; exit 2 ;;
esac
