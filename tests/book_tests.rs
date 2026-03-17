use duck_chess::OpeningBook;
use shakmaty::{fen::Fen, CastlingMode, Chess, Position};
use std::path::Path;

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn opening_book_default_is_empty() {
    let book = OpeningBook::default();
    assert!(book.is_empty());
    assert_eq!(book.len(), 0);
}

#[test]
fn opening_book_loads_pgn_file() {
    let book = OpeningBook::load_pgn(Path::new("opening_books/8moves_v3.pgn"));
    assert!(book.is_ok());
    let book = book.unwrap();
    assert!(!book.is_empty());
}

#[test]
fn opening_book_probe_startpos_returns_move() {
    let book = OpeningBook::load_pgn(Path::new("opening_books/8moves_v3.pgn")).unwrap();
    let pos = Chess::default();
    let mv = book.probe(&pos);
    assert!(
        mv.is_some(),
        "Book should have a move for the starting position"
    );
}

#[test]
fn opening_book_probe_returns_legal_move() {
    let book = OpeningBook::load_pgn(Path::new("opening_books/8moves_v3.pgn")).unwrap();
    let pos = Chess::default();
    if let Some(mv) = book.probe(&pos) {
        assert!(pos.is_legal(&mv), "Book move must be legal");
    }
}

#[test]
fn opening_book_probe_unknown_position_returns_none() {
    let book = OpeningBook::load_pgn(Path::new("opening_books/8moves_v3.pgn")).unwrap();
    // A position deep in a game, unlikely to be in the opening book
    let pos = from_fen("8/8/8/3k4/8/8/8/4K3 w - - 0 1");
    let mv = book.probe(&pos);
    assert!(
        mv.is_none(),
        "King vs king endgame should not be in opening book"
    );
}
