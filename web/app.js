// The page: it loads the core and the data, then each frame gives the core
// the clock, the keys and the mouse, draws what it returns, plays what it
// lists, and shows the screen ui() describes. It decides nothing (card:spec.R6).
import init, { Game } from "./pkg/tool_shim.js";
import { makeRenderer } from "./gl.js";
import { makeAudio } from "./audio.js";
import { makeInput } from "./input.js";
import * as store from "./save/store.js";

const $ = (id) => document.getElementById(id);
const json = async (path) => (await fetch(path)).json();
const text = async (path) => (await fetch(path)).text();
const parse = (s) => JSON.parse(s);

async function main() {
  await init();
  const [strings, controls, sounds, acoustics, display, vertex, fragment, depthVertex, depthFragment] = await Promise.all([
    json("data/strings.json"),
    json("data/controls.json"),
    json("data/sounds.json"),
    json("data/acoustics.json"),
    json("data/display.json"),
    text("data/shaders/scene.vert"),
    text("data/shaders/scene.frag"),
    text("data/shaders/depth.vert"),
    text("data/shaders/depth.frag"),
  ]);
  const say = (key) => strings[key] || "";
  document.querySelectorAll("[data-text]").forEach((n) => {
    n.textContent = say(n.dataset.text);
  });

  // a seed, and the computer's minute of the day for the mall's clocks
  const seed = crypto.getRandomValues(new Uint32Array(1))[0];
  const now = new Date();
  const game = new Game(seed, now.getHours() * display.minutes_per_hour + now.getMinutes());
  let N = parse(game.numbers());

  const canvas = $("view");
  const renderer = makeRenderer(canvas, N, { vertex, fragment, depthVertex, depthFragment }, display);
  if (!renderer) {
    $("message").textContent = say("no_webgl");
    document.body.dataset.ready = "true";
    return;
  }
  renderer.setStatic(game.static_boxes());
  if (renderer.software) game.act("tier", "0");
  function size() {
    const [w, h] = game.resize(innerWidth, innerHeight, devicePixelRatio || 1);
    renderer.resize(w, h);
  }
  size();
  addEventListener("resize", size);

  const audio = makeAudio(sounds, acoustics, (n) => game.noise(n), (d, r, l) => game.impulse(d, r, l));
  const input = makeInput(canvas, controls, N.actions);
  // CH-Q6 A: the settings the core wrote last visit (kits/saves)
  const kept = store.read();
  if (kept) game.act("settings", kept);
  let ui = parse(game.ui());
  audio.setOutput(ui.output);
  audio.setVolume(ui.volume / N.pct);

  function act(name, value) {
    const r = parse(game.act(name, value));
    if (r.error) $("message").textContent = r.error;
    if (name === "tier") {
      N = parse(game.numbers());
      size();
    }
    ui = parse(game.ui());
    audio.setOutput(ui.output);
    audio.setVolume(ui.volume / N.pct);
    store.write(game.settings());
    pressed();
    // buttons are pressed only after setup, so draw and lastUi exist by then
    lastUi = "";
    draw();
  }
  // buttons show which setting is on
  function pressed() {
    const state = {
      output: ui.output,
      ring: ui.ring ? "on" : "off",
      subtitles: ui.subtitles ? "on" : "off",
      flashes: ui.reduce_flashes ? "reduce" : "keep",
      tier: String(ui.tier),
    };
    document.querySelectorAll("[data-act]").forEach((b) => b.setAttribute("aria-pressed", String(state[b.dataset.act] === b.dataset.value)));
    $("sens").textContent = String(ui.sensitivity);
    document.querySelectorAll("input.volume").forEach((v) => (v.value = String(ui.volume)));
  }

  // the title's steps: flashes, the blessing, how you listen, the doors
  const show = (id) => document.querySelectorAll(".step").forEach((s) => (s.hidden = s.id !== id));
  document.querySelectorAll("[data-act]").forEach((b) =>
    b.addEventListener("click", () => {
      audio.unlock();
      act(b.dataset.act, b.dataset.value);
      if (b.closest("#step-warn")) show("step-bless");
      if (b.closest("#step-listen")) show("step-go");
    }),
  );
  let holdStart = null;
  const hold = $("bless-hold");
  const holding = () => {
    if (holdStart === null) return;
    const done = (performance.now() - holdStart) / display.bless_hold_ms;
    hold.style.setProperty("--held", String(done * N.pct) + "%");
    if (done >= 1) {
      holdStart = null;
      $("candle").classList.add("lit");
      $("bless-lines").hidden = false;
      hold.hidden = true;
      $("bless-next").textContent = say("bless_done");
      $("bless-next").classList.remove("quiet");
      return;
    }
    requestAnimationFrame(holding);
  };
  hold.addEventListener("pointerdown", () => {
    audio.unlock();
    holdStart = performance.now();
    requestAnimationFrame(holding);
  });
  addEventListener("pointerup", () => {
    holdStart = null;
  });
  $("bless-next").addEventListener("click", () => show("step-listen"));
  $("enter").addEventListener("click", () => {
    audio.unlock();
    act("start", "");
    input.lock();
  });
  $("resume").addEventListener("click", () => input.lock());
  $("retry").addEventListener("click", () => {
    act("retry", "");
    input.lock();
  });
  $("again").addEventListener("click", () => {
    act("again", "");
    act("start", "");
    input.lock();
  });
  document.querySelectorAll("input.volume").forEach((v) =>
    v.addEventListener("input", () => {
      audio.unlock();
      act("volume", v.value);
    }),
  );
  $("slower").addEventListener("click", () => act("sensitivity", String(ui.sensitivity - display.sensitivity_step)));
  $("faster").addEventListener("click", () => act("sensitivity", String(ui.sensitivity + display.sensitivity_step)));
  canvas.addEventListener("click", () => {
    if (ui.phase === "play" && !input.locked()) input.lock();
  });

  // the tab knows when you leave (journal tab 5): its title calls you back
  const away = ["tab_away_one", "tab_away_two", "tab_away_three"];
  let awayAt = 0;
  document.addEventListener("visibilitychange", () => {
    if (document.hidden && ui.phase === "play") {
      document.title = say(away[awayAt % away.length]);
      awayAt += 1;
    } else {
      document.title = say("title");
    }
  });

  let shownLetters = 0;
  let wasBlackout = false;
  let lastUi = "";
  function draw() {
    document.body.dataset.phase = ui.phase;
    document.body.dataset.mode = ui.mode;
    document.body.dataset.output = ui.output;
    document.body.dataset.walked = String(ui.walked_cm);
    $("title-screen").hidden = ui.phase !== "title";
    $("hud").hidden = ui.phase !== "play";
    $("dead").hidden = ui.phase !== "dead";
    $("won").hidden = ui.phase !== "won";
    $("pause").hidden = !(ui.phase === "play" && !input.locked());
    if (ui.phase !== "play" && input.locked()) document.exitPointerLock();
    $("prompt").textContent = say(ui.prompt);
    $("hint").textContent = say("hint_" + ui.mode);
    document.body.classList.toggle("hurt", ui.hurt);
    // the direction ring and subtitles (CH-Q16 A)
    $("ring").hidden = !ui.ring;
    $("ring-marks").replaceChildren(
      ...ui.marks.map((m) => {
        const d = document.createElement("div");
        d.style.transform = "rotate(" + String(m.degrees) + "deg)";
        d.style.opacity = String(m.life / N.thousand);
        return d;
      }),
    );
    $("subs").hidden = !ui.subtitles;
    $("subs").replaceChildren(
      ...ui.marks.map((m) => {
        const li = document.createElement("li");
        li.textContent = say(m.key);
        li.style.opacity = String(m.life / N.thousand);
        return li;
      }),
    );
    // signs on the walls where they hang: the core's matrix carries each
    // from its centre onto the view (T4)
    $("signs").replaceChildren(
      ...ui.signs.map((s) => {
        const d = document.createElement("div");
        const t = document.createElement("span");
        t.textContent = say(s.key).split("{clock}").join(ui.clock);
        d.className = s.key === "sign_open_late" ? "neon" : "";
        d.style.transform = "matrix3d(" + s.m.map((v) => v / N.one).join(",") + ")";
        d.append(t);
        return d;
      }),
    );
    // R U N: each letter slams as the planchette reaches it
    const spans = $("run").children;
    const word = say("run_letters");
    while (shownLetters < ui.letters) {
      spans[shownLetters].textContent = word[shownLetters];
      spans[shownLetters].classList.add("slam");
      shownLetters += 1;
      document.body.classList.remove("shake");
      void document.body.offsetWidth;
      document.body.classList.add("shake");
    }
    if (ui.letters < shownLetters) {
      shownLetters = 0;
      for (const s of spans) s.classList.remove("slam");
      $("run").classList.remove("gone");
    }
    $("run").classList.toggle("gone", ui.blackout && ui.letter_ms > display.letter_slam_ms * spans.length);
    if (ui.blackout && !wasBlackout && !ui.reduce_flashes) {
      document.body.classList.remove("flash");
      void document.body.offsetWidth;
      document.body.classList.add("flash");
    }
    wasBlackout = ui.blackout;
  }

  document.addEventListener("pointerlockchange", draw);
  addEventListener("keydown", (e) => {
    if (e.code === "Escape" && ui.phase === "play") {
      input.release();
      draw();
    }
  });
  document.body.dataset.ready = "true";
  pressed();
  draw();

  function frame(t) {
    const [bits, dx, dy] = input.take();
    const live = ui.phase === "play" && input.locked();
    game.advance(t, live ? bits : 0, live ? dx : 0, live ? dy : 0);
    const u = game.ui();
    if (u !== lastUi) {
      lastUi = u;
      ui = parse(u);
      draw();
    }
    if (ui.phase === "play") renderer.frame(game.uniforms(), game.dynamic_boxes(), game.held_boxes(), N.shadow_size);
    audio.update(parse(game.sounds()), game.listener());
    document.body.dataset.channels = String(audio.channels());
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

main().catch((e) => {
  $("message").textContent = String(e && e.message ? e.message : e);
  document.body.dataset.ready = "true";
});
