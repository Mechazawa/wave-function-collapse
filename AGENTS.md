# Working in this repo

## Layout

Three packages in one workspace.

- `crates/wave-function-collapse` is the algorithm and the published crate. It carries
  six dependencies and knows nothing about images, files or terminals. Keep it that
  way: a tile's value is a generic the algorithm never reads.
- `crates/wfc-cli` is the `wfc` binary. Everything that knows what a tile looks like
  belongs here, including cutting a sample image into tiles, reading a tile config,
  composing the result, and the SDL2 view. Because an inherent impl only compiles in
  the crate defining the type, additions to `Tile` or `Wave` that need `image` are
  free functions in `tiles` or `compose`, not methods.
- `crates/wfc-wasm` is the solver for a JavaScript host. It takes tile indices and
  weights and returns grid state. It must never depend on `image`, `web-sys` or
  anything else the CLI uses; `crates/wfc-wasm/www` is a demo that draws its own tiles.

## Commands

```sh
cargo test -p wave-function-collapse            # the algorithm
cargo test -p wave-function-collapse --features threaded
cargo test -p wfc-cli                           # needs SDL2
cargo test -p wfc-cli --no-default-features     # without the live view
cargo clippy --workspace --all-targets           # CI runs this with -Dwarnings
cargo fmt --all -- --check
```

The wasm package is not part of `--workspace` builds for the host in any useful sense.
Build it for its own target:

```sh
cargo clippy -p wfc-wasm --target wasm32-unknown-unknown
cd crates/wfc-wasm && wasm-pack build --target web --out-dir www/pkg
cd crates/wfc-wasm/www && npm install && npm run dev    # 127.0.0.1:5173
```

Never `cargo build --workspace --target wasm32-unknown-unknown`. That builds the CLI
for wasm too, which cannot work.

## What CI enforces beyond the usual

The library must not pick up CLI dependencies, and the wasm solver must not pick up
image handling or OS randomness. Both are dependency-tree greps in
`.github/workflows/rust.yml`. `rand` is deliberately `default-features = false` in the
library: its `os_rng` default pulls in `getrandom`, which refuses to compile for
`wasm32-unknown-unknown`. Every seed comes from the caller.

## Things that are easy to get wrong

**A sample image's tile size is not in the code.** It is encoded in the file name: the
grid, not the pixels. `castle-115x30.png` is 805 by 210, so `-i 7`.
`circles-24x10.png` is 768 by 320, so `-i 32`. Passing the wrong size produces a tile
set that cannot tile anything, and the tool now reports that rather than hanging.

**Adjacency has to read the same from both sides.** `neighbors[Up]` means the tiles
allowed *above* this one. If tile A allows B above it, B has to allow A below it, or
the set cannot be satisfied and the solver will restart forever. A tile set built
from edge labels satisfies this by construction; one written by hand does not.

**Edge labels pair by reversal, not equality.** Two tiles may touch when the label on
the side that touches reads as the reverse of the other's, so `"ab"` meets `"ba"` and
a palindrome like `"aa"` meets itself.

**A wave that reports `done` has been checked.** `tick_cell` re-checks cells that
have already collapsed, which is what catches a neighbour collapsing to something
they forbid. Do not reintroduce an early return for `entropy == 1`, and keep the
collapsed-cell count to one push per cell or `remaining()` goes wrong.

**A tile set that cannot tile the grid restarts forever.** That is why `Wave::run`
takes a restart budget and `Wave::restarts` is public. A caller that cannot wait has
to make the give-up decision itself; the library will not make it for them.
