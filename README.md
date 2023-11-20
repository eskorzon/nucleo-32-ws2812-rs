# Control A WS2812B fully-addressable LED light string using a Nucleo-32
This repo contains code that can be compiled to a Nucleo-32 and allows it to control a light string using smart-leds and ws8212-spi-rs.

# Installation
1. Clone this repo: `git clone https://github.com/eskorzon/nucleo-32-rs.git`.
2. Cd into the repo: `cd nucleo-32-rs`.
3. Install probe-rs: `cargo install probe-rs --features cli`.
4. Install the target: `rustup target add thumbv7em-none-eabihf`.
5. Run blinky: `cargo run`.
