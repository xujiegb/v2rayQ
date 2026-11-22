#!/bin/sh
set -eu
dest="/usr/share/cockpit/v2rayq"
if [ "$#" -gt 0 ]; then dest="$1"; fi
mkdir -p "$dest/assets"
install -m 0644 frontends/cockpit/manifest.json "$dest/manifest.json"
install -m 0644 frontends/web/client.html "$dest/client.html"
install -m 0644 frontends/web/server.html "$dest/server.html"
install -m 0644 frontends/shared/panel.css "$dest/assets/panel.css"
install -m 0644 frontends/shared/panel.js "$dest/assets/panel.js"
echo "Cockpit UI preview installed to $dest"
