# Search

Alpha-beta negamax with iterative deepening.

- Iterative Deepening: Search depth 1, then 2, then 3... until time runs out
- Alpha-Beta: Prune branches that cannot affect the final decision
- Transposition Table**: Cache search results by position hash
- Quiescence: At leaf nodes, continue searching captures only
- Move Ordering: Try best moves first (TT move, captures, killers) to maximize alpha-beta cutoffs