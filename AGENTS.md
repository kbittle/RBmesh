# AGENTS.md

Rust workspace for an STM32WL/LoRa-E5 mesh modem. Two crates: `lora_e5_package` (firmware binary) and `rb_mesh_lib` (no_std mesh library `bm_network`).

## Commands

- Build firmware: `cargo build --release` (default target `thumbv7m-none-eabi` from `.cargo/config.toml`)
- CI-style build: `cargo build --release --target thumbv7m-none-eabi`
- Flash (probe-rs): `cargo flash --release --chip STM32WLE5JC`
- Debug + RTT: `cargo embed --release`
- Host unit tests: `cargo test -p bm_network --target x86_64-unknown-linux-gnu`
- Host tests with output: `cargo test -p bm_network --target x86_64-unknown-linux-gnu -- --nocapture`
- Single test: `cargo test -p bm_network --target x86_64-unknown-linux-gnu <test_name>`

Do not run bare `cargo test`: the default target is `thumbv7m-none-eabi` and the firmware has no std.

## Structure

- Firmware entrypoint: `lora_e5_package/src/main.rs` (RTIC app; radio, AT commands, GPIO).
- Mesh library: `rb_mesh_lib/` (`lib.rs`, engine, routing table, node, packet).
- Compile-time config: `rb_mesh_lib/bm_network_configs.rs`.
- Tests: `rb_mesh_lib/tests/network_tests.rs` plus inline tests in `rb_mesh_lib/bm_network_engine.rs`.

## Quirks

- `.cargo/config.toml` sets default target `thumbv7m-none-eabi` and link args (`link.x`, `defmt.x`); tests must override target to `x86_64-unknown-linux-gnu`.
- `rb_mesh_lib` is `no_std` on device (`#![cfg_attr(not(test), no_std)]`); host tests use stubs in `lib.rs` for critical-section and defmt.
- `rust-toolchain.toml` pins stable with default target `thumbv7m-none-eabi`.
- `DEFMT_LOG=trace` is set in `.cargo/config.toml`.
- `lora_e5_package` has `test = false`; only `rb_mesh_lib` is testable.
- VS Code: rust-analyzer target `thumbv7m-none-eabi`, `allTargets = false`.
