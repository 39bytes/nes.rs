export class RingBuffer {
  private buffer: Float32Array;
  private readIndex: number;
  private writeIndex: number;

  constructor(size: number) {
    this.buffer = new Float32Array(size);
    this.readIndex = 0;
    this.writeIndex = 0;
  }

  pop() {
    if (this.length() == 0) {
      console.log("Buffer underrun");
      return 0;
    }
    const val = this.buffer[this.readIndex];
    this.readIndex = (this.readIndex + 1) % this.buffer.length;
    return val;
  }

  queue(samples: Float32Array) {
    for (let i = 0; i < samples.length; i++) {
      if (this.length() == this.buffer.length) {
        console.log("Buffer overrun");
        return;
      }
      this.buffer[this.writeIndex] = samples[i];
      this.writeIndex = (this.writeIndex + 1) % this.buffer.length;
    }
  }

  length() {
    const num = this.writeIndex - this.readIndex;
    const size = this.buffer.length;
    return (((num % size) + size) % size) + 1;
  }

  getBuffer() {
    return this.buffer;
  }
}
