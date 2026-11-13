# Instructions for a coding agent in a Builder project

`AGENTS.md` holds the same rules for agents that read that file instead.

## Where code goes

1. Read `BRIEF.md`, `BUILDER-SPEC.md` and `GLOSSARY.md` before changing code.
2. The core (`crates/core`) owns every application decision and performs no input or output. Scripts and tools that need a rule read the core's output; they do not recompute it.
3. The shim (`crates/shim`) translates values and decides no application behaviour.
4. The page (`web/`) draws what the core returns and does not reproduce a core rule. The core describes the screen (`ui`) and takes settings, screens and flags through one function (`act`). Presentation numbers go in `data/*.json`; shaders go in `data/shaders/` and take rule values as uniforms.

## How to work

5. Run `cargo test` after each meaningful core change.
6. For each new behaviour test, add a row to `tests/breaks.json` and run `python3 scripts/break_tests.py`: the test must fail with its behaviour broken. Work expected values out by hand; never compare a value with the function under test. When a test needs a workaround to pass, the workaround is the finding: record it.
7. Keep `SECOND-ORDER.md` while you work: one row per assumption, surprise, divergence from the plan, or item the person must do, written when you notice it. It is how the person sees what you decided alone. A row that produced a rule names where the rule now lives; a row with status *person* is named in `HANDOFF.md`.
8. A scripted edit that replaces text asserts that it matched. A count in a document comes from `./scripts/count-tests.sh`.
9. Do not deploy unless `BRIEF.md`'s Autonomy paragraph says you may. Deploying is otherwise the person's decision (stage 8).
10. Before handoff, run `./scripts/check.sh` and record the result in `HANDOFF.md`.

## Words

11. Use each term in the sense `GLOSSARY.md` gives it, and never a word from its `Not:` line. A new term gets an entry before it appears in a string.
12. Reader-visible text follows `TONE.md`. A string that looks wrong is reported in `SECOND-ORDER.md`, not repaired. A string you write yourself is listed at the foot of `SECOND-ORDER.md` for the person to read. A list you generate that a reader will see (names, labels, word lists) is a worklist row for the person.

## The design journal

13. Keep the design journal unless `BRIEF.md`'s Journal section says `none`. Start it with `python3 tools/journal/new_journal.py`; it lives beside the project (by default `../<this folder>-journal/`) as its own repository and is never pushed. `tools/journal/ENTRIES.md` says what each kind of entry holds.
14. Before writing `PLAN.md`, open every project `BRIEF.md` names under References (its digest, when it has one), then write the journal's opening entry: read Builder (through the start pack, rule 19), the brief and those references back to the person, with numbered rules and what holds each, the question prefix, where you stop, and your questions. When the person answers, each rule that applies to every later session goes under "Rules agreed for this project" below, and each rule the core can enforce becomes a test.
15. Every decision put to the person, visible change, divergence from the plan and deploy gets a journal entry when it happens, citing its rows (T4, R2.3, XX-Q5). Never remove an entry. `scripts/journal_check.py` checks the form; commit in the journal's repository after each change, and republish it if it has an address.
16. Put questions to the person as answer chips in the journal (`decisions.json`, drawn by `tools/journal/decisions.py`), each with a default, never as a list to answer in chat. When the person says the answers are in the journal, read them there and record them with `decisions.py --answers`.

## Cost

17. Work as one agent unless `BRIEF.md`'s Autonomy paragraph names subagents. A question that offers them gives a token estimate. A subagent's brief says it runs no browser check; the coordinator merges several parts and runs one.
18. While iterating, run `./scripts/quick-check.sh`, which runs only the tests the changed files touch. `./scripts/check.sh` stays the check before a handoff, a deploy or a publish.

## Context

Each model call re-reads the whole context, so what is read once is paid for on every later call. These rules change how Builder and your own work are found and carried, not what Builder asks (`docs/context.md`).

19. Read Builder through its cards. Start with `python3 tools/ask.py --start`, and ask `python3 tools/ask.py "question"` (`--code` for code) before opening a Builder document or source file. Open a file only where a card points, and only the lines it names. Read `CLAUDE.md`, `BRIEF.md`, `BUILDER-SPEC.md` and `GLOSSARY.md` in full as rule 1 says. Cite cards as `card:<id>`.
20. Read an earlier project through its digest (`tools/digest.py`), not its transcripts. When a reference has none, write one, fill in its "What to copy", and read the project only for what the digest names. When "What to copy" names code for something you are building, open those lines and port them with their constants and tests; do not rewrite them from the digest's summary. Writing it afresh instead needs a `SECOND-ORDER.md` row saying why.
21. Keep `STATE.md`: rewrite it at each milestone (a stage boundary, a finished feature, a deploy), after each request you record in `TRIAGE.md`, and before a handoff, naming the latest commit. When the context passes about 200k tokens, or two thirds of a compaction cap when one is set (the after-tool hook says so; `python3 tools/tokens.py --print` shows it), rewrite it, commit, and ask the person to type `/compact`: you cannot compact yourself. After a compaction or in a new session, read `STATE.md` and the cards the after-compaction hook gives for its next step, and ask for more; do not re-read files it already summarises. `scripts/context_check.py` fails when `TRIAGE.md` changed after `STATE.md` was last rewritten.
22. `./scripts/check.sh` and `./scripts/quick-check.sh` keep their full output in `notes/runs/` and print only a summary; run any other long command through `./scripts/quiet.sh`. Read a log only for what the summary leaves out (`BUILDER_LOUD=1` prints everything). A failed approach becomes a `SECOND-ORDER.md` row saying how it failed.
23. Before a handoff, run `python3 tools/tokens.py` and quote its row in `HANDOFF.md`.
24. Ask your own work before re-reading it. `python3 tools/ask.py --mine "question"` searches this project's journal entries, notebook and triage rows, `STATE.md`, `PLAN.md`, commits, code, run logs and the tool results its sessions have seen; `--target` searches a codebase this project explains. Before reading a long file again, ask, then open only the lines a card names; the read hook lists a long file's parts when you read it whole. In `STATE.md`, cite work cards (`card:mine.<id>`) rather than restating them.
25. Change a file of more than about 200 lines with edits, not by writing it again whole: a whole rewrite sends all of it again, and it stays in the context. When a rewrite is really needed, add a `SECOND-ORDER.md` row saying why.

## Rules agreed for this project

None yet. When the person answers the opening entry's questions, add each rule that applies to every later session here, with its journal ID.

## Stopping

Write the journal's opening entry, then `PLAN.md` from the brief, and stop until the person has read it, unless `BRIEF.md`'s Autonomy paragraph approves the plan in advance. When a rule in `BUILDER-SPEC.md` seems to block the work, stop and say so in a worklist row; do not loosen a check to make it pass. When a request is refused on safety grounds, leave the feature out and say so; do not reword the request to get past the refusal.
