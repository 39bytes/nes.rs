import { Nes } from "emu-wasm";

export const SCREEN_WIDTH = 256;
export const SCREEN_HEIGHT = 240;

const scratch = document.getElementById(
  "nes-screen-scratch",
) as HTMLCanvasElement;
const scratchCtx = scratch.getContext("2d")!;

export const drawScreen = (ctx: CanvasRenderingContext2D, nes: Nes) => {
  let buf = nes.screen();

  const imageData = new ImageData(SCREEN_WIDTH, SCREEN_HEIGHT);
  const pixels = imageData.data;

  for (let i = 0; i < SCREEN_HEIGHT * SCREEN_WIDTH; i += 1) {
    const bufIdx = i * 3;
    const pxIdx = i * 4;

    pixels[pxIdx] = buf[bufIdx];
    pixels[pxIdx + 1] = buf[bufIdx + 1];
    pixels[pxIdx + 2] = buf[bufIdx + 2];
    pixels[pxIdx + 3] = 255;
  }

  scratchCtx.putImageData(imageData, 0, 0);

  ctx.drawImage(scratchCtx.canvas, 0, 0, ctx.canvas.width, ctx.canvas.height);
};
