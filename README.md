# RB Mesh

<!-- Build Status Badge -->
![Build](https://github.com/kbittle/RBmesh/actions/workflows/build.yml/badge.svg)

<!-- Test Status Badge -->
![Test](https://github.com/kbittle/RBmesh/actions/workflows/test.yml/badge.svg)

A mesh stack written in rust to run on the STM32WL / LoRa-E5 module. Like the LoRaWAN modem software Seed Studio provides with the hardware. This software package will also support a AT command set to interact with the modem. This mesh stack is self forming and self healing. If nodes are mobile, a link is formed and then broken, the mesh is smart enough record a failure and find a better route. Simply point your payload to a node address and all decisions happen under the hood. This software package will also supply GPIO support for radio TX/RX and incoming message ring indication.

[LoRa_E5_Package](lora_e5_package/README.md) - Main project source.

[Rb_Mesh_lib](rb_mesh_lib/README.md) - Sub project crate for all mesh related logic.

I used custom PCB for this development but the "Wio-E5 mini Dev Board" should work just fine.<br />
![Alt text](resources/3d_render.png?raw=true "Custom PCB")

## Todo's / Issue's:
- Add network discovery retries.
- Improve route managment. Currently store 5 and always delete the oldest.
- Test with more nodes. Have only tested with 2 nodes.
- Add periodic neighbor table transmits. Might want to TX subset/only 3 entries. Maybe one every 10min?
- Add some sort of randomized delay for transmitting. What happens when 3 nodes want to relay a packet? Adding a delay will help stagger packets and theoretically the other nodes will detect TX preambles and block.

## Environment Setup:

### Setup in WSL:

If you’re a Windows Subsystem for Linux user run the following in your terminal, then follow the on-screen instructions to install Rust.

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

`. "$HOME/.cargo/env"`

Download the target with:

`rustup target add thumbv7m-none-eabi`

## Build/Run/Test Commands:
General rust/cargo command to initialize project space:<br />
`cargo init RBmesh`

General rust/cargo command to build application:<br />
`cargo build --release`

Command to load code on platform: **requires probe-rs**<br />
`cargo flash --release --chip STM32WLE5JC`

Command to load and debug code on platform: **requires probe-rs**<br />
`cargo embed --release`

## Unit Test:

Command to call test code that runs on build machine. This will exercise the network stack code.

`cargo test -p bm_network --target x86_64-unknown-linux-gnu`

You cannot just call `cargo test`. I think it has to do with the target being ARM and not supporting std lib.
https://github.com/rust-lang/cargo/issues/6784

Command to print verbose println's while running test:

`cargo test -p bm_network --target x86_64-unknown-linux-gnu -- --nocapture`

## Usefull links:
https://jonathanklimt.de/electronics/programming/embedded-rust/rust-on-stm32-2/
https://github.com/stm32-rs/stm32wlxx-hal/blob/main/testsuite/README.md

MSVC toolchain doesnt work with windows:<br />
https://asyncbulbs.blogspot.com/2017/06/workaround-for-rustc-with-new-visual.html

Setup probe-rs:<br />
https://probe.rs/docs/getting-started/installation/<br />
PS ...\Documents\RBmesh> `Set-ExecutionPolicy RemoteSigned -scope CurrentUser`<br />
PS ...\Documents\RBmesh> `irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex`<br />

https://probe.rs/docs/tools/cargo-flash/
My knock off probe works with this. Does not work with STM programmer software.
`probe-rs list` to list programmers
[0]: STLink V2 -- 0483:3748:48 (ST-LINK)

## Hardware:
3 led unit ID is: 5678875<br />
1 led unit ID is: 5677364<br />

AT+MSEND=5678875,true,1,hello

