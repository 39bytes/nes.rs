import { Nes } from "emu-wasm";
import { getControllerInput } from "./input";
import NesAudioWorkletUrl from "./audio?worker&url";

const SCREEN_WIDTH = 256;
const SCREEN_HEIGHT = 240;
const FPS = 60;

const canvas = document.getElementById(
  "nes-screen-canvas",
) as HTMLCanvasElement;
canvas.width = SCREEN_WIDTH;
canvas.height = SCREEN_HEIGHT;

const romSelect = document.getElementById(
  "rom-select-input",
) as HTMLInputElement;

const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Couldn't get 2D canvas context");
}

const handleRomSelect = async (e: Event) => {
  const target = e.target as HTMLInputElement;
  const file = target.files?.[0];
  if (!file) {
    throw new Error("No file");
  }
  const bytes = new Uint8Array(await file.arrayBuffer());

  await initialize(bytes);
};
romSelect.onchange = handleRomSelect;

const drawScreen = (nes: Nes) => {
  let buf = nes.screen();
  const imageData = ctx.getImageData(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
  const pixels = imageData.data;
  for (let i = 0; i < SCREEN_WIDTH * SCREEN_HEIGHT; i += 1) {
    const bufIdx = i * 3;
    const pxIdx = i * 4;

    pixels[pxIdx] = buf[bufIdx];
    pixels[pxIdx + 1] = buf[bufIdx + 1];
    pixels[pxIdx + 2] = buf[bufIdx + 2];
    pixels[pxIdx + 3] = 255;
  }

  ctx.putImageData(imageData, 0, 0);
};

class NesWorkletNode extends AudioWorkletNode {
  constructor(context: BaseAudioContext, options?: AudioWorkletNodeOptions) {
    super(context, "nes-audio", options);
  }
  queue(samples: Float32Array) {
    this.port.postMessage(samples);
  }
}

const initialize = async (rom: Uint8Array) => {
  const audioContext = new AudioContext();
  const nes = Nes.new(audioContext.sampleRate);
  console.log("Initializing");
  console.log(audioContext.sampleRate);
  await audioContext.audioWorklet.addModule(NesAudioWorkletUrl);
  const nesWorkletNode = new NesWorkletNode(audioContext);
  nesWorkletNode.connect(audioContext.destination);

  console.log(`Loaded rom (size: ${rom.length})`);
  nes.load_rom(rom);

  const renderLoop = () => {
    const input = getControllerInput();
    nes.trigger_inputs(input);

    nes.advance_frame();
    const samples = nes.audio_samples();
    nesWorkletNode.queue(samples);
    nes.clear_audio_samples();
    drawScreen(nes);
  };

  nes.reset();
  setInterval(renderLoop, (1 / FPS) * 1000);
};
