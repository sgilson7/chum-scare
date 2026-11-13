# Triage

Stage 7. One row per request after the person has used the build: a bug, a change, or a new idea, in the person's own words. Rows are numbered T1, T2 and so on, and each commit that answers one names it ("Fix the timer reset (T4)"). `C4 Verify against hand-computed cases`.

Compare at least one result with a case worked out by hand. **Severity** is *blocks use*, *wrong*, or *wanted*. **Cost** is the agent's estimate. **Disposition** is *done*, *open*, *declined* (with the reason), or *try* (built on a branch and published as a TRY commit that says how to undo it).

| # | Request, in the person's words | Expected | What happened | Severity | Cost | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| T1 | CH-Q8: B (one more box of six hidden in a store off the route) | A spare box in Video Vault, taken with E | Built: `PropKind::Spare`, `World::modes` | wanted | small | done |
| T2 | CH-Q12 note: "cultists should be shootable as well with your gun" | One hit drops a cultist; the shot alerts the rest | Built: `World::fire`, test `a_shot_cultist_falls` | wanted | small | done |
| T3 | "there should be a volume slider for the game" | A volume slider before the doors and in the pause menu, kept with the settings | Built: `Settings::volume` (percent, 0 to 100), `act("volume")`; the page scales its master gain; older kept settings load at 100 | wanted | small | done |
| T4 | "the signs all turn to face you as you move, they should remain fixed in the directionthey are supposed to face. maybe when you are running back during the run phase the signs can start to turn towards you like they do now" | Each sign hangs facing one way, seen in perspective and not from behind; in the chase every sign turns to look at you | Built: `Sign::yaw`, `World::sign_yaws` turned 30° a second in the chase, `view::sign_matrix` (a CSS `matrix3d` from the core) | wrong | medium | done |
