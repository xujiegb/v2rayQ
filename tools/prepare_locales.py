#!/usr/bin/env python3
"""Install one application's language resources from the central locale folder."""
import json
from pathlib import Path
import shutil
import sys

scope, destination = sys.argv[1:]
source = Path(__file__).resolve().parents[1] / "locales" / scope
destination = Path(destination)
destination.mkdir(parents=True, exist_ok=True)
manifest = []
for file in sorted(source.glob("*.json")):
    catalog = json.loads(file.read_text(encoding="utf-8"))
    manifest.append({"code": file.stem, "name": catalog["_name"]})
    shutil.copyfile(file, destination / file.name)
(destination / "index.json").write_text(json.dumps(manifest, ensure_ascii=False), encoding="utf-8")
