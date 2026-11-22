#!/usr/bin/env python3
"""Format changed Rust sources without reformatting unrelated legacy modules."""
import os
import subprocess

base = os.environ.get("V2RAYQ_BASE_REF", "HEAD^")
files = subprocess.check_output(
    ["git", "diff", "--name-only", "--diff-filter=AM", f"{base}...HEAD"], text=True
).splitlines()
files = [path for path in files if path.endswith(".rs")]
if files:
    subprocess.run(["rustfmt", "--edition", "2024", "--config", "skip_children=true", *files], check=True)
