import { Nes } from "emu-wasm";
import { getControllerInput } from "./input";
import NesAudioWorkletUrl from "./audio?worker&url";
import { drawScreen } from "./renderer";
import { AudioWriter, RingBuffer } from "ringbuf.js";

const FPS = 60.0988;
const TIME_PER_FRAME = 1000 / FPS;

const canvas = document.getElementById(
  "nes-screen-canvas",
) as HTMLCanvasElement;
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

const initAudio = async () => {
  const context = new AudioContext();
  context.suspend();

  await context.audioWorklet.addModule(NesAudioWorkletUrl);
  const sab = RingBuffer.getStorageForCapacity(4096, Float32Array);
  const rb = new RingBuffer(sab, Float32Array);
  const audioWriter = new AudioWriter(rb);

  const node = new AudioWorkletNode(context, "nes-audio", {
    processorOptions: {
      audioQueue: sab,
    },
  });
  node.connect(context.destination);

  return { audioContext: context, audioWriter };
};

let acc = 0;
let prev = 0;

const renderLoop = (timestamp: number, nes: Nes, audioWriter: AudioWriter) => {
  requestAnimationFrame((t) => renderLoop(t, nes, audioWriter));

  acc += timestamp - prev;
  let frameTicked = false;
  while (acc >= TIME_PER_FRAME) {
    const input = getControllerInput();
    nes.trigger_inputs(input);

    nes.advance_frame();
    const samples = nes.audio_samples();
    audioWriter.enqueue(samples);
    nes.clear_audio_samples();

    frameTicked = true;
    acc -= TIME_PER_FRAME;
  }

  if (frameTicked) {
    drawScreen(ctx, nes);
  }

  prev = timestamp;
};

const initialize = async (rom: Uint8Array) => {
  const { audioContext, audioWriter } = await initAudio();
  const nes = Nes.new(audioContext.sampleRate);

  console.log(`Loaded rom (size: ${rom.length})`);
  nes.load_rom(rom);
  nes.reset();

  nes.advance_frame();
  nes.advance_frame();
  audioContext.resume();

  requestAnimationFrame((timestamp) => {
    prev = timestamp;
    renderLoop(timestamp, nes, audioWriter);
  });
};
