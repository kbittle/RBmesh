# RB Mesh

A mesh stack written in rust to run on the STM32WL / LoRa-E5 module.

## Commands:

Cargo init RBmesh
Cargo build --release

cargo test --features stm32wle5

openocd -f interface/stlink.cfg -f target/stm32wle5.cfg -c "program target/thumbv6m-none-eabi/release/stm32wle5_blinky verify reset exit"

https://jonathanklimt.de/electronics/programming/embedded-rust/rust-on-stm32-2/
https://github.com/stm32-rs/stm32wlxx-hal/blob/main/testsuite/README.md

MSVC toolchain doesnt work with windows:
https://asyncbulbs.blogspot.com/2017/06/workaround-for-rustc-with-new-visual.html

Setup probe-rs:
https://probe.rs/docs/getting-started/installation/
PS C:\Users\Kris Bittle\Documents\RBmesh> Set-ExecutionPolicy RemoteSigned -scope CurrentUser
PS C:\Users\Kris Bittle\Documents\RBmesh> irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex

https://probe.rs/docs/tools/cargo-flash/
"probe-rs list" to list programmers
[0]: STLink V2 -- 0483:3748:48 (ST-LINK)

Following command to load code:
cargo flash --release --chip STM32WLE5JC