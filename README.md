# Wave-Function-Collapse
Implimentation of the Wave Function Collapse algorithm with an emphasis on performance. 

## Note 

This project is currently a work in progress. The eventual goal is for this to be both a re-usable library and application.

## Usage

Generating an image on the command line
```sh
cargo run --release -- images/circuit-1-57x30.png -i 14 -o 50x50 output/circuit-1.png
```

Visualising the process but not storing an output image
```sh
cargo run --release -- images/circuit-1-57x30.png -i 14 -o 50x50 --visual
```
