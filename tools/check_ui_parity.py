#!/usr/bin/env python3
"""Verify that native, web and LuCI navigation maintain identical labels."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
rust = (root / "crates/contracts/src/lib.rs").read_text()
js = (root / "frontends/shared/panel.js").read_text()
luci = (root / "frontends/luci/htdocs/luci-static/resources/view/v2rayq/client.js").read_text()

def titles(text, start, end):
    fragment = text.split(start, 1)[1].split(end, 1)[0]
    return re.findall(r'title: "([^"]+)"', fragment)

client = titles(rust, "pub const CLIENT_NAV:", "pub const SERVER_NAV:")
server = titles(rust, "pub const SERVER_NAV:", "pub trait CoreBackend")

js_client = re.findall(r'\["([^"]+)","[^"]+","[^"]+"\]', js.split("const clientPages = [", 1)[1].split("];", 1)[0])
js_server = re.findall(r'\["([^"]+)","[^"]+","[^"]+"\]', js.split("const serverPages = [", 1)[1].split("];", 1)[0])
luci_client = re.findall(r"\['([^']+)', '[^']+', '[^']+'\]", luci.split("var sections = [", 1)[1].split("];", 1)[0])

assert client == js_client, (client, js_client)
assert server == js_server, (server, js_server)
assert client == luci_client, (client, luci_client)
print("Client / server / Cockpit / LuCI navigation titles match")
