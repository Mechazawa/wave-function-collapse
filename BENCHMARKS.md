# Performance Benchmarks Guide

This document provides comprehensive guidance on running, analyzing, and creating benchmarks for the Wave Function Collapse implementation.

## Overview

The project uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical benchmarking, providing detailed performance analysis with statistical confidence intervals, regression detection, and HTML reports.

## Running Benchmarks

### Prerequisites

Ensure you have the required dependencies installed:
```bash
cargo build --release --all-features
```

### Basic Benchmark Execution

Run all benchmarks with default configuration:
```bash
cargo bench
```

Run specific benchmark groups:
```bash
cargo bench maybe_collapse
cargo bench superstate_tick
cargo bench grid_operations
```

Run benchmarks for specific input sizes:
```bash
cargo bench -- "size_25"
cargo bench -- "medium"
```

## Benchmark Structure

### Current Benchmark Coverage

1. **`maybe_collapse`** - Tests the core collapse selection algorithm
   - Grid sizes: 10×10, 20×20, 50×50
   - Measures: Time to find and collapse lowest-entropy cell
   - Key metric: Microseconds per collapse operation

2. **`superstate_tick`** - Tests constraint propagation performance
   - Scenarios: Many possibilities, few constraints
   - Measures: Time to filter invalid tile combinations
   - Key metric: Nanoseconds per constraint check

3. **`superstate_collapse`** - Tests weighted random selection
   - Scenarios: Weighted tile selection with RNG
   - Measures: Time to perform weighted random selection
   - Key metric: Nanoseconds per collapse with fixed seed

4. **`grid_operations`** - Tests basic grid functionality
   - Operations: Creation, neighbor access
   - Grid sizes: 25×25, 50×50, 100×100
   - Key metric: Operations per second

5. **`tile_from_image`** *(feature: image)* - Tests tile extraction from images
   - Image sizes: 64px, 128px, 256px with various tile sizes
   - Measures: Time to extract and process tiles
   - Key metric: Milliseconds per tile extraction

6. **`wave_tick`** - Tests overall algorithm performance
   - Grid sizes: 15×15, 25×25, 35×35
   - Scenarios: Single tick on partially collapsed wave
   - Key metric: Microseconds per wave state update

## Analyzing Results

### Understanding Criterion Output

```
maybe_collapse/size_25   time:   [45.2 µs 46.1 µs 47.3 µs]
                         change: [-2.1% +0.5% +3.4%] (p = 0.19 > 0.05)
                         No change in performance detected.
```

- **Time range**: [lower_bound estimate upper_bound] with 95% confidence
- **Change**: Percentage change from previous run
- **P-value**: Statistical significance (p < 0.05 indicates significant change)
- **Status**: Performance regression/improvement detection

### HTML Report Analysis

Criterion generates detailed HTML reports in `target/criterion/`:

- **Summary**: Overview of all benchmarks
- **Individual Reports**: Detailed analysis per benchmark
- **Violin Plots**: Distribution of measurement samples
- **Line Charts**: Performance over time
- **Comparison**: Before/after performance comparison

## Performance Debugging

For investigating performance issues:

1. **Profile with perf**:
   ```bash
   # Build benchmark binary first
   cargo build --release --benches
   # Find the benchmark binary path
   find target/release/deps -name "wfc_benchmarks-*" -executable
   # Profile with perf (replace with actual path)
   perf record -g target/release/deps/wfc_benchmarks-<hash> maybe_collapse
   perf report
   ```

2. **Use flamegraphs** (requires `cargo install flamegraph`):
   ```bash
   cargo flamegraph --bench wfc_benchmarks -- maybe_collapse
   ```

3. **Debug specific functions**:
   ```bash
   cargo bench maybe_collapse  # Run specific benchmark group
   cargo bench -- "size_25"   # Filter by specific test
   ```
