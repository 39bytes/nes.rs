dk:
    RUST_LOG=info cargo run --release assets/test_roms/donkeykong.nes

nestest:
    RUST_LOG=info cargo run --release assets/test_roms/nestest.nes

mario:
    RUST_LOG=info cargo run --release assets/test_roms/mario.nes

debug ROM_PATH:
    RUST_LOG=info cargo run {{ROM_PATH}}

profile:
    valgrind --tool=cachegrind target/release/nesrs assets/test_roms/mario.nes
