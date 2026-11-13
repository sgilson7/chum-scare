// The WebGL 2 renderer, after Chumontillado's gl.js (c48d8dd): it uploads
// what the core returns and draws it, and holds no layout or rule of its
// own. Every box is the unit cube carried by an instance's columns; the
// layout of an instance, the uniforms' names and lengths, and the shadow's
// size come from the core's numbers() (D3). Two passes: the flashlight's
// shadow, then the mall; the held revolver and flashlight are drawn last,
// over a cleared depth, so they never sink into a wall.

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  return program;
}

// Sets the core's uniforms, [name, call, length] each, from one array.
function setList(gl, program, list, values) {
  let at = 0;
  for (const [name, call, length] of list) {
    const loc = gl.getUniformLocation(program, name);
    const data = values.subarray(at, at + length);
    if (loc) {
      if (call === "uniformMatrix4fv") gl[call](loc, false, data);
      else gl[call](loc, data);
    }
    at += length;
  }
}

// Returns null when the browser has no WebGL 2; the page then says so.
export function makeRenderer(canvas, N, shaders, display) {
  const gl = canvas.getContext("webgl2", { antialias: true, alpha: false, powerPreference: "high-performance" });
  if (!gl) return null;
  const BYTES = Float32Array.BYTES_PER_ELEMENT;
  const fill = (src) => src.split("MAX_LIGHTS").join(String(N.max_lights)).split("MAT_COUNT").join(String(display.materials.length));
  const scene = link(gl, fill(shaders.vertex), fill(shaders.fragment));
  const depth = link(gl, shaders.depthVertex, shaders.depthFragment);

  // the cube: positions and normals, from the core
  const cubeFloats = new Float32Array(N.cube.map((v) => v / N.one));
  const cubeBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, cubeBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, cubeFloats, gl.STATIC_DRAW);
  const corners = cubeFloats.length / N.cube_stride;
  const half = N.cube_part;

  const buffers = { static: gl.createBuffer(), dynamic: gl.createBuffer(), held: gl.createBuffer() };
  const counts = { static: 0, dynamic: 0, held: 0 };

  function vaoFor(program) {
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const pos = gl.getAttribLocation(program, "aPos");
    const nor = gl.getAttribLocation(program, "aNormal");
    gl.bindBuffer(gl.ARRAY_BUFFER, cubeBuffer);
    gl.enableVertexAttribArray(pos);
    gl.vertexAttribPointer(pos, half, gl.FLOAT, false, N.cube_stride * BYTES, 0);
    if (nor !== -1) {
      gl.enableVertexAttribArray(nor);
      gl.vertexAttribPointer(nor, half, gl.FLOAT, false, N.cube_stride * BYTES, half * BYTES);
    }
    const attrs = N.attributes.map(([name, size, offset]) => ({ loc: gl.getAttribLocation(program, name), size, offset })).filter((a) => a.loc !== -1);
    for (const a of attrs) {
      gl.enableVertexAttribArray(a.loc);
      gl.vertexAttribDivisor(a.loc, 1);
    }
    return { vao, attrs };
  }
  const sceneVao = vaoFor(scene);
  const depthVao = vaoFor(depth);

  function drawSet(v, which) {
    if (!counts[which]) return;
    gl.bindBuffer(gl.ARRAY_BUFFER, buffers[which]);
    for (const a of v.attrs) gl.vertexAttribPointer(a.loc, a.size, gl.FLOAT, false, N.instance * BYTES, a.offset * BYTES);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, corners, counts[which]);
  }

  function upload(which, values, usage) {
    gl.bindBuffer(gl.ARRAY_BUFFER, buffers[which]);
    gl.bufferData(gl.ARRAY_BUFFER, values, usage);
    counts[which] = values.length / N.instance;
  }

  // the constants every frame shares: shader numbers and materials
  gl.useProgram(scene);
  for (const [name, values] of Object.entries(display.shader)) gl.uniform4fv(gl.getUniformLocation(scene, name), values);
  gl.uniform4fv(gl.getUniformLocation(scene, "uMat"), display.materials.flat());
  gl.uniform1i(gl.getUniformLocation(scene, "uShadow"), 0);

  // the flashlight's shadow map
  let shadowSize = 0;
  const shadowTex = gl.createTexture();
  const shadowFbo = gl.createFramebuffer();
  function shadowFor(size) {
    if (size === shadowSize) return;
    shadowSize = size;
    gl.bindTexture(gl.TEXTURE_2D, shadowTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, size, size, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_COMPARE_MODE, gl.COMPARE_REF_TO_TEXTURE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_COMPARE_FUNC, gl.LEQUAL);
    gl.bindFramebuffer(gl.FRAMEBUFFER, shadowFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, shadowTex, 0);
    gl.drawBuffers([gl.NONE]);
    gl.readBuffer(gl.NONE);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";

  return {
    name,
    software: display.software.some((s) => name.includes(s)),
    setStatic(values) {
      upload("static", values, gl.STATIC_DRAW);
    },
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
    },
    frame(uniforms, dynamic, held, shadowSizeNow) {
      upload("dynamic", dynamic, gl.STREAM_DRAW);
      upload("held", held, gl.STREAM_DRAW);
      gl.enable(gl.DEPTH_TEST);
      gl.disable(gl.BLEND);
      gl.disable(gl.CULL_FACE);
      gl.depthMask(true);
      // the shadow pass
      shadowFor(shadowSizeNow);
      gl.bindFramebuffer(gl.FRAMEBUFFER, shadowFbo);
      gl.viewport(0, 0, shadowSize, shadowSize);
      gl.clear(gl.DEPTH_BUFFER_BIT);
      gl.useProgram(depth);
      setList(gl, depth, N.uniforms, uniforms);
      gl.bindVertexArray(depthVao.vao);
      drawSet(depthVao, "static");
      drawSet(depthVao, "dynamic");
      // the mall
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.useProgram(scene);
      setList(gl, scene, N.uniforms, uniforms);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, shadowTex);
      gl.bindVertexArray(sceneVao.vao);
      drawSet(sceneVao, "static");
      drawSet(sceneVao, "dynamic");
      gl.clear(gl.DEPTH_BUFFER_BIT);
      drawSet(sceneVao, "held");
    },
  };
}
