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
const resetButton = document.getElementById(
  "nes-reset-button",
) as HTMLInputElement;
const volumeSlider = document.getElementById(
  "volume-slider",
) as HTMLInputElement;
const volumeLabel = document.getElementById("volume-label") as HTMLLabelElement;
const errorMessage = document.getElementById("error-message") as HTMLDivElement;

const ctx = canvas?.getContext("2d");
if (!ctx) {
  throw new Error("Couldn't get 2D canvas context");
}
ctx.imageSmoothingEnabled = false;

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

volumeSlider.onchange = (e) => {
  const val = parseFloat((e.target as HTMLInputElement).value);
  volumeLabel.innerText = Math.floor(val * 100).toString();
};

const withErrorReport = (f: () => void) => {
  try {
    f();
  } catch (e) {
    errorMessage.innerText = String(e);
  }
};

const resetSaveStateButtons = (nes: Nes) => {
  nes.clear_save_states();
  for (let slot = 1; slot <= 5; slot++) {
    const loadButton = document.getElementById(`slot-${slot}-load`)!;
    loadButton.style.display = "none";
  }
};

const setupSaveStateButtons = (nes: Nes) => {
  for (let slot = 1; slot <= 5; slot++) {
    const saveButton = document.getElementById(`slot-${slot}-save`)!;
    const loadButton = document.getElementById(`slot-${slot}-load`)!;
    saveButton.onclick = () => {
      withErrorReport(() => {
        nes.write_state(slot);
        loadButton.style.display = "block";
      });
    };
    loadButton.onclick = () => {
      withErrorReport(() => {
        nes.load_state(slot);
      });
    };
  }
};

let acc = 0;
let prev = 0;
let animId = 0;

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

  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      context.suspend();
    } else {
      context.resume();
      prev = performance.now();
    }
  });

  return { audioContext: context, audioWriter };
};

const renderLoop = (timestamp: number, nes: Nes, audioWriter: AudioWriter) => {
  animId = requestAnimationFrame((t) => renderLoop(t, nes, audioWriter));

  acc += timestamp - prev;
  let frameTicked = false;
  const input = getControllerInput();
  while (acc >= TIME_PER_FRAME) {
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
  cancelAnimationFrame(animId);
  const { audioContext, audioWriter } = await initAudio();
  const nes = Nes.new(audioContext.sampleRate);
  resetSaveStateButtons(nes);
  setupSaveStateButtons(nes);

  nes.set_volume(parseFloat(volumeSlider.value));

  resetButton.onclick = () => nes.reset();
  volumeSlider.onchange = (e) => {
    const val = parseFloat((e.target as HTMLInputElement).value);
    volumeLabel.innerText = Math.floor(val * 100).toString();
    nes.set_volume(val);
  };

  errorMessage.innerText = "";
  withErrorReport(() => nes.load_rom(rom));

  console.log(`Loaded rom (size: ${rom.length})`);
  nes.reset();

  nes.advance_frame();
  nes.advance_frame();
  audioContext.resume();

  animId = requestAnimationFrame((timestamp) => {
    prev = timestamp;
    renderLoop(timestamp, nes, audioWriter);
  });
};
