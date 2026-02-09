# DuckChess

A UCI chess engine written in Rust from scratch, featuring NNUE (Efficiently Updatable Neural Network) evaluation.

## Features

### Board Representation
- **Bitboards**: 64-bit integers for efficient piece representation
- **Magic Bitboards**: Fast sliding piece move generation
- **Zobrist Hashing**: Efficient position identification

### Move Generation
- Legal move generation with pinned piece handling
- Support for all special moves:
  - Castling (kingside and queenside)
  - En passant
  - Pawn promotion
- Perft-verified correctness

### NNUE Evaluation
- HalfKP-like feature set (768 features per perspective)
- Two hidden layers (256 → 32 → 1)
- ClippedReLU activation
- Incremental accumulator updates
- Fallback piece-square table evaluation

### Search
- Iterative deepening
- Alpha-beta pruning with fail-soft
- Principal Variation Search (PVS)
- Transposition table with Zobrist hashing
- Move ordering:
  - TT move priority
  - MVV-LVA (Most Valuable Victim - Least Valuable Attacker)
  - Killer moves
  - History heuristic
- Pruning and reductions:
  - Null move pruning
  - Late Move Reductions (LMR)
  - Mate distance pruning
  - Check extension
- Quiescence search with delta pruning
- Aspiration windows

### UCI Protocol
Full UCI protocol support including:
- `uci`, `isready`, `ucinewgame`
- `position startpos/fen [moves ...]`
- `go depth/nodes/movetime/wtime/btime/winc/binc/movestogo/infinite`
- `stop`, `quit`
- Hash size option

Non-standard but useful commands:
- `d` - Display the current board
- `perft <depth>` - Run perft test
- `eval` - Show position evaluation

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## Usage

Run the engine:
```bash
./target/release/duck_chess
```

Example UCI session:
```
uci
id name DuckChess 1.0.0
id author DuckChess Team

option name Hash type spin default 64 min 1 max 4096
option name Threads type spin default 1 min 1 max 1
uciok

isready
readyok

position startpos
go depth 8
info depth 1 score cp 10 nodes 22 nps 22000 time 1 hashfull 0 pv e2e4
info depth 2 score cp 0 nodes 145 nps 145000 time 1 hashfull 0 pv e2e4
...
bestmove e2e4

quit
```

## Project Structure

```
src/
├── lib.rs              # Library root
├── main.rs             # Entry point
├── core/               # Core chess types
│   ├── mod.rs          # Module exports
│   ├── bitboard.rs     # Bitboard representation
│   ├── board.rs        # Board/position representation
│   ├── moves.rs        # Move encoding
│   └── zobrist.rs      # Zobrist hashing
├── engine/             # Engine components
│   ├── mod.rs          # Module exports
│   ├── movegen.rs      # Move generation with magic bitboards
│   ├── nnue.rs         # NNUE neural network evaluation
│   ├── search.rs       # Alpha-beta search engine
│   └── tt.rs           # Transposition table
└── uci/                # UCI protocol
    ├── mod.rs          # Module exports
    └── protocol.rs     # UCI protocol handler

tests/
├── perft_tests.rs      # Move generation verification (28 tests)
├── search_tests.rs     # Search and evaluation tests (16 tests)
└── uci_tests.rs        # UCI protocol and FEN tests (24 tests)
```

## Testing

The engine includes comprehensive unit and integration tests:

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test perft_tests
cargo test --test search_tests
cargo test --test uci_tests
```

### Test Summary
- **62** unit tests (in library modules)
- **28** perft tests (move generation verification)
- **16** search tests (tactics, evaluation, time management)
- **24** UCI tests (FEN parsing, board state, special positions)
- **130** total tests

### Perft Results
```
Position            | Depth | Nodes
--------------------|-------|----------
Starting Position   | 5     | 4,865,609
Kiwipete            | 4     | 4,085,603
Position 3          | 4     | 43,238
Position 4          | 3     | 9,467
Position 5          | 3     | 62,603
Position 6          | 3     | 89,890
```

## Performance

On a typical modern CPU:
- ~40M nodes/second for perft in release mode
- ~700k nodes/second for search with evaluation

## Architecture Notes

### NNUE Network
The NNUE network uses a simplified HalfKP feature set:
- **Input Layer**: 768 features (64 squares × 12 piece types)
- **Hidden Layer 1**: 256 neurons with ClippedReLU
- **Hidden Layer 2**: 32 neurons with ClippedReLU
- **Output**: Single evaluation score

The network weights are initialized with piece-square table heuristics. For maximum strength, you would train the network on millions of positions from engine self-play.

### Magic Bitboards
The engine uses pre-computed magic numbers for efficient sliding piece (rook/bishop) attack generation. Magic bitboards achieve O(1) lookup time for attack sets.

## License

MIT License
