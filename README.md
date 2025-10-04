<img align="left" style="height: 17ch; margin: 1ch" src="docs/logo.png#gh-dark-mode-only"><img align="left" style="height: 17ch; margin: 1ch" src="docs/logo-alt.png#gh-light-mode-only">

# Sh4derJockey
*A tool for shader coding and live performances*

<br clear="left"/>

Sh4derJockey lets you design custom render pipelines in yaml files,
supports as many fragment, vertex, and compute shader stages as your machine can handle,
provides 20+ audio textures and uniforms for audio-reactive effects,
offers plenty of buttons and sliders which can be hooked up to midi controllers,
allows for quick prototyping and live shader coding with automatic pipeline reloading,
includes support for live NDI® video input,
supports Spout texture sharing for Windows (with SpoutLibrary.dll),
and so much more!

## Documentation

The documentation on how to use this tool can be found in the [docs](docs/) folder or using the links below:

[Read in English](docs/readme_en.md) | [日本語で読む](docs/readme_jp.md)

## Setup

To build this project from source, you will need a Rust compiler and the Cargo package manager.
We highly recommend installing `rustup` which takes care of installing and updating the entire Rust toolchain.

Checkout the [Getting Started](https://www.rust-lang.org/learn/get-started) section on the rust-lang website for more.

```sh
# clone the repo
git clone https://github.com/slerpyyy/sh4der-jockey.git
cd sh4der-jockey

# build and run
cargo run

# install so you can run it from anywhere
cargo install --path .
```

## License

This project is licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

This program makes use of [NDI®](https://www.ndi.tv/) (Network Device Interface), a standard developed by [NewTek, Inc](https://www.newtek.com/).

This program supports [Spout](https://leadedge.github.io/) for texture sharing on Windows. Spout requires SpoutLibrary.dll - see [SPOUT_SETUP.md](SPOUT_SETUP.md) for installation instructions.

Please refer to https://www.ndi.tv/ for NDI and https://leadedge.github.io/ for Spout for further information about these technologies.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
