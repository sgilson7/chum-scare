# Chum Scare

A first-person horror game for a desktop browser. You walk an abandoned mall at night with an empty six-shot revolver in your right hand and a shake-charged flashlight in your left. Loading the revolver takes your eyes and both hands, and that is when the mall moves behind you. Avoid the cultists in the centre court, reach the cursed store, use the ouija board, and then get out.

Play it at https://sgilson7.github.io/chum-scare/. Headphones are best; a Mono setting puts every sound in both ears and turns on a direction ring and subtitles.

## Controls

Mouse looks. WASD walks, Shift sprints, C crouches, Space hops. Left button fires. Look down and hold the right button to load: flick left to open the cylinder, reach to your pocket, take a round, set it in a chamber, flick right to close. Hold the right button on a door handle and draw circles to turn it. Tap F for the flashlight; hold F and shake the mouse to charge it. E hides and takes. Esc pauses.

## How it is built

Built with Builder: every rule lives in a Rust core (`crates/core`) with no input or output, a WebAssembly shim moves values, and the page in `web/` draws and plays what the core returns. Movement is copied from Chumontillado's engine and slowed for methodical play. Sound is placed through the mall's rooms and doorways by the core and played with the browser's Web Audio panner.

```bash
cargo test --workspace
./scripts/build.sh
python3 scripts/serve.py
./scripts/check.sh
```

Nothing leaves the page: no server, no accounts, no analytics. The only thing kept between visits is your settings.
