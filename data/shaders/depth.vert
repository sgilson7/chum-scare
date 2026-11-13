#version 300 es
// The flashlight's shadow pass: the same boxes, seen from the flashlight.
in vec3 aPos;
in vec3 aC0;
in vec3 aC1;
in vec3 aC2;
in vec3 aT;
uniform mat4 uFlashVP;
void main() {
  vec3 w = mat3(aC0, aC1, aC2) * aPos + aT;
  gl_Position = uFlashVP * vec4(w, 1.0);
}
