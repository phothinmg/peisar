# Benchmark results

Peisar includes a repeatable Node.js benchmark that compares end-to-end
Markdown-to-HTML rendering against:

- `marked`
- `markdown-it`
- `micromark` with `micromark-extension-gfm`
- `showdown`

The benchmark implementation lives in [bench/src/index.js](../bench/src/index.js).
Running it also regenerates [benchmark-results.svg](./benchmark-results.svg)
from the measured throughput and heap results.

## What the benchmark measures

- output validation before timing
- throughput in operations per second
- peak heap usage during repeated renders
- three document sizes built from the same representative GFM input

The current benchmark suite uses:

- Tinybench for timing
- `node --expose-gc` for more stable memory measurements
- Peisar 1.2.3 on Node.js 24.22.1 for the results below
- `micromark` configured with its GFM syntax and HTML extensions
- seven memory batches, with 100 / 25 / 10 render iterations per batch for
  base / medium / large documents
- one base document plus `medium-x8` and `large-x32` expanded inputs

## Throughput

Average throughput in operations per second (higher is better):

| Scenario | peisar | marked | markdown-it | showdown | micromark |
| --- | ---: | ---: | ---: | ---: | ---: |
| Base (528 B) | 20,588 | 9,653 | 7,886 | 2,155 | 901 |
| Medium (4,542 B) | 2,625 | 1,329 | 1,001 | 345 | 111 |
| Large (18,220 B) | 723 | 358 | 261 | 80 | 25 |

## Average peak heap usage

Measured average peak heap in MiB (lower is better):

| Scenario | peisar | marked | markdown-it | showdown | micromark |
| --- | ---: | ---: | ---: | ---: | ---: |
| Base | 0.14 | 4.71 | 6.07 | 20.25 | 58.62 |
| Medium | 0.23 | 9.54 | 13.18 | 50.79 | 62.88 |
| Large | 0.35 | 15.29 | 21.01 | 63.88 | 86.15 |

## Summary

In the benchmark currently documented in this repository, Peisar had the
highest measured throughput and lowest measured average peak heap usage across
the tested scenarios, including micromark configured with GFM.

## Notes

- Results reflect the workload and configuration in
  [bench/src/index.js](../bench/src/index.js).
- Memory profiling uses seven batches with 100 / 25 / 10 iterations per batch
  for base / medium / large inputs; heap values are approximate and sensitive
  to the runtime and process state.
- Benchmark results vary with runtime, hardware, input characteristics, and
  parser configuration.
- Because this file lives in the repository rather than the published package
  metadata alone, it can be updated between releases.

## Run the benchmark

From the repository root:

```sh
npm run benchmark
```

Or directly inside [bench/](../bench):

```sh
cd bench
npm i
npm run bench
```
