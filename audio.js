// Sound (journal tab 4): the page plays what the core says, where the core
// says. Each voice is a recipe (data/sounds.json) synthesized from the
// core's noise, then filtered at the core's cut-off, set to the core's
// gain, and placed with a PannerNode at the core's apparent position (its
// own roll-off off: the core alone decides loudness). Each voice also sends
// to the reverb of the room it was made in and the listener's room; the
// impulse responses come from the core. Mono sends the mix through one
// channel, which the browser spreads to both ears (CH-6). Nothing plays
// until the reader clicks or presses a key.
export function makeAudio(recipes, rooms, makeNoise, makeImpulse) {
  const K = recipes.constants;
  let ctx = null;
  let master = null;
  let mono = null;
  let out = null;
  let noise = null;
  let reverbs = [];
  let model = "HRTF";
  let isMono = false;
  // the player's volume, 0 to 1, from the core's settings
  let level = 1;
  const beds = new Map();
  const sec = (ms) => ms / K.thousand;

  function route() {
    if (!ctx) return;
    master.disconnect();
    mono.disconnect();
    if (isMono) {
      master.connect(mono);
      mono.connect(out);
    } else {
      master.connect(out);
    }
  }

  function unlock() {
    if (ctx) {
      if (ctx.state === "suspended") ctx.resume();
      return;
    }
    const Context = window.AudioContext || window.webkitAudioContext;
    if (!Context) return;
    ctx = new Context();
    const [threshold, knee, ratio, attack, release] = K.compressor;
    out = ctx.createDynamicsCompressor();
    out.threshold.value = threshold;
    out.knee.value = knee;
    out.ratio.value = ratio;
    out.attack.value = attack;
    out.release.value = release;
    out.connect(ctx.destination);
    master = ctx.createGain();
    master.gain.value = K.master * level;
    // one channel, explicitly: the browser sums left and right into it
    mono = ctx.createGain();
    mono.channelCount = 1;
    mono.channelCountMode = "explicit";
    mono.channelInterpretation = "speakers";
    route();
    noise = ctx.createBuffer(1, K.noise_samples, ctx.sampleRate);
    noise.copyToChannel(makeNoise(K.noise_samples), 0);
    reverbs = rooms.reverbs.map((r) => {
      const len = (r.decay_ms * ctx.sampleRate) / K.thousand;
      const buf = ctx.createBuffer(1, len, ctx.sampleRate);
      buf.copyToChannel(makeImpulse(r.decay_ms, ctx.sampleRate, len), 0);
      const conv = ctx.createConvolver();
      conv.buffer = buf;
      const wet = ctx.createGain();
      wet.gain.value = r.wet;
      conv.connect(wet).connect(master);
      return conv;
    });
  }

  function envelope(peak, ms, at) {
    const g = ctx.createGain();
    g.gain.setValueAtTime(K.silence, at);
    g.gain.linearRampToValueAtTime(peak, at + sec(K.attack_ms));
    g.gain.exponentialRampToValueAtTime(K.silence, at + sec(ms));
    return g;
  }

  function panner(x, y, z) {
    const p = ctx.createPanner();
    p.panningModel = model;
    p.distanceModel = "inverse";
    p.refDistance = K.ref_distance;
    p.rolloffFactor = K.rolloff;
    p.positionX.value = x;
    p.positionY.value = y;
    p.positionZ.value = z;
    return p;
  }

  function voice(v, hrtf) {
    const r = recipes.voices[v.recipe];
    if (!r || v.gain <= 0) return;
    const at = ctx.currentTime;
    const vary = (v.seed % K.variety) / K.variety;
    const gain = ctx.createGain();
    gain.gain.value = v.gain / K.thousand;
    const lp = ctx.createBiquadFilter();
    lp.type = "lowpass";
    lp.frequency.value = v.hz;
    const p = panner(v.x, v.y, v.z);
    if (!hrtf) p.panningModel = "equalpower";
    gain.connect(lp).connect(p).connect(master);
    const send = ctx.createGain();
    send.gain.value = v.send / K.thousand;
    lp.connect(send).connect(reverbs[v.room] || master);
    if (v.send_here > 0) {
      const here = ctx.createGain();
      here.gain.value = v.send_here / K.thousand;
      lp.connect(here).connect(reverbs[v.here] || master);
    }
    for (let k = 0; k < r.repeat; k += 1) {
      const t = at + sec(k * r.repeat_ms);
      const pitch = 1 + (vary - K.half) * r.jitter;
      if (r.noise > 0) {
        const src = ctx.createBufferSource();
        src.buffer = noise;
        src.playbackRate.value = pitch;
        const f = ctx.createBiquadFilter();
        f.type = r.filter;
        f.frequency.value = r.filter_hz * pitch;
        f.Q.value = r.q;
        src.connect(f).connect(envelope(r.noise, r.noise_ms, t)).connect(gain);
        src.start(t, vary * noise.duration * (1 - vary));
        src.stop(t + sec(r.noise_ms));
      }
      if (r.tone > 0) {
        const osc = ctx.createOscillator();
        osc.type = r.wave;
        osc.frequency.setValueAtTime(r.tone_hz * pitch, t);
        osc.frequency.exponentialRampToValueAtTime(r.tone_to_hz * pitch, t + sec(r.tone_ms));
        osc.connect(envelope(r.tone, r.tone_ms, t)).connect(gain);
        osc.start(t);
        osc.stop(t + sec(r.tone_ms));
      }
    }
  }

  function bed(b) {
    let node = beds.get(b.id);
    const now = ctx.currentTime;
    if (!node) {
      const r = recipes.beds[b.recipe];
      if (!r) return;
      let src;
      if (r.kind === "noise") {
        src = ctx.createBufferSource();
        src.buffer = noise;
        src.loop = true;
      } else {
        src = ctx.createOscillator();
        src.type = r.wave;
        src.frequency.value = r.tone_hz;
      }
      const f = ctx.createBiquadFilter();
      f.type = r.filter;
      f.frequency.value = r.filter_hz;
      f.Q.value = r.q;
      const wob = ctx.createGain();
      wob.gain.value = r.level;
      const lfo = ctx.createOscillator();
      lfo.frequency.value = r.wobble_hz;
      const depth = ctx.createGain();
      depth.gain.value = r.level * r.wobble;
      lfo.connect(depth).connect(wob.gain);
      // a second, slower wobble at an unrelated rate, so the bed never
      // settles into a regular beat (T22)
      if (r.wobble2_hz) {
        const lfo2 = ctx.createOscillator();
        lfo2.frequency.value = r.wobble2_hz;
        const depth2 = ctx.createGain();
        depth2.gain.value = r.level * r.wobble2;
        lfo2.connect(depth2).connect(wob.gain);
        lfo2.start();
      }
      const gain = ctx.createGain();
      gain.gain.value = 0;
      const lp = ctx.createBiquadFilter();
      lp.type = "lowpass";
      const p = panner(b.x, b.y, b.z);
      src.connect(f).connect(wob).connect(gain).connect(lp).connect(p).connect(master);
      src.start();
      lfo.start();
      node = { src, lfo, gain, lp, p };
      beds.set(b.id, node);
    }
    node.seen = true;
    node.gain.gain.setTargetAtTime(b.gain / K.thousand, now, K.bed_glide);
    node.lp.frequency.setTargetAtTime(b.hz, now, K.bed_glide);
    node.p.positionX.setTargetAtTime(b.x, now, K.bed_glide);
    node.p.positionY.setTargetAtTime(b.y, now, K.bed_glide);
    node.p.positionZ.setTargetAtTime(b.z, now, K.bed_glide);
  }

  return {
    unlock,
    // "headphones", "speakers" or "mono", from the core's ui()
    setOutput(output) {
      model = output === "headphones" ? "HRTF" : "equalpower";
      isMono = output === "mono";
      route();
    },
    setVolume(v) {
      level = v;
      if (master) master.gain.value = K.master * level;
    },
    channels() {
      return isMono && mono ? mono.channelCount : out ? out.channelCount : 0;
    },
    update(sound, listener) {
      if (!ctx || ctx.state !== "running") return;
      const L = ctx.listener;
      const [x, y, z, fx, fy, fz, ux, uy, uz] = listener;
      if (L.positionX) {
        L.positionX.value = x;
        L.positionY.value = y;
        L.positionZ.value = z;
        L.forwardX.value = fx;
        L.forwardY.value = fy;
        L.forwardZ.value = fz;
        L.upX.value = ux;
        L.upY.value = uy;
        L.upZ.value = uz;
      } else {
        L.setPosition(x, y, z);
        L.setOrientation(fx, fy, fz, ux, uy, uz);
      }
      // the loudest voices get HRTF, the rest equal-power (R1.10)
      const order = sound.voices.slice().sort((a, b) => b.gain - a.gain);
      order.forEach((v, i) => voice(v, model === "HRTF" && i < K.hrtf_voices));
      for (const node of beds.values()) node.seen = false;
      sound.beds.forEach(bed);
      for (const [id, node] of beds) {
        if (!node.seen) {
          node.gain.gain.setTargetAtTime(0, ctx.currentTime, K.bed_glide);
          node.src.stop(ctx.currentTime + sec(K.fade_ms));
          node.lfo.stop(ctx.currentTime + sec(K.fade_ms));
          beds.delete(id);
        }
      }
    },
  };
}
