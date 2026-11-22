#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
base="${1:-/usr/share/cockpit}"
mode="${2:-all}"
client_assets="frontends/luci/htdocs/luci-static/resources/v2rayq"

install_panel() {
    panel="$1"
    dest="$base/v2rayq-$panel"
    mkdir -p "$dest/assets"
    install -m 0644 "frontends/cockpit/$panel.json" "$dest/manifest.json"
    sed 's/content="local"/content="cockpit"/' "frontends/web/$panel.html" > "$dest/$panel.html"
    install -m 0644 frontends/shared/transport.mjs "$dest/assets/"
    if [ "$panel" = client ]; then
        install -m 0644 "$client_assets/client.css" "$client_assets/client-ui.js" frontends/web/client-entry.mjs "$dest/assets/"
    else
        install -m 0644 frontends/web/server.css frontends/web/server.mjs frontends/web/server-fields.mjs "$dest/assets/"
    fi
    printf 'Installed v2rayQ Cockpit %s panel to %s\n' "$panel" "$dest"
}

case "$mode" in
    client|server) install_panel "$mode" ;;
    all) install_panel client; install_panel server ;;
    *) printf 'Use client, server, or all\n' >&2; exit 2 ;;
esac
