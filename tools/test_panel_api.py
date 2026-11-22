#!/usr/bin/env python3
"""Exercise the real HTTP and CLI entry points against isolated profile stores."""
from base64 import b64encode
from contextlib import contextmanager
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
from urllib.parse import urlsplit

BINARIES = Path(os.environ.get("CARGO_TARGET_DIR", "target")) / "debug"
SHARE = "trojan://secret@example.org:443?security=tls#Europe"


@contextmanager
def launch(binary, env):
    with subprocess.Popen([BINARIES / binary], env=env, stdout=subprocess.PIPE, text=True) as panel:
        try:
            startup = panel.stdout.readline()
            url = urlsplit(startup.rsplit(" ", 1)[-1].strip())
            assert url.port and url.fragment.startswith("access="), startup
            yield url.port, url.fragment.removeprefix("access=")
        finally:
            panel.terminate()


def command(port, token, action, **headers):
    request = http.client.HTTPConnection("127.0.0.1", port, timeout=8)
    try:
        request.request("POST", "/api/v1/command", json.dumps(action), {
            "Content-Type": "application/json", "Authorization": f"Bearer {token}", **headers,
        })
        response = request.getresponse()
        if response.status != 200:
            return response.status, response.read().decode()
        return response.status, json.load(response)
    finally:
        request.close()


def fragmented_request(port, token):
    body = b'{"action":"profile"}'
    with socket.create_connection(("127.0.0.1", port), timeout=5) as connection:
        connection.sendall(b"POST /api/v1/command HTTP/1.1\r\n")
        connection.sendall((f"Host: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n"
                            f"Content-Type: application/json\r\nContent-Length: {len(body)}\r\n\r\n").encode())
        connection.sendall(body[:7])
        connection.sendall(body[7:])
        response = http.client.HTTPResponse(connection)
        response.begin()
        assert response.status == 200
        assert json.load(response)["result"]["profile"]["nodes"]


