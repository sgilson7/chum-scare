# Brief

Stage 1. The person's own words come first; the rest can be written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

> DESCRIPTION: you are walking through an old abandoned mall, with a 6 shooter revolver with no bullets loaded. you have a box of bullets with 6 bullets. to load the gun, you must look down, manually pop out the barrel, and physically pull the box of bullets out of your pocket with your hands conrolled by your mouse. however, whenver you look down, after getting one bullet loaded, things should start to happen in the background that can scare you, like noises from doors opening, things falling in the environment, things shifting in the background, etc. the enenmy in this game is a little evil spirit that becomes increasingly menacing the farther youg et into the mall. sometimes the spirit will posses objects like manikins, cars, etc that you may have to shoot to get them to stop. there is also a group of cultists in the middle of the mall doing a ritual that attack you, so there needs to be a stealth system to avoid the cultists, while the spirit actively harrasses you as you move through the mall. the goal should be to reach a specific cursed store, use the ouija board within to contact the spirit of your ancestor, who only says R U N in styled, red font that slams onto the screen menacingly, then any lights in the mall break, and the spirit begins to chase you and grow larger as it does, the cultists will also get killed by the spirit and draw its rage away from you; you can either hide or run until you lose the spirit, then hide until you can make your way back out of the mall. the emphasis of the game should be on things happening in your peripheral vision as you intearct with objects in the world. like your gun, or opening a door should require you to really spin the handle, so you can be spooked by thing sbehind you. in this game, we will need to focus on in depth sound plane simulation to make the sound come from the correct direction (while still having a mono option for my friend who isd eaf in one ear), so that scares can be done by having something that sounds like its behind you occur. so first write the design log journal on builder and how you'll use it to make this game, (use snapclass journal as an example of the level of animations, flow diagrams, mockups, etc that I want in my journals), then create the design journal interpreting this prompt into a summary, flow diagrams, animations, etc. the enemies in the game should be created in blender, so don't do the blender pipeline here but instead in the builder refined art style directory if you want to create creepy, uncanny looking models in blender for this game which would be perfect. emphasis needs to be placed on having the environment be old and dusty, and a lot of animations for the player character moving, like a realistic and well animation loop of you walking with the gun pointed upwards, slowly fidgeting. your foot steps should be heavy. the third design log tab should be a research paper on how to do 3d sound in a game, using arc raiders as a great example of that 3d sound we are looking for. the fourth tab in the design log is how you will implement the realistic sound scape in this horror game specifically. then, the fifth tab should be on how to perform a digital cleansing ritual over this computer to ensure the game is blessed, and some content in the game should reflect the fact that playing this game may cause untoward effects in real life, to add to the scaer factor, breaking the fourth wall a little bit, like signs that say "behind you!" and little cheeky stuff like that. then the 6th tab should be on making the lighting realistic, ie emanate from real sources that can be broken, and emanates from your flashlight realisiticly (which has a charge and needs to be shook to charge, can be in left hand while gun is in right for animations sake) please work until all the design log tabs i asked for are complete, then i will review, provide my design deciisions, then you will build the first version of the game and deploy it for my review.

