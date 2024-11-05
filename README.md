# RB Mesh
[![Build](https://github.com/kbittle/RBmesh/actions/workflows/rust.yml/badge.svg)]

A mesh stack written in rust to run on the STM32WL / LoRa-E5 module. Like the LoRaWAN modem software Seed Studio provides with the hardware. This software package will also support a AT command set to interact with the modem. This software package will also supply GPIO support for radio TX/RX and incoming message ring indication.

This design takes concepts from the RadioHead library to form routes between nodes. Combined with a volatile RAM based routing table to store those paths. Message routes are prioritized by shortest distance and then by best signal strength. 

## Build/Run/Test Commands:
General rust/cargo command to initialize project space:<br />
`Cargo init RBmesh`

General rust/cargo command to build application:<br />
`Cargo build --release`

General rust/cargo command to run tests: **(have not added tests yet)**<br />
`cargo test --features stm32wle5`

Command to load code on platform: **requires probe-rs**<br />
`cargo flash --release --chip STM32WLE5JC`

Command to load and debug code on platform: **requires probe-rs**<br />
`cargo embed --release`

## Usefull links:
https://jonathanklimt.de/electronics/programming/embedded-rust/rust-on-stm32-2/
https://github.com/stm32-rs/stm32wlxx-hal/blob/main/testsuite/README.md

MSVC toolchain doesnt work with windows:
https://asyncbulbs.blogspot.com/2017/06/workaround-for-rustc-with-new-visual.html

Setup probe-rs:
https://probe.rs/docs/getting-started/installation/
PS C:\Users\Kris Bittle\Documents\RBmesh> Set-ExecutionPolicy RemoteSigned -scope CurrentUser
PS C:\Users\Kris Bittle\Documents\RBmesh> irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex

https://probe.rs/docs/tools/cargo-flash/
My knock off probe works with this. Does not work with STM programmer software.
`probe-rs list` to list programmers
[0]: STLink V2 -- 0483:3748:48 (ST-LINK)

## Hardware:
3 led unit ID is: 5678875<br />
1 led unit ID is: 5677364<br />

AT+MSEND=5678875,true,1,hello

