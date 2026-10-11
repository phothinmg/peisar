# Benchmark results

Peisar includes a repeatable Node.js benchmark that compares end-to-end
Markdown-to-HTML rendering against:

- `marked`
- `markdown-it`
- `showdown`

The benchmark implementation lives in [bench/src/index.js](../bench/src/index.js).

## What the benchmark measures

- output validation before timing
- throughput in operations per second
- peak heap usage during repeated renders
- three document sizes built from the same representative GFM input

The current benchmark suite uses:

- Tinybench for timing
- `node --expose-gc` for more stable memory measurements
- one base document plus `medium-x8` and `large-x32` expanded inputs

## Throughput

| Scenario | peisar | marked | markdown-it | showdown |
| --- | ---: | ---: | ---: | ---: |
| Base (528 B) | 21,041 ops/s | 8,692 ops/s | 7,800 ops/s | 2,192 ops/s |
| Medium (4,542 B) | 2,661 ops/s | 1,295 ops/s | 1,037 ops/s | 347 ops/s |
| Large (18,220 B) | 664 ops/s | 350 ops/s | 264 ops/s | 82 ops/s |

## Peak heap usage

| Scenario | peisar | marked | markdown-it | showdown |
| --- | ---: | ---: | ---: | ---: |
| Base | 4.68 MiB | 9.95 MiB | 8.15 MiB | 16.06 MiB |
| Medium | 6.70 MiB | 31.59 MiB | 26.90 MiB | 29.71 MiB |
| Large | 5.35 MiB | 51.65 MiB | 61.22 MiB | 57.54 MiB |

## Summary

In the benchmark currently documented in this repository, Peisar had the
highest measured throughput and the lowest measured peak heap usage across the
tested scenarios.

## Notes

- Results reflect the workload and configuration in
  [bench/src/index.js](../bench/src/index.js).
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
