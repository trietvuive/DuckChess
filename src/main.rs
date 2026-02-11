//! DuckChess UCI Chess Engine

use duck_chess::uci::UCI;

fn main() {
    println!("DuckChess v1.0.0 - UCI Chess Engine");
    println!("Type 'uci' to start UCI mode, 'd' to display board, 'quit' to exit");

    let mut uci = UCI::new();
    uci.run();
}
