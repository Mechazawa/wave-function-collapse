# Wave Function Collapse

High-performance Rust implementation of the Wave Function Collapse algorithm for procedural image generation.

## What it does

Wave Function Collapse analyzes patterns in an input image and generates new images that locally resemble the original while creating novel, larger compositions. The algorithm extracts small tile patterns from the input, learns adjacency rules, and uses constraint propagation to generate coherent output.

## Usage

Generate an image from a pattern:
```sh
cargo run --release -- images/circuit-1-57x30.png -i 14 -o 50x50 output/circuit-1.png
```

Visual mode (real-time generation):
```sh
cargo run --release -- images/circuit-1-57x30.png -i 14 -o 50x50 --visual
```

## Feature Flags

- `image` (default): Enables image processing, serialization, and text rendering
- `sdl2` (default): Enables visual mode with real-time generation display
- `threaded`: Enables parallel processing for faster generation

Build with specific features:
```sh
cargo build --release --features "image,threaded"  # Fast generation without visual mode
cargo build --release --no-default-features --features "image"  # Minimal build
```

## Examples

### Circuit Generation
Input: `images/circuit-1-57x30.png`
```sh
cargo run --release -- images/circuit-1-57x30.png -i 14 -o 25x19 examples/circuit-example.png
```
Creates complex electronic circuit patterns with proper connectivity.

![Circuit Output](examples/circuit-example.png)

### Summer Landscape
Input: `images/summer-1-16x9.png`
```sh
cargo run --release -- images/summer-1-16x9.png -i 8 -o 64x36 examples/summer-example.png
```
Generates natural landscape variations with smooth transitions.

![Summer Output](examples/summer-example.png)

## Installation

Requires Rust and SDL2 development libraries.

```sh
cargo build --release
```

## License

CC0 (Public Domain)
