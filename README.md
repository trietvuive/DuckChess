# DuckChess

A WIP UCI chess engine written in Rust.

## Opening book

The engine supports an opening book (PGN). Set the path with the UCI option **BookPath** and enable **OwnBook** (default: true). When the current position is in the book, the engine plays a random book move. Example: `opening_books/8moves_v3.pgn` (from [Stockfish books](https://github.com/official-stockfish/books)).