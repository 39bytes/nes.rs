dk:
    RUST_LOG=info cargo run --release assets/roms/donkeykong.nes

nestest:
    RUST_LOG=info cargo run --release assets/roms/nestest.nes

mario:
    RUST_LOG=info cargo run --release assets/roms/mario.nes

debug ROM_PATH:
    RUST_LOG=info cargo run {{ROM_PATH}}
