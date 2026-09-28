# Build + rsync fleets&flats console to cyberproxy for cyberia.my
#
#   nu scripts/deploy.nu
#   nu scripts/deploy.nu --skip-build

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

  print "→ rsync dist/ → cyberproxy:/var/www/html/cyberia.my/"
  ^ssh cyberproxy "mkdir -p /var/www/html/cyberia.my"
  # the build is flat files only; every top-level directory in the docroot
  # (cx, hackathon, music, fonts — synced from the pages repo) is left alone
  ^rsync -az --delete --exclude "/*/" $"($root)/dist/" "cyberproxy:/var/www/html/cyberia.my/"

  print ""
  print "✓ deployed → https://cyberia.my/  (after DNS + TLS)"
  print "  nginx: scripts/nginx-cyberia.my.conf"
}
