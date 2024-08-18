const keyDown: Record<string, boolean> = {};
window.onkeyup = (e: KeyboardEvent) => {
  keyDown[e.code] = false;
};
window.onkeydown = (e: KeyboardEvent) => {
  keyDown[e.code] = true;
};

export const getControllerInput = () => {
  let byte = 0;
  // A button
  if (keyDown["KeyX"]) {
    byte |= 1 << 0;
  }
  // B button
  if (keyDown["KeyZ"]) {
    byte |= 1 << 1;
  }
  // Select button
  if (keyDown["KeyA"]) {
    byte |= 1 << 2;
  }
  // Start button
  if (keyDown["KeyS"]) {
    byte |= 1 << 3;
  }
  // Arrows
  if (keyDown["ArrowUp"]) {
    byte |= 1 << 4;
  }
  if (keyDown["ArrowDown"]) {
    byte |= 1 << 5;
  }
  if (keyDown["ArrowLeft"]) {
    byte |= 1 << 6;
  }
  if (keyDown["ArrowRight"]) {
    byte |= 1 << 7;
  }

  return byte;
};
