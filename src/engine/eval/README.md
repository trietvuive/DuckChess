# Evaluation

Static position scores for search (centipawns, side to move).

- Evaluator picks material or NNUE, then runs shared leaf rules on the raw score.
- Material uses classical piece values; NNUE loads weights from nnue.bin.
- Common code applies tempo and other shared finalization; also has insufficient-material draws.
- UCI option Eval selects the backend; Searcher uses an evaluator at leaves and in quiescence.
- The crate evaluate() helper is material-only with the same rules as the default UCI eval.
