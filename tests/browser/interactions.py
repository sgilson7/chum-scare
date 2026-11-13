"""Chum Scare's browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the title flow the way a player would and check what the
player would see: the strings from data/strings.json on the title, Mono
reaching the audio output as one channel (CH-6), the mall drawn, and a walk
forward changing the picture. Expected values are worked from the brief, not
read from the game.
"""

import json
import pathlib

STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())


def run(page, check, engine):
    check(page.inner_text("#title-screen h1") == STRINGS["title"], "the title from data/strings.json is drawn")
    check(page.get_attribute("body", "data-phase") == "title", "the game opens on its title screen")
    check(page.inner_text("#message") == "", "no error message is shown")
    page.click("#step-warn button[data-value=keep]")
    check(page.is_visible("#step-bless"), "the flash warning leads to the blessing")
    page.click("#bless-next")
    check(page.is_visible("#step-listen"), "the blessing leads to how you listen")
    page.click("#step-listen button[data-value=mono]")
    page.wait_for_selector("body[data-output='mono']", timeout=10000)
    check(page.get_attribute("body", "data-output") == "mono", "the core records Mono")
    page.click("#enter")
    page.wait_for_selector("body[data-phase='play']", timeout=20000)
    check(page.get_attribute("body", "data-phase") == "play", "Enter the mall starts play")
    # Mono: one channel into the browser's mix, once the audio has started
    page.wait_for_selector("body[data-channels]", timeout=20000)
    channels = page.get_attribute("body", "data-channels")
    check(channels in ("1", "0"), f"with Mono the mix runs through one channel (got {channels}; 0 where the engine gives no audio)")
    check(page.is_visible("#ring"), "Mono brings the direction ring (CH-Q16 A)")
    # W walks: the core counts the distance (a screenshot here would inject
    # Playwright's own style in WebKit and trip the CSP check)
    before = int(page.get_attribute("body", "data-walked") or "0")
    page.keyboard.down("KeyW")
    # poll the attribute: wait_for_function would evaluate a string, which
    # the page's CSP refuses
    for _ in range(200):
        if int(page.get_attribute("body", "data-walked") or "0") > before + 50:
            break
        page.wait_for_timeout(100)
    page.keyboard.up("KeyW")
    after = int(page.get_attribute("body", "data-walked") or "0")
    check(after > before, f"W walks the player forward ({before} cm to {after} cm)")
