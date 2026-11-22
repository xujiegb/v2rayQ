#!/usr/bin/env python3
"""Keep shared client navigation aligned with the native app and server navigation."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
contracts = (root / "crates/contracts/src/lib.rs").read_text()
client = (root / "frontends/luci/htdocs/luci-static/resources/v2rayq/client-ui.js").read_text()
server = (root / "frontends/web/server.mjs").read_text()


def native(start, end):
    block = contracts.split(start, 1)[1].split(end, 1)[0]
    return re.findall(r'id: "([^"]+)",\s*title: "([^"]+)"', block)


def web(source, array):
    block = source.split(f"const {array} = [", 1)[1].split("];", 1)[0]
    return re.findall(r'\["([^"]+)", "([^"]+)",', block)


assert native("pub const CLIENT_NAV:", "pub const SERVER_NAV:") == web(client, "clientPages")
assert native("pub const SERVER_NAV:", "pub trait CoreBackend") == web(server, "serverPages")
assert "./client-ui.js" in (root / "frontends/web/client-entry.mjs").read_text()
assert "v2rayq/client-ui.js" in (root / "frontends/luci/htdocs/luci-static/resources/view/v2rayq/client.js").read_text()
assert "client-ui.js" in (root / "frontends/cockpit/install.sh").read_text()
print("Desktop / Web / Cockpit / LuCI navigation parity: OK")
