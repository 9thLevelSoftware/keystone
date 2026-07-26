#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
pat = re.compile(
    r"init_pack_folder\(([^,]+),\s*((?:\"[^\"]+\")|(?:[A-Za-z_][\w:]*))\.to_owned\(\)\)"
)

for path in root.rglob("*.rs"):
    if "target" in path.parts or path.name == "index.rs":
        continue
    text = path.read_text(encoding="utf-8")
    if "init_pack_folder(" not in text:
        continue
    new, n = pat.subn(
        r"init_pack_folder(\1, InitPackOptions::for_tests(\2))",
        text,
    )
    if n == 0:
        continue
    if "InitPackOptions" not in new:
        # Prefer adding to existing asset_mapper_io use
        if "use asset_mapper_io::" in new:
            new = new.replace(
                "use asset_mapper_io::{\n",
                "use asset_mapper_io::{\n    InitPackOptions,\n",
                1,
            )
            if "InitPackOptions" not in new.split("use asset_mapper_io")[1][:200]:
                # single-line import
                new = re.sub(
                    r"use asset_mapper_io::\{([^}]+)\}",
                    r"use asset_mapper_io::{InitPackOptions, \1}",
                    new,
                    count=1,
                )
        elif "use asset_mapper_editor::commands::" in new or "commands::" in new:
            pass
        # editor commands import from io
        if "io_init_pack_folder" in new and "InitPackOptions" not in new:
            new = new.replace(
                "init_pack_folder as io_init_pack_folder,",
                "InitPackOptions, init_pack_folder as io_init_pack_folder,",
            )
    path.write_text(new, encoding="utf-8")
    print(f"patched {n}: {path.relative_to(root)}")