with tempfile.TemporaryDirectory() as directory:
    env = {**os.environ, "V2RAYQ_CLIENT_DB": f"{directory}/client.redb",
           "V2RAYQ_SERVER_DB": f"{directory}/server.redb"}
    Path(directory, "client.json").write_text('{"settings":{"route_mode":"global","dns_server":"9.9.9.9"}}')
    with launch("v2rayq-web-client", env) as (port, token):
        old_settings = command(port, token, {"action": "profile"})[1]["result"]["profile"]["settings"]
        assert old_settings["route_mode"] == "global" and old_settings["route_rules"] == []
        assert command(port, "bad-token", {"action": "profile"})[0] == 403
        assert command(port, token, {"action": "profile"}, Origin="https://example.org")[0] == 403
        assert command(port, token, {"action": "profile"}, Host="example.org")[0] == 403
        status, payload = command(port, token, {"action": "add_link", "link": SHARE})
        assert status == 200 and payload["result"]["profile"]["nodes"][0]["name"] == "Europe"
        assert "secret" not in json.dumps(payload)
        fragmented_request(port, token)
        stored = subprocess.check_output([BINARIES / "v2rayq-cli", "nodes"], env=env, text=True)
        assert "Europe" in stored and "secret" not in stored
        terminal_link = SHARE.replace("Europe", "Terminal%1B%5B31m")
        assert command(port, token, {"action": "add_link", "link": terminal_link})[0] == 200
        stored = subprocess.check_output([BINARIES / "v2rayq-cli", "nodes"], env=env, text=True)
        assert "Terminal" in stored and "\x1b" not in stored
        luci_cmd = json.dumps({"command": '{"action":"profile"}'})
        luci = subprocess.check_output([BINARIES / "v2rayq-cli", "--luci-rpc"], input=luci_cmd, env=env, text=True)
        assert json.loads(luci)["result"]["profile"]["nodes"]
        node = payload["result"]["profile"]["nodes"][0]
        edit = {"action": "update_node", "id": node["id"], "name": "Updated", "host": "[2001:db8::7]", "port": "8443"}
        updated = command(port, token, edit)[1]["result"]["profile"]["nodes"][0]
        assert updated["id"] == node["id"] and updated["protocol"] == "trojan"
        assert updated["name"] == "Updated" and updated["host"] == "2001:db8::7" and updated["port"] == 8443
        assert "error" in command(port, token, {**edit, "port": "invalid"})[1]
        copy = command(port, token, {"action": "copy_node", "id": node["id"]})[1]["result"]["profile"]
        assert copy["nodes"][-1]["name"] == "Copy of Updated" and copy["selected_node"] != node["id"]
        remaining = command(port, token, {"action": "remove_node", "id": node["id"]})[1]["result"]["profile"]["nodes"]
        assert len(remaining) == 2 and all(item["id"] != node["id"] for item in remaining)

        add = {"action": "save_route_rule", "id": None}
        rule1 = {"kind": "domain_suffix", "value": ".Example.ORG.", "target": "direct"}
        rule2 = {"kind": "ip_cidr", "value": "2001:db8::/32", "target": "proxy"}
        saved = command(port, token, {**add, "rule": rule1})[1]["result"]["profile"]["settings"]
        first_id = saved["route_rules"][0]["id"]
        assert saved["route_rules"][0]["value"] == "example.org" and saved["dns_server"] == "9.9.9.9"
        saved = command(port, token, {**add, "rule": rule2})[1]["result"]["profile"]["settings"]
        second_id = saved["route_rules"][1]["id"]
        assert "error" in command(port, token, {**add, "rule": {**rule2, "value": "2001:db8::/129"}})[1]
        moved = command(port, token, {"action": "move_route_rule", "id": second_id, "direction": "up"})[1]
        assert [rule["id"] for rule in moved["result"]["profile"]["settings"]["route_rules"]] == [second_id, first_id]
        changed = {"kind": "port", "value": "443", "target": "block"}
        subprocess.run([BINARIES / "v2rayq-cli", "edit-rule", first_id], input=json.dumps(changed), env=env, text=True, check=True, capture_output=True)
        stored = subprocess.check_output([BINARIES / "v2rayq-cli", "routes"], env=env, text=True).splitlines()
        assert second_id in stored[0] and first_id in stored[1] and "443 → Block" in stored[1]
        removed = command(port, token, {"action": "remove_route_rule", "id": second_id})[1]
        assert removed["result"]["profile"]["settings"]["route_rules"] == [{"id": first_id, **changed}]
        share_list = "\ufeff# Local import\n" + (SHARE.replace("example.org", "next.example.net") + "\n") * 2 + "invalid"
        encoded = b64encode(share_list.encode()).decode()
        preview = command(port, token, {"action": "preview_import", "content": encoded})[1]["result"]
        assert preview == {"message": "Ready to import 1 node · 1 duplicate · 1 skipped"}
        cli_preview = subprocess.run([BINARIES / "v2rayq-cli", "preview-import"], input=encoded, env=env, text=True, check=True, capture_output=True)
        assert cli_preview.stdout.strip() == preview["message"]
        imported = command(port, token, {"action": "import_nodes", "content": encoded})[1]["result"]["profile"]
        assert len(imported["nodes"]) == 3 and imported["selected_node"] == copy["selected_node"]
        repeated = command(port, token, {"action": "import_nodes", "content": encoded})[1]["result"]
        assert len(repeated["profile"]["nodes"]) == 3 and "2 duplicates" in repeated["message"]

        new = command(port, token, {"action": "add_subscription", "name": "", "url": "https://127.0.0.1:1/list?token=private"})[1]["result"]["profile"]
        item = new["subscriptions"][0]
        assert item["name"] == "127.0.0.1" and item["node_count"] == 0 and "private" not in json.dumps(new)
        updated = command(port, token, {"action": "update_subscription", "id": item["id"], "name": "Replacement", "url": "https://127.0.0.1:2/next?token=private"})[1]["result"]["profile"]
        assert updated["subscriptions"][0]["name"] == "Replacement" and updated["subscriptions"][0]["last_error"] is None
    with launch("v2rayq-server", env) as (port, token):
        request = {"action": "save", "section": "inbounds", "revision": 0, "value": [{"tag": "proxy"}]}
        assert command(port, token, request)[1]["result"]["revision"] == 1
        assert "error" in command(port, token, request)[1]
        result = command(port, token, {"action": "load", "section": "inbounds"})[1]["result"]
        assert result == {"revision": 1, "value": [{"tag": "proxy"}]}
        assert command(port, token, {"action": "overview"})[1]["result"]["counts"]["inbounds"] == 1

        tls = [{"name": "Public TLS", "certificate_path": "/cert.pem", "key_path": "/key.pem"}]
        reality = [{"name": "Gateway", "private_key": "secret-key", "handshake_port": 443}]
        for section, values in [("tls", tls), ("reality", reality)]:
            saved = command(port, token, {"action": "save", "section": section, "revision": 0, "value": values})[1]
            assert saved["result"]["revision"] == 1
            loaded = command(port, token, {"action": "load", "section": section})[1]["result"]
            assert loaded["value"] == values
        names = command(port, token, {"action": "security_profiles"})[1]["result"]
        assert names == {"tls": ["Public TLS"], "reality": ["Gateway"]}

print("Panel authorization, fragmented HTTP bodies, shared client storage, batch imports, subscriptions and TLS/REALITY drafts: OK")

