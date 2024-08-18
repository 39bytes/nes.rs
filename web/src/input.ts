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
    byte = byte | (1 << 0);
  }
  // B button
  if (keyDown["KeyZ"]) {
    byte = byte | (1 << 1);
  }
  // Select button
  if (keyDown["KeyA"]) {
    byte = byte | (1 << 2);
  }
  // Start button
  if (keyDown["KeyS"]) {
    byte = byte | (1 << 3);
  }
  // Arrows
  if (keyDown["ArrowUp"]) {
    byte = byte | (1 << 4);
  }
  if (keyDown["ArrowDown"]) {
    byte = byte | (1 << 5);
  }
  if (keyDown["ArrowLeft"]) {
    byte = byte | (1 << 6);
  }
  if (keyDown["ArrowRight"]) {
    byte = byte | (1 << 7);
  }

  return byte;
};