The opening message also set the engine and the process (Chumontillado's engine, copied from `engine/chumontillado@c48d8dd/`, tuned for slower, more methodical play; the speed tech decided with the person in the journal; every `movement::TUNING` number given a horror value with its reason).

## The tool

**Chum Scare**: a first-person horror game in a desktop browser, opened from a GitHub Pages address with no install. The player walks an abandoned, dusty mall holding an empty six-shot revolver in the right hand and a shake-charged flashlight in the left. Loading the revolver, opening doors and using the ouija board are done with the hands, steered by the mouse, while scares happen outside the player's view and are placed by direction in sound. A spirit harasses the player and possesses objects; cultists in the centre court are avoided by stealth. Contacting the ancestor in the cursed store breaks every light and starts a chase; the player hides or runs, then gets out. Keyboard and mouse, headphones recommended, with a mono option.

## Done means

- The page loads from the GitHub Pages address in Chromium, Firefox and WebKit, makes no off-origin request, and holds 60 frames a second on the Medium tier in the scene `MEASUREMENTS.md` names.
- Loading a round takes these hand steps, with the mouse steering the hands and not the view: look down, open the cylinder, take the box from the pocket, put a round in a chamber, close the cylinder. A test walks the steps.
- After the first round is in, a scare can fire while the player is loading, and every reload or door scare is placed outside the player's view; a test shows a scare never spawns inside the view.
- Opening a door takes turning its handle with circles of the mouse; a test counts the turns.
- Sounds are placed in 3D (HRTF on headphones) with occlusion and paths through doorways computed in the core; a Mono setting sends every sound to both ears; the browser check turns Mono on.
- Every light in the mall is a source in the map that can be broken, and a broken light gives no light; the flashlight's charge runs down and shaking it charges it.
- The spirit escalates with depth into the mall, possesses at least a mannequin and a car, and a hit from the revolver stops a possession.
- Cultists see and hear the player (light, noise, sight lines), and the player can pass them unseen.
- The ouija board spells R U N in red letters that slam onto the screen; every light breaks; the spirit chases and grows; it can kill cultists and turn to them; the player can lose it by hiding or running, and leave by the exit.
- Speed tech the person did not keep cannot be done (tests).
- `./scripts/check.sh` passes.

## Decisions that stay with the person

- Which speed tech stays, and the feel numbers after the first play (journal CH-Q1 to CH-Q4).
- The models of the spirit, the cultists and the possessed objects, made in Blender in `~/Documents/BuilderRefinedArtstyle`, and any third-party sound or texture.
- How frightening and how violent the game is: the cultists' deaths and anything near gore.
- The fourth-wall content: each sign, message and line that talks to the player.
- Whether settings (Mono, mouse sensitivity) are kept between visits (D17).
- The scare list: which scares exist and how often they may fire.

## Accessibility

Direction is carried by sound and, on request, by a direction ring around the crosshair that marks where each important sound came from; the Mono setting sums both channels so a player deaf in one ear hears every sound. Light and dark are never the only cue for a state the player must act on: the flashlight charge has a needle and a sound, and a possessed object also shakes and hums. Subtitles name important sounds. Keyboard and mouse (pointer lock), on desktop. Strong flashes (the R U N slam, lights breaking) have a Reduce Flashes setting.

## Style

A description: an old, dusty, abandoned 1980s–90s indoor mall at night, lit only by what is still working; grime, dust motes in the flashlight beam, peeling signage, water stains. Figures (spirit, cultists, mannequins) are uncanny and made in Blender in `~/Documents/BuilderRefinedArtstyle`; until then, primitive-built stand-ins in Chumontillado's manner. The R U N letters are a heavy red display face.

## References

- **Chumontillado** (refs/chumontillado.md): the engine: Q16.16 movement (`movement.rs`, `TUNING`), input between ticks, collide-and-slide, the WebGL page, synthesized sound from recipes, primitive-built figures, the arms and hands modules.
- **Chumworld Arena** (refs/chumworld-arena.md): the Verlet arm steered by the mouse (`arm.rs`), for the hands during reloading, doors and the ouija board; the behaviour-tree pilots from data (`pilot.rs`), for the cultists and the spirit; hit feel that decays to zero; Watch mode for tuning the AI.

## Words

Terms from the description: revolver, cylinder (the description says "barrel"), chamber, round, box, possession, cultist, ritual, ouija board, planchette, cursed store, ancestor, flashlight, charge. They go into `GLOSSARY.md` before any string uses them.

## Autonomy

The plan is not approved in advance: the person reviews the journal, answers its questions, and then the agent builds the first version and deploys it for the person's review. Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request. The person's grant, word for word: "You may create a public repository named Chum Scare and deploy it to GitHub Pages when the checks pass." "Do not use subagents."

- Deploys: `when the checks pass`
- Subagents: `none`

## Journal

`default`

## Allowed core dependencies

`serde`, `serde_json` (reference default).
