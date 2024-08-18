import { RingBuffer } from "./ring-buffer";

class NesAudio extends AudioWorkletProcessor {
  buffer: RingBuffer;

  constructor(options: any) {
    super();
    this.buffer = new RingBuffer(8192);
    this.port.addEventListener("message", (e) => {
      this.buffer.queue(e.data);
    });
    this.port.start();
  }

  process(
    _inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>,
  ) {
    const output = outputs[0];
    output.forEach((channel) => {
      for (let i = 0; i < channel.length; i++) {
        channel[i] = this.buffer.pop();
      }
      // console.log(channel);
    });
    return true;
  }
}

registerProcessor("nes-audio", NesAudio);
