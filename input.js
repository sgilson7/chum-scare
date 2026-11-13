// Keys and the mouse, as the core's bits (after Chumontillado's input.js).
// data/controls.json names the action each key code holds; the core's
// numbers() gives each action its bit. Mouse movement adds up between
// frames; pointer lock keeps it coming when the mouse leaves the window.
export function makeInput(canvas, controls, actions) {
  const bit = new Map(actions);
  const held = new Set();
  let dx = 0;
  let dy = 0;
  // a browser that refuses pointer lock (the realtime kit's fallback): the
  // mouse's plain movement looks around instead
  let fallback = false;
  document.addEventListener("pointerlockerror", () => {
    fallback = true;
  });
  const actionOf = (code) => controls[code];
  const press = (code) => {
    const a = actionOf(code);
    if (a && bit.has(a)) held.add(a);
    return Boolean(a);
  };
  const release = (code) => {
    const a = actionOf(code);
    if (a) held.delete(a);
  };
  addEventListener("keydown", (e) => {
    if (press(e.code)) e.preventDefault();
  });
  addEventListener("keyup", (e) => release(e.code));
  canvas.addEventListener("mousedown", (e) => {
    press("Mouse" + e.button);
    e.preventDefault();
  });
  addEventListener("mouseup", (e) => release("Mouse" + e.button));
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  let last = null;
  addEventListener("mousemove", (e) => {
    if (document.pointerLockElement === canvas) {
      dx += e.movementX;
      dy += e.movementY;
    } else if (fallback && last) {
      // without the lock: how far the pointer moved since the last event
      dx += e.clientX - last[0];
      dy += e.clientY - last[1];
    }
    last = [e.clientX, e.clientY];
  });
  addEventListener("blur", () => held.clear());
  return {
    lock() {
      if (canvas.requestPointerLock) {
        const r = canvas.requestPointerLock();
        if (r && r.catch) r.catch(() => {
          fallback = true;
        });
      } else {
        fallback = true;
      }
    },
    // Esc in the fallback pauses, as losing the lock does
    release() {
      fallback = false;
    },
    locked() {
      return document.pointerLockElement === canvas || fallback;
    },
    // the bits held now, and the mouse since the last call
    take() {
      let bits = 0;
      for (const a of held) bits |= bit.get(a);
      const out = [bits, dx, dy];
      dx = 0;
      dy = 0;
      return out;
    },
  };
}
