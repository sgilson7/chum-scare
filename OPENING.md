Paste this, with the slots filled, then your description of the game.

Builder is already in this folder as builder/ (do not clone it). Use Builder
to constrain and direct your work as the game's co-designer and developer.
The goal is a page on GitHub Pages that anyone can open in a browser, so put
running well in the browser first.

This is a first-person horror game built on Chumontillado's engine, tuned
for slower, more methodical play. Earlier projects built with Builder, to
read for what works: Chumontillado (digest refs/chumontillado.md) and
Chumworld Arena, whose engine Chumontillado was built on (digest
refs/chumworld-arena.md). List them under References in BRIEF.md.

Chumontillado's source is in engine/chumontillado@c48d8dd/, a read-only
copy at that commit. Copy modules from it into your project as the digest
says (copy at a named commit, then record every number you change as a
notebook row with its old value); never edit engine/. Its movement is tuned
for speed: bunny hops, surfing, slides, wall jumps and slams. Decide with me,
in the journal, which of those this game keeps, and give each feel number in
movement::TUNING a horror value with the reason beside it.

First, read Builder through its cards (python3 <project>/tools/ask.py
--start, then ask.py "question"; open a Builder file only where a card
points) and write the design journal's opening entry: how you will use
Builder for this game, the rules you will hold to and what holds each, and
your questions as answer chips.

[PERMISSIONS: e.g. "Do not deploy." / "You may create a public repository
named ____ and deploy it to GitHub Pages when the checks pass." /
"Do not use subagents."]
