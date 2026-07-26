#!/usr/bin/env python3
"""Inject license/provenance/vocabulary fields into PackRecord literals."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

INSERT_QUALIFIED = """\
        license_summary: "MIT OR Apache-2.0".to_owned(),
        provenance: asset_mapper_core::PackProvenance {
            notes: Some("test fixture".to_owned()),
            ..asset_mapper_core::PackProvenance::default()
        },
        vocabulary: asset_mapper_core::ControlledVocabulary::default(),
"""

INSERT_LOCAL = """\
        license_summary: "MIT OR Apache-2.0".to_owned(),
        provenance: PackProvenance {
            notes: Some("test fixture".to_owned()),
            ..PackProvenance::default()
        },
        vocabulary: ControlledVocabulary::default(),
"""


def patch_rs(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if "PackRecord {" not in text:
        return False
    if "license_summary:" in text:
        return False
    header = text[: min(len(text), 2500)]
    insert = (
        INSERT_LOCAL
        if ("PackProvenance" in header or "ControlledVocabulary" in header)
        else INSERT_QUALIFIED
    )
    lines = text.splitlines(keepends=True)
    out = []
    changed = False
    i = 0
    while i < len(lines):
        line = lines[i]
        out.append(line)
        if "default_units:" in line and i + 1 < len(lines):
            nxt = lines[i + 1]
            if "connector_classes:" in nxt and "license_summary" not in "".join(
                lines[max(0, i - 5) : i + 3]
            ):
                # inject before connector_classes line
                indent_match = len(nxt) - len(nxt.lstrip(" "))
                # insert already has indent of 8 spaces
                out.append(insert)
                changed = True
        i += 1
    if changed:
        path.write_text("".join(out), encoding="utf-8")
    return changed


def patch_fixture(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if '"license_summary"' in text:
        # still bump schema if needed
        pass
    import json

    data = json.loads(text)
    data["schema_version"] = 2
    data.setdefault(
        "license_summary",
        "MIT OR Apache-2.0 — test fixture",
    )
    data.setdefault(
        "provenance",
        {
            "notes": "Phase fixture pack",
            "source": "fixtures",
        },
    )
    data.setdefault(
        "vocabulary",
        {
            "semantic_tags": [
                "wall",
                "floor",
                "corner",
                "door",
                "window",
                "walkable",
                "cover",
                "decorative",
                "hazard",
                "lootable",
                "entry",
                "exit",
                "corridor",
                "roof",
                "prop",
            ],
            "affordances": [
                "block_movement",
                "provide_cover",
                "openable",
                "climbable",
                "sittable",
                "interactable",
                "light_source",
            ],
            "placement_constraints": [
                "grounded",
                "wall_mounted",
                "ceiling_mounted",
                "indoor_only",
                "outdoor_only",
                "requires_floor",
                "requires_wall",
            ],
            "allow_namespaced_extensions": True,
        },
    )
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return True


def main() -> None:
    for path in ROOT.rglob("*.rs"):
        if "target" in path.parts:
            continue
        if patch_rs(path):
            print("patched", path.relative_to(ROOT))
    for path in (ROOT / "fixtures").rglob("*.assetmap.json"):
        patch_fixture(path)
        print("fixture", path.relative_to(ROOT))


if __name__ == "__main__":
    main()
