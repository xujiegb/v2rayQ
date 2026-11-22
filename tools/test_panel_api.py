#!/usr/bin/env python3
"""Exercise the real HTTP and CLI entry points against isolated profile stores."""
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
    with launch("v2rayq-web-client", env) as (port, token):
        assert command(port, "bad-token", {"action": "profile"})[0] == 403
        assert command(port, token, {"action": "profile"}, Origin="https://example.org")[0] == 403
        assert command(port, token, {"action": "profile"}, Host="example.org")[0] == 403
        status, payload = command(port, token, {"action": "add_link", "link": SHARE})
        assert status == 200 and payload["result"]["profile"]["nodes"][0]["name"] == "Europe"
        assert "secret" not in json.dumps(payload)
        fragmented_request(port, token)
        stored = subprocess.check_output([BINARIES / "v2rayq-cli", "nodes"], env=env, text=True)
        assert "Europe" in stored and "secret" not in stored
        luci_cmd = json.dumps({"command": '{"action":"profile"}'})
        luci = subprocess.check_output([BINARIES / "v2rayq-cli", "--luci-rpc"], input=luci_cmd, env=env, text=True)
        assert json.loads(luci)["result"]["profile"]["nodes"]
        node = payload["result"]["profile"]["nodes"][0]
        assert not command(port, token, {"action": "remove_node", "id": node["id"]})[1]["result"]["profile"]["nodes"]
    with launch("v2rayq-server", env) as (port, token):
        request = {"action": "save", "section": "inbounds", "revision": 0, "value": [{"tag": "proxy"}]}
        assert command(port, token, request)[1]["result"]["revision"] == 1
        assert "error" in command(port, token, request)[1]
        result = command(port, token, {"action": "load", "section": "inbounds"})[1]["result"]
        assert result == {"revision": 1, "value": [{"tag": "proxy"}]}
        assert command(port, token, {"action": "overview"})[1]["result"]["counts"]["inbounds"] == 1

print("Panel authorization, fragmented HTTP bodies, shared client storage and server drafts: OK")
