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
| 7 | **Analyze** proposes connectors (mesh sockets preferred); status shows mesh vs AABB counts | | |
| 8 | Green markers sit on meaningful faces/openings (not only wrong box centers on irregular pieces) | | |
| 9 | Compatibility rules include useful cross-class pairs when pack has doors/walls | | |
| 10 | **Pack assembly → Auto layout pack** shows ≥3 pieces when kit has ≥3 mateable assets | | |
| 11 | **Two-piece mate** still works for a chosen connector pair | | |
| 12 | Add/move connector with gizmo; edit numeric XYZ | | |
| 13 | Set role, mating axis, class; edit rules | | |
| 14 | Edit tags from vocabulary | | |
| 15 | **Validate** — only warnings allowed (no errors) | | |
| 16 | **Save** sidecar under `.asset-mapper/` | | |
| 17 | **Export bundle**; open JSON (no raw quaternions) | | |
| 18 | CLI: `analyze`, `propose-assembly`, `validate`, `bundle`, `resolve` | | |
| 19 | Dirty edit → Reload discards; Discard works | | |
| 20 | Change mesh → Index shows drift → Accept drift | | |

## CLI smoke (same pack)

```powershell
asset-mapper analyze .\my-pack --replace
asset-mapper propose-assembly .\my-pack --max-pieces 8 -o plan.json
asset-mapper resolve .\my-pack .\plan.json
asset-mapper validate .\my-pack
asset-mapper bundle .\my-pack > bundle.json
asset-mapper export-engine .\my-pack --target unity
```

## Sign-off

- [ ] All steps passed **or** failures filed as GitHub issues before release claim  
- Tester signature: _______________
