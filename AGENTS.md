# Sequential agent instructions

You are implementing [Echo](../echo) for Linux from an ordered specification. Work like an
engineer taking over a clean shift.

Read `docs/PORTING_GUIDE.md` completely before implementing a feature.

## Start every session

1. Run `pwd`.
2. Read `AGENTS.md`, `progress.md`, and `feature-list.json`.
3. Read the last 10 git commits and inspect the worktree.
4. Run `scripts/bootstrap.sh`.
5. Run `scripts/check.sh`.
6. If the baseline is broken, repair it before selecting a new feature and
   explain the repair in `progress.md`.

## Select work

- Choose the first feature with `"passes": false` whose dependency IDs all
  pass.
- Work on one feature only.
- Do not add adjacent features, speculative abstractions, or cleanup.
- Do not change a feature's ID, description, dependencies, or acceptance steps.
- If the feature is too large for one clean session, leave it failing, document
  the exact state, and do not commit a knowingly broken midpoint.

## Implement and verify

- Keep GTK work on the main thread and slow work off it.
- Never log API keys, request authorization headers, transcript text, or raw
  audio.
- Add the smallest tests that prove the feature.
- Run the feature's acceptance steps.
- Run `scripts/check.sh`.
- Manual acceptance must be tested manually; code inspection is not evidence.
- If the required display server, hardware, or credential is unavailable, keep
  `"passes": false` and write precise follow-up steps.

## End every session

1. Leave the repository buildable and tests passing.
2. Change only the completed feature's `passes` field to `true`.
3. Append a dated entry to `progress.md` using its existing format.
4. Include commands run, evidence, known limitations, and the next eligible
   feature.
5. Commit with `FEATURE-ID: concise result`.
6. Confirm the worktree is clean.

Finishing one verified feature is success. Declaring the whole application done
before every feature passes is not.
