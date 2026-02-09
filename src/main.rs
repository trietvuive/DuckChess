//! DuckChess - A UCI Chess Engine with NNUE Evaluation
//!
//! This is the main entry point for the chess engine.
//! Run without arguments to start the UCI interface.

use duck_chess::engine::MoveGen;
use duck_chess::uci::UCI;

fn main() {
    // Print engine info on startup
    eprintln!("DuckChess v1.0.0 - UCI Chess Engine with NNUE");
    eprintln!("Type 'uci' to start UCI mode, 'd' to display board, 'quit' to exit");
    
    // Initialize move generation tables (done lazily, but can be forced here)
    // This ensures responsive behavior when the GUI first sends commands
    let _ = MoveGen::instance();
    
    // Start UCI loop
    let mut uci = UCI::new();
    uci.run();
}
