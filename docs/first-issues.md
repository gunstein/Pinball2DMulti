# First Issues

Last updated: 2026-02-11

Small, beginner-friendly tasks with clear boundaries.

## Rules for picking one

- Pick one issue from a single codebase (`client`, `server`, or `client_bevy`).
- Keep scope to one behavior change or one documentation change.
- Add/adjust tests where relevant.
- Keep commit message explicit about the user-visible effect.

## Candidate issues

1. Add missing inline docs for Bevy coord module
   File: `client_bevy/src/coord.rs`
   Estimate: 20-30 min
   Done when: all public structs/functions have concise rustdoc comments and examples for Y-axis convention.

2. Add README note about Bevy WASM WS override
   File: `client_bevy/README.md`
   Estimate: 10-15 min
   Done when: local trunk `:8080` -> server `:9001` behavior is documented in Run-in-browser section.

3. Add TS test for launcher stacked-ball scale edge
   File: `client/tests/gameConfig.test.ts`
   Estimate: 20-30 min
   Done when: test covers count=0 fallback and verifies no divide-by-zero or negative output.

4. Add server unit test for paused player not selected in reroute target sampling
   File: `server/src/deep_space.rs`
   Estimate: 30-45 min
   Done when: deterministic test proves reroute target excludes paused players.

5. Add docs snippet for “where to log first” when debugging network desync
   File: `docs/onboarding.md`
   Estimate: 15-25 min
   Done when: includes concrete file/function pointers for TS, Bevy, and server.

6. Add TS lint check command to top-level README scripts section
   File: `README.md`
   Estimate: 10 min
   Done when: lint command and expected use are documented consistently with existing script list.

7. Add Bevy test for escape direction guard (downward velocity should not escape)
   File: `client_bevy/src/game/ball.rs`
   Estimate: 30-45 min
   Done when: test asserts no despawn/send for downward velocity collision with escape sensor.

8. Add “test-only notes” section in docs for running focused test subsets
   File: `docs/onboarding.md`
   Estimate: 15-20 min
   Done when: shows 3-5 focused commands for fast local iteration.

9. Add server README-style command quicklist in docs index
   File: `docs/README.md`
   Estimate: 10-15 min
   Done when: docs index links directly to onboarding sections for run/test commands.

10. Add short architecture glossary cross-links in `docs/design.md`
    File: `docs/design.md`
    Estimate: 20-30 min
    Done when: terms like “TransferIn”, “Capture”, “Reroute”, “PPM” link to onboarding glossary.
