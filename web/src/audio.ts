import { AudioReader, RingBuffer } from "ringbuf.js";

class NesAudio extends AudioWorkletProcessor {
  audioReader: AudioReader;

  constructor(options: any) {
    super();
    const { audioQueue } = options.processorOptions;
    this.audioReader = new AudioReader(
      new RingBuffer(audioQueue, Float32Array),
    );
  }

  process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>,
  ) {
    this.audioReader.dequeue(outputs[0][0]);
    return true;
  }
}

registerProcessor("nes-audio", NesAudio);
