# Evaluation

Static position scores for search (centipawns, side to move).

NNUE (default): `(768 → 256) × 2 → 1` SCReLU network with SIMD acceleration.
  Trained with [bullet](https://github.com/jw1912/bullet) on Stockfish binpack data.
  Quantized weights are embedded via `include_bytes!("nnue/net.bin")`.
Material: Classical piece values, available as a fallback via UCI option.
Common code applies tempo and other shared finalization; also detects insufficient-material draws.
UCI option `Eval` selects the backend (`NNUE` default, `Material` fallback).
The crate `evaluate()` helper uses NNUE with the same leaf rules as UCI.