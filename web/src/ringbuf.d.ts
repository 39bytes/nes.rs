declare module "ringbuf.js" {
  class RingBuffer {
    static getStorageForCapacity(
      capacity: number,
      type: ArrayLike,
    ): SharedArrayBuffer;
    constructor(sab: SharedArrayBuffer, type: ArrayLike);
    push(elements: ArrayLike, length?: number, offset?: number);
    pop(elements: ArrayLike, length?: number, offset?: number);
  }

  class AudioWriter {
    constructor(ringbuf: RingBuffer);
    enqueue(buf: Float32Array);
    available_write();
  }

  class AudioReader {
    constructor(ringbuf: RingBuffer);
    dequeue(buf: Float32Array);
    available_read();
  }
}
