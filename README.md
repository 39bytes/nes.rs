# nes.rs

(WIP) An emulator for the [(Nintendo Entertainment System)](https://en.wikipedia.org/wiki/Nintendo_Entertainment_System) written in Rust.
![video](https://github.com/39bytes/nes.rs/assets/47371088/6206ec77-0d29-4c09-b8a4-913615860527)


This isn't meant to be a full-featured or 100% accurate emulator, but rather a fun educational project.
It still aims to be fairly accurate, emulating some hardware quirks such as:
- Unofficial CPU opcodes
- Sprite evaluation bug
- CPU Page boundary bug

## Running
Ensure that you have [cargo](https://doc.rust-lang.org/cargo/) installed.
```
cargo build --release
cd target/release
./nesrs <path-to-rom>
```
**NOTE:** Audio emulation is not fully completed yet so game audio will sound a bit off.

## Testing
```
cargo test
```
This will run test ROMs for the CPU emulation.

### Passing tests
- [x] `nestest` (kevtris)
- [x] `instr_test-v5` (blargg)

## Compatability
[iNES Mappers](https://www.nesdev.org/wiki/Mapper#iNES_1.0_mapper_grid) 0, 1, 2, and 3 are supported. 
Any game that uses a different mapper will not work for now. 
To find out which mapper a game uses, search it on [NesCartDB](https://nescartdb.com/).

## TODO
- [ ] Accurate audio emulation
    - [ ] Implement DCPM channel for APU
    - [ ] Investigate issues with envelope/sweep unit
- [ ] Investigate performance issues
- [ ] Implement mappers 4, 5, 7, 9, 10 and 66
- [ ] Run test ROMs for PPU emulation
- [ ] Open bus behavior emulation
- [ ] Cycle accurate sprite evaluation/drawing

## Resources
- [Nesdev Wiki](https://www.nesdev.org/wiki/Nesdev_Wiki)
- [6502 CPU opcode reference](https://www.nesdev.org/obelisk-6502-guide/reference.html), 
- [Unofficial opcodes](https://www.oxyron.de/html/opcodes02.html)
- [NES test roms](https://github.com/christopherpow/nes-test-roms)
- [NesCartDB](https://nescartdb.com/)
