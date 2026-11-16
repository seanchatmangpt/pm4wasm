# pm4wasm: land or delete remote branch `ws4/ggen-sync-canonical-20260827-pm4wasm`

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: remote branch `ws4/ggen-sync-canonical-20260827-pm4wasm` — not merged into `main`, no open PR
- Evidence: `git branch -r --no-merged origin/main` lists it; absent from `gh pr list` heads

## Work to complete
- Decide: open a PR (`gh pr create -R seanchatmangpt/pm4wasm --head ws4/ggen-sync-canonical-20260827-pm4wasm`) or delete (`git push origin --delete ws4/ggen-sync-canonical-20260827-pm4wasm`).
- If superseded, delete; otherwise land through review.

## Acceptance
- After `git fetch --prune`, `git branch -r --no-merged origin/main` no longer lists `ws4/ggen-sync-canonical-20260827-pm4wasm`.

## History
- 2026-09-19 | OPEN | survey found PR-less unmerged branch | ws4/ggen-sync-canonical-20260827-pm4wasm | decision pending
