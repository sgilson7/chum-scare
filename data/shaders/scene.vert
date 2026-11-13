#version 300 es
// One box: the unit cube's corner carried by the instance's three columns
// and translation, in metres, as floats from the shim. No number lives
// here (D3): array sizes are tokens the page fills from the core's numbers.
in vec3 aPos;
in vec3 aNormal;
in vec3 aC0;
in vec3 aC1;
in vec3 aC2;
in vec3 aT;
in vec3 aColor;
in float aMaterial;
in float aGlow;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec3 vColor;
out float vMaterial;
out float vGlow;
void main() {
  mat3 m = mat3(aC0, aC1, aC2);
  vec3 w = m * aPos + aT;
  vWorld = w;
  vNormal = normalize(m * aNormal);
  vColor = aColor;
  vMaterial = aMaterial;
  vGlow = aGlow;
  gl_Position = uVP * vec4(w, 1.0);
}
