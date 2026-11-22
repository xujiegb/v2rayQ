#!/usr/bin/env python3
"""Check language coverage and placeholder parity for both applications."""
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
required = {"en", "zh-CN", "zh-TW", "fr"}
for scope in ["client", "server"]:
    folder = root / "locales" / scope
    catalogs = {file.stem: json.loads(file.read_text(encoding="utf-8")) for file in folder.glob("*.json")}
    assert required <= catalogs.keys(), (scope, "Missing language")
    english = catalogs["en"]
    for code, catalog in catalogs.items():
        assert set(catalog) == set(english), (scope, code, sorted(set(english) ^ set(catalog)))
        for key, value in catalog.items():
            assert isinstance(value, str) and value, (scope, code, key)
            assert sorted(re.findall(r"\{[a-zA-Z0-9_]+\}", value)) == sorted(re.findall(r"\{[a-zA-Z0-9_]+\}", english[key])), (scope, code, key)
    print(f"{scope}: {len(english) - 1} messages, {len(catalogs)} languages")
