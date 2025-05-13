#[derive(Debug)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Debug)]
pub struct Piece {
    pub color: Color,
    pub piece_type: PieceType,
}