# Portable client data must survive different database IDs and repeated merges.
with tempfile.TemporaryDirectory() as directory:
    env = {**os.environ, "V2RAYQ_CLIENT_DB": f"{directory}/client.redb", "V2RAYQ_SERVER_DB": f"{directory}/server.redb"}
    with launch("v2rayq-web-client", env) as (port, token):
        command(port, token, {"action": "add_link", "link": SHARE})
        command(port, token, {"action": "save_routing", "route_mode": "global", "dns_server": "9.9.9.9"})
        exported = command(port, token, {"action": "export_configuration"})[1]["result"]["configuration"]
        decoded = json.loads(exported)
        assert decoded["format"] == "v2rayq-client" and decoded["version"] == 1
        assert decoded["profile"]["nodes"][0]["node"]["protocol"]["options"]["password"] == "secret"
        cli_json = subprocess.check_output([BINARIES / "v2rayq-cli", "export-config"], env=env, text=True)
        assert json.loads(cli_json) == decoded
        original_id = decoded["profile"]["nodes"][0]["node"]["id"]
        command(port, token, {"action": "remove_node", "id": original_id})
        command(port, token, {"action": "save_routing", "route_mode": "direct", "dns_server": "1.1.1.1"})
        request = {"action": "merge_configuration", "content": exported, "preview": True}
        assert command(port, token, request)[1]["result"]["merge"]["nodes"] == 1
        assert command(port, token, {"action": "profile"})[1]["result"]["profile"]["nodes"] == []
        result = command(port, token, {**request, "preview": False})[1]["result"]
        assert result["profile"]["nodes"][0]["id"] != original_id
        assert result["profile"]["settings"]["dns_server"] == "1.1.1.1"
        result = command(port, token, {**request, "preview": False, "use_preferences": True})[1]["result"]
        assert result["merge"]["nodes"] == 0 and result["merge"]["duplicates"] == 1
        assert result["profile"]["settings"]["dns_server"] == "9.9.9.9"
        malformed = {**decoded, "version": 99}
        assert "error" in command(port, token, {**request, "content": json.dumps(malformed)})[1]
        for code in ["en", "zh-CN", "zh-TW", "fr"]:
            profile = command(port, token, {"action": "save_language", "language": code})[1]["result"]["profile"]
            assert profile["settings"]["language"] == code
            help_text = subprocess.check_output([BINARIES / "v2rayq-cli", "--help"], env=env, text=True)
            assert "export-config" in help_text and "CLI help" not in help_text
    with launch("v2rayq-server", env) as (port, token):
        command(port, token, {"action": "save", "section": "settings", "revision": 0, "value": {"name": "Example"}})
        command(port, token, {"action": "save_language", "language": "zh-TW"})
        assert command(port, token, {"action": "language"})[1]["result"]["language"] == "zh-TW"
        settings = command(port, token, {"action": "load", "section": "settings"})[1]["result"]
        assert settings == {"revision": 1, "value": {"name": "Example"}}
print("Portable configuration preview, idempotent merge and independent language preferences: OK")


with tempfile.TemporaryDirectory() as directory:
    env = {**os.environ, "V2RAYQ_CLIENT_DB": f"{directory}/groups.redb"}
    with launch("v2rayq-web-client", env) as (port, token):
        result = command(port, token, {"action": "add_link", "link": SHARE})[1]["result"]
        node = result["profile"]["nodes"][0]["id"]
        result = command(port, token, {"action": "save_node_group", "id": None, "name": "Travel"})[1]["result"]
        group = result["profile"]["groups"][0]["id"]
        result = command(port, token, {"action": "assign_node_group", "node_id": node, "group_id": group})[1]["result"]
        assert result["profile"]["nodes"][0]["group_id"] == group
        assert result["profile"]["groups"][0]["node_count"] == 1
        assert "error" in command(port, token, {"action": "save_node_group", "id": None, "name": "travel"})[1]
        exported = command(port, token, {"action": "export_configuration"})[1]["result"]["configuration"]
        command(port, token, {"action": "remove_node_group", "id": group})
        profile = command(port, token, {"action": "profile"})[1]["result"]["profile"]
        assert len(profile["nodes"]) == 1 and profile["nodes"][0]["group_id"] is None
        assert profile["groups"] == []
        command(port, token, {"action": "merge_configuration", "content": exported})
        profile = command(port, token, {"action": "profile"})[1]["result"]["profile"]
        assert len(profile["groups"]) == 1 and profile["groups"][0]["node_count"] == 1
        assert profile["nodes"][0]["group_id"] == profile["groups"][0]["id"]
        command(port, token, {"action": "merge_configuration", "content": exported})
        assert len(command(port, token, {"action": "profile"})[1]["result"]["profile"]["groups"]) == 1
        runtime = command(port, token, {"action": "status"})[1]["result"]["runtime"]
        assert runtime["traffic"] == {"core_connected": False, "upload_bps": None, "download_bps": None}
print("Node group lifecycle, configuration membership and unavailable telemetry: OK")
