mod chess_piece;
use chess_piece::{Piece, Color, PieceType};

fn main() {
    println!("Hello, world!");
    let piece = Piece {
        color: Color::White,
        piece_type: PieceType::Pawn,
    };
    println!("Piece: {:?}", piece);
}
