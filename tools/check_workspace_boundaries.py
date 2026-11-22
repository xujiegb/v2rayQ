#!/usr/bin/env python3
"""Reject accidental UI/server coupling or the former large SQL runtime."""
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def manifest(relative):
    with (ROOT / relative / "Cargo.toml").open("rb") as file:
        return tomllib.load(file)


def dependencies(relative):
    data = manifest(relative)
    result = set(data.get("dependencies", {}))
    for target in data.get("target", {}).values():
        result.update(target.get("dependencies", {}))
    return result


def require(condition, message):
    if not condition:
        raise AssertionError(message)


root = manifest(".")
workspace = set(root["workspace"]["members"])
required = {
    "apps/client", "apps/server", "apps/web-client",
    "crates/contracts", "crates/http-preview", "crates/storage",
}
require(required <= workspace, "Missing independent app or shared crate")
require("turso" not in dependencies("."), "Turso must not return")
require("tokio" not in dependencies("."), "Old database async runtime must not return")
require("redb" in dependencies("crates/storage"), "redb must remain the storage engine")
require("v2rayq-storage" in dependencies("."), "Shared client domain must use redb")
require("eframe" in dependencies("apps/client"), "Native UI must stay native egui")

for app in ["apps/server", "apps/web-client"]:
    deps = dependencies(app)
    require("eframe" not in deps and "egui" not in deps and "wgpu" not in deps,
            f"{app}: native graphics dependencies forbidden")
    require("v2rayQ" not in deps and "v2rayq-client" not in deps,
            f"{app}: client domain must not leak into HTTP host")

require("v2rayq-http-preview" in dependencies("apps/server"), "Server HTTP host missing")
require("v2rayq-http-preview" in dependencies("apps/web-client"), "Web HTTP host missing")

client_sources = ROOT / "apps/client/src"
for rust in client_sources.rglob("*.rs"):
    source = rust.read_text(encoding="utf-8")
    require("WebView" not in source and "webview" not in source,
            f"{rust}: no browser shells inside native UI")
    require("unsafe {" not in source and "transmute(" not in source,
            f"{rust}: prohibited Rust construct")
print("Workspace dependency boundaries: OK")
