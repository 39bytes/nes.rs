// import { RingBuffer } from "./ring-buffer";
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
    // const output = outputs[0];
    // output.forEach((channel) => {
    //   for (let i = 0; i < channel.length; i++) {
    //     channel[i] = this.buffer.pop();
    //   }
    //   // console.log(channel);
    // });
    return true;
  }
}

registerProcessor("nes-audio", NesAudio);
