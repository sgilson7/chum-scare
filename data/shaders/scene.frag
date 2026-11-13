#version 300 es
// Light from where it comes from (journal tab 6): each working fixture the
// core lists, the flashlight with its shadow, a little ambient, fog toward
// the dark, the eye's exposure. Dust settles on faces that look up; grime
// blotches everything. Every constant is a uniform from data/display.json.
precision highp float;
precision highp sampler2DShadow;
in vec3 vWorld;
in vec3 vNormal;
in vec3 vColor;
in float vMaterial;
in float vGlow;
uniform vec3 uEye;
uniform vec4 uLightPos[MAX_LIGHTS];
uniform vec4 uLightCol[MAX_LIGHTS];
uniform float uLightCount;
uniform vec4 uFlashPos;
uniform vec4 uFlashDir;
uniform vec4 uFlashCol;
uniform mat4 uFlashVP;
uniform vec4 uAmbient;
uniform vec4 uFog;
uniform vec4 uTime;
uniform vec4 uK0;
uniform vec4 uK1;
uniform vec4 uK2;
uniform vec4 uK3;
uniform vec4 uDust;
uniform vec4 uMat[MAT_COUNT];
uniform sampler2DShadow uShadow;
out vec4 outColor;

float hash(vec3 p) {
  return fract(sin(dot(p, uK2.xyz)) * uK1.y);
}

float noise(vec3 p) {
  vec3 i = floor(p);
  vec3 f = fract(p);
  float a = mix(hash(i), hash(i + vec3(1.0, 0.0, 0.0)), f.x);
  float b = mix(hash(i + vec3(0.0, 1.0, 0.0)), hash(i + vec3(1.0, 1.0, 0.0)), f.x);
  float c = mix(hash(i + vec3(0.0, 0.0, 1.0)), hash(i + vec3(1.0, 0.0, 1.0)), f.x);
  float d = mix(hash(i + vec3(0.0, 1.0, 1.0)), hash(i + vec3(1.0, 1.0, 1.0)), f.x);
  return mix(mix(a, b, f.y), mix(c, d, f.y), f.z);
}

float falloff(float d, float r) {
  float q = d / r;
  float edge = clamp(1.0 - q * q * q * q, 0.0, 1.0);
  return edge * edge / (d * d + 1.0);
}

void main() {
  vec4 mat = uMat[int(vMaterial + uK0.x)];
  vec3 n = normalize(vNormal);
  if (!gl_FrontFacing) n = -n;
  // grime: two octaves of blotches; dust: on faces that look up
  float g = noise(vWorld * mat.w) * noise(vWorld * mat.w * uK3.x + uK3.y);
  vec3 base = vColor * (1.0 - mat.y * g);
  float up = clamp(n.y, 0.0, 1.0);
  base = mix(base, uDust.rgb * vColor.g + uDust.rgb * uK0.x, mat.z * up * (uK0.x + uK0.x * noise(vWorld * uK1.x)));
  vec3 V = normalize(uEye - vWorld);
  vec3 col = uAmbient.rgb * base;
  int count = int(uLightCount + uK0.x);
  for (int i = 0; i < MAX_LIGHTS; i++) {
    if (i >= count) break;
    vec3 L = uLightPos[i].xyz - vWorld;
    float d = length(L);
    L /= max(d, uK2.w);
    float att = falloff(d, uLightPos[i].w);
    float diff = max(dot(n, L), 0.0);
    vec3 H = normalize(L + V);
    float spec = pow(max(dot(n, H), 0.0), mat.x) * uK1.z * (1.0 - g);
    col += uLightCol[i].rgb * att * (base * diff + vec3(spec) * diff);
  }
  // the flashlight: a cone with a hot middle, its shadow from the map
  vec3 Lf = uFlashPos.xyz - vWorld;
  float df = length(Lf);
  Lf /= max(df, uK2.w);
  float cosA = dot(-Lf, uFlashDir.xyz);
  float cone = smoothstep(uFlashDir.w, uFlashCol.w, cosA);
  float ring = 1.0 - uK3.z * smoothstep(uFlashCol.w, 1.0, cosA) * uK3.w * (1.0 - smoothstep(uFlashCol.w, 1.0, cosA));
  vec4 lp = uFlashVP * vec4(vWorld + n * uK2.w * uK3.w, 1.0);
  vec3 sp = lp.xyz / lp.w * uK0.x + uK0.x;
  float lit = 1.0;
  if (lp.w > 0.0 && sp.x > 0.0 && sp.x < 1.0 && sp.y > 0.0 && sp.y < 1.0 && sp.z < 1.0) {
    lit = texture(uShadow, vec3(sp.xy, sp.z - uK2.w));
  }
  float attf = clamp(1.0 - df / uFlashPos.w, 0.0, 1.0);
  attf *= attf;
  float difff = max(dot(n, Lf), 0.0);
  vec3 Hf = normalize(Lf + V);
  float specf = pow(max(dot(n, Hf), 0.0), mat.x) * uK1.z * (1.0 - g);
  col += uFlashCol.rgb * cone * ring * attf * lit * (base * difff + vec3(specf) * difff);
  // things that glow: lamps, lenses, eyes
  col += vColor * vGlow;
  // fog toward the dark, then the eye's exposure and a soft curve
  float fog = exp(-uAmbient.w * length(uEye - vWorld));
  col = mix(uFog.rgb, col, fog);
  col *= uFog.w;
  col = col / (1.0 + col);
  col = pow(col, vec3(uK0.z));
  // film grain
  col += (hash(vec3(gl_FragCoord.xy, uTime.x)) - uK0.x) * uK1.w;
  outColor = vec4(col, 1.0);
}
