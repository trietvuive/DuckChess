mod repr;
use repr::bitboard::Bitboard;

fn main() {
    // Create a bitboard with a few pieces
    let mut bb = Bitboard::EMPTY;
    
    // Set up a few pieces (for example, pawns on the second rank)
    for file in 0..8 {
        bb.set_bit(8 + file); // Second rank (rank 1)
    }
    
    // Print the bitboard
    println!("Bitboard representation:");
    println!("{}", bb);
    
    // Print some statistics
    println!("Number of pieces: {}", bb.pop_count());
    
    // Example of bitwise operations
    let bb2 = Bitboard::from_square(0);
    let combined = bb | bb2;
    println!("\nCombined with a piece at a1:");
    println!("{}", combined);
}
