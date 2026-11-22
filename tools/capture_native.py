#!/usr/bin/env python3
"""Capture the real egui executable on an isolated X server (preview dependencies only)."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

root = Path(__file__).resolve().parents[1]
bin_dir = Path(os.environ.get("CARGO_TARGET_DIR", root / "target")) / "debug"
out = Path(os.environ.get("V2RAYQ_PREVIEW_OUT", root / "dist/ui-previews"))
out.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="v2rayq-native-") as directory:
    tcp = os.environ.get("V2RAYQ_X_TCP") == "1"
    display = "127.0.0.1:100" if tcp else ":100"
    transport = ["-nolisten", "unix", "-nolisten", "local", "-listen", "tcp"] if tcp else ["-nolisten", "tcp"]
    env = {**os.environ, "DISPLAY": display, "V2RAYQ_CLIENT_DB": str(Path(directory) / "client.redb"), "XDG_RUNTIME_DIR": directory}
    xserver = subprocess.Popen([os.environ.get("V2RAYQ_XVFB", "Xvfb"), ":100", "-screen", "0", "1200x920x24", *transport, "-ac"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for attempt in range(60):
            if subprocess.run(["xdpyinfo"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
                break
            time.sleep(.1)
        else:
            raise RuntimeError("The preview X server could not start")
        def rpc(command):
            reply = json.loads(subprocess.check_output([str(bin_dir / "v2rayq-cli"), "--rpc"], input=json.dumps(command), text=True, env=env))
            if "error" in reply:
                raise RuntimeError(reply["error"])
            return reply["result"]
        rpc({"action": "add_link", "link": "trojan://preview-secret@edge.example.net:443#Tokyo%20%2F%20Premium"})
        rpc({"action": "save_language", "language": "zh-CN"})
        for theme in ["light", "dark"]:
            rpc({"action": "save_theme", "theme": theme})
            app = subprocess.Popen([str(bin_dir / "v2rayq-client")], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            try:
                window = None
                for attempt in range(80):
                    if app.poll() is not None:
                        raise RuntimeError(app.stderr.read().decode())
                    tree = subprocess.check_output(["xwininfo", "-root", "-tree"], env=env, text=True)
                    window = re.search(r'(0x[0-9a-f]+) "v2rayQ"', tree)
                    if window:
                        break
                    time.sleep(.1)
                if not window:
                    raise RuntimeError("The egui window did not appear")
                time.sleep(2)
                subprocess.run(["import", "-window", window[1], str(out / f"egui-client-{theme}.png")], env=env, check=True)
            finally:
                app.terminate()
                app.wait(timeout=10)
    finally:
        xserver.terminate()
        xserver.wait(timeout=10)
print("Captured the actual egui client in light and dark appearance")
