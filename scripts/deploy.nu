# Build + rsync the map (the globe and the land console) to the front for cyberia.my
#
#   nu scripts/deploy.nu
#   nu scripts/deploy.nu --skip-build
#
# The host and the ssh command come from the environment, so no machine's path
# lives here:  CYBERIA_DEPLOY_HOST (default front-primary),
#              CYBERIA_DEPLOY_SSH  (default ssh; e.g. "ssh -F <ssh_config>")

def main [--skip-build] {
  let root = (
    if ($"($env.PWD)/src/app.rs" | path exists) { $env.PWD }
    else { error make {msg: "run from cyberia-my root"} }
  )
  cd $root

  if not $skip_build {
    print "→ trunk build --release"
    # rustup cargo + homebrew trunk
    let path = $"($env.HOME)/.cargo/bin:/opt/homebrew/bin:($env.PATH)"
    with-env { PATH: $path } { ^trunk build --release }
  }

  if not ($"($root)/dist/index.html" | path exists) {
    error make {msg: "dist/index.html missing — build first"}
  }

  # one crawlable page per flat, plus sitemap.xml and robots.txt
  print "→ render plot pages"
  ^python3 $"($root)/scripts/render-plot-pages.py"

  let host = ($env.CYBERIA_DEPLOY_HOST? | default "front-primary")
  let ssh = ($env.CYBERIA_DEPLOY_SSH? | default "ssh")
  let dest = $"($host):/var/www/html/cyberia.my/"
  print $"→ rsync dist/ → ($dest)"
  ^sh -c $"($ssh) ($host) 'mkdir -p /var/www/html/cyberia.my/plot /var/www/html/cyberia.my/covers /var/www/html/cyberia.my/globe'"
  # the build is flat files at the root; every top-level directory in the
  # docroot (cx, hackathon, music, fonts — synced from the pages repo) is
  # left alone, except plot/, which this build owns
  # the directories go first and the root (index.html, wasm) last, so a page
  # never asks for a file that is not there yet
  ^rsync -az --delete -e $ssh $"($root)/dist/plot/" $"($dest)plot/"
  # the globe's line layers and the states' names
  ^rsync -az --delete -e $ssh $"($root)/dist/globe/" $"($dest)globe/"
  # the documents' first pages, beside the library
  ^rsync -az --delete -e $ssh $"($root)/dist/covers/" $"($dest)covers/"
  # cyberia.kml lives in the docroot and in no build (it is not in git): it stays
  ^rsync -az --delete --exclude "/*/" --exclude "/cyberia.kml" -e $ssh $"($root)/dist/" $dest

  print ""
  print "✓ deployed → https://cyberia.my/  (after DNS + TLS)"
  print "  nginx: scripts/nginx-cyberia.my.conf"
}
