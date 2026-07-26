# Desktop QA checklist (production)

Run this on **Windows** against a Release build or `npm run tauri:dev`.  
Record date, build (tag/commit), and pass/fail.

| Field | Value |
| --- | --- |
| Date | |
| Tester | |
| Build | tag / commit |
| Pack used | |

## Prerequisites

- [ ] WebView2 available
- [ ] CLI `asset-mapper.exe` on PATH or local path
- [ ] Modular pack with ≥2 `.glb` assets (or `fixtures/phase2/modular_pack` after `npm run fixture:phase2`)

## Checklist

| # | Step | Pass? | Notes |
| --- | --- | --- | --- |
| 1 | Launch editor (installer or `tauri:dev`) | | |
| 2 | **Init** pack: name + **license** + **author** required | | |
| 3 | Pack Completeness banner **not** shown after valid init | | |
| 4 | Asset list shows indexed files | | |
| 5 | **Measure bounds** clears bounds placeholder warnings | | |
| 6 | Select glb; viewport previews mesh | | |
| 7 | Add connector; move with gizmo; edit numeric XYZ | | |
| 8 | Set role, mating axis, class; add class + compatibility rule | | |
| 9 | Edit tags from vocabulary | | |
| 10 | **Validate** — only warnings allowed (no errors) | | |
| 11 | **Save** sidecar under `.asset-mapper/` | | |
| 12 | **Export bundle**; open JSON (no raw quaternions) | | |
| 13 | CLI: `validate`, `bundle`, `resolve` on same pack | | |
| 14 | Dirty edit → Reload discards; Discard works | | |
| 15 | Change mesh → Index shows drift → Accept drift | | |

## CLI smoke (same pack)

```powershell
asset-mapper validate .\my-pack
asset-mapper bundle .\my-pack > bundle.json
asset-mapper export-engine .\my-pack --target unity
# optional: asset-mapper resolve .\my-pack .\plan.json
```

## Sign-off

- [ ] All steps passed **or** failures filed as GitHub issues before release claim  
- Tester signature: _______________
