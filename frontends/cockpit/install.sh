#!/bin/sh
set -eu
dest="${1:-/usr/share/cockpit/v2rayq}"
mkdir -p "$dest/assets"
install -m 0644 frontends/cockpit/manifest.json "$dest/manifest.json"
install -m 0644 frontends/web/client.html "$dest/client.html"
install -m 0644 frontends/shared/panel.css "$dest/assets/panel.css"
install -m 0644 frontends/shared/panel.js "$dest/assets/panel.js"
printf 'Installed v2rayQ Cockpit client assets to %s\n' "$dest"
