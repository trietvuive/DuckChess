//! Opening book: load fixtures and return legal moves when probing.

use duck_chess::OpeningBook;
use shakmaty::{fen::Fen, CastlingMode, Chess, Position};
use std::path::PathBuf;

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn default_book_is_empty() {
    assert!(OpeningBook::default().is_empty());
}

#[test]
fn load_epd_fixture() {
    let book = OpeningBook::load_epd(&fixture("test_book.epd")).expect("load epd");
    assert!(!book.is_empty());
    let pos = from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3");
    assert!(book.contains(&pos));
}

#[test]
fn load_pgn_fixture_probe_legal() {
    let book = OpeningBook::load_pgn(&fixture("test_book.pgn")).expect("load pgn");
    assert!(!book.is_empty());
    let pos = Chess::default();
    let mv = book.probe(&pos).expect("book move");
    assert!(pos.is_legal(&mv));
}

#[test]
fn load_auto_detects_epd() {
    let book = OpeningBook::load(&fixture("test_book.epd")).expect("load");
    assert!(!book.is_empty());
}
