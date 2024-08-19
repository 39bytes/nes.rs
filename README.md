# nes.rs

An emulator for the [Nintendo Entertainment System](https://en.wikipedia.org/wiki/Nintendo_Entertainment_System) written in Rust, with SDL2 and web frontends.

[Try it out in your browser!](https://39bytes.github.io/nes.rs/)

https://github.com/39bytes/nes.rs/assets/47371088/69a3ce36-a5a2-4d11-b13a-cede656cec81

This isn't meant to be a full-featured or 100% accurate emulator, but rather a fun educational project.
It still aims to be fairly accurate, emulating some hardware quirks such as unofficial CPU opcodes.

## Running (SDL2)
Ensure that you have [cargo](https://doc.rust-lang.org/cargo/) installed.
```
cargo build --release
cd target/release
./nesrs <path-to-rom>
```
Controls are bound to:
- Z: B
- X: A
- A: Select
- S: Start
- Arrow keys: Up/Down/Left/Right

There are 5 save state slots. Save states can be loaded with the number keys 1-5, and written with SHIFT + [number].

## Running the WASM frontend locally
Ensure that you have [wasm-pack](https://rustwasm.github.io/wasm-pack/), [Node](https://nodejs.org/en) and [pnpm](https://pnpm.io/) installed.
First, build the wasm-package:
```
cd emu-wasm
wasm-pack build
```
Then just spin up the frontend:
```
cd web
pnpm i
pnpm dev
```

## Testing
```
cargo test
```
This will run test ROMs for the CPU emulation.

### Passing tests
- [x] `nestest` (kevtris)
- [x] `instr_test-v5` (blargg)

## Compatability
[iNES Mappers](https://www.nesdev.org/wiki/Mapper#iNES_1.0_mapper_grid) 0, 1, 2, 3, 4 and 9 are supported. 
Any game that uses a different mapper will not work for now. 
To find out which mapper a game uses, search it on [NesCartDB](https://nescartdb.com/).

## TODO
### Emulation
- [x] Accurate audio emulation
    - [x] Implement DMC channel for APU
    - [x] Investigate issues with envelope/sweep unit
    - [x] Fix issues with buffer underrun/overrun and reduce audio latency
- [x] Investigate performance issues
- [ ] Implement more mappers 
    - [x] Mapper 4 (MMC3)
    - [ ] Mapper 5 (MMC5)
    - [ ] Mapper 7 (AxROM)
    - [x] Mapper 9 (MMC2)
    - [ ] Mapper 10 (MMC4)
    - [ ] Mapper 66 (GxROM)
- [ ] Run test ROMs for PPU emulation

### QOL
- [x] Save states
- [ ] Remappable controls

## Resources
- [Nesdev Wiki](https://www.nesdev.org/wiki/Nesdev_Wiki)
- [6502 CPU opcode reference](https://www.nesdev.org/obelisk-6502-guide/reference.html)
- [Unofficial opcodes](https://www.oxyron.de/html/opcodes02.html)
- [NES test roms](https://github.com/christopherpow/nes-test-roms)
- [NesCartDB](https://nescartdb.com/)
