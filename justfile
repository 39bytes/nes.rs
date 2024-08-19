fmt:
    cargo +nightly fmt --all --check

lint:
    cargo clippy --all

debug ROM_PATH:
    RUST_LOG=info cargo run {{ROM_PATH}}

profile:
    CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -- assets/test_roms/mario.nes

build-wasm:
    ./build-wasm.sh

dev-wasm:
    cd web && pnpm dev


# Rom tests
dk:
    RUST_LOG=info cargo run assets/test_roms/donkeykong.nes

nestest:
    RUST_LOG=info cargo run assets/test_roms/nestest.nes

mario:
    RUST_LOG=info cargo run assets/test_roms/mario.nes
