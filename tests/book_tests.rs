use duck_chess::OpeningBook;
use shakmaty::{fen::Fen, CastlingMode, Chess, Position};
use std::path::PathBuf;

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

fn test_fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(filename);
    path
}

#[test]
fn opening_book_default_is_empty() {
    let book = OpeningBook::default();
    assert!(book.is_empty());
}

#[test]
fn opening_book_loads_epd_fixture() {
    let book = OpeningBook::load_epd(&test_fixture_path("test_book.epd"));
    assert!(book.is_ok());
    let book = book.unwrap();
    assert!(!book.is_empty());
}

#[test]
fn opening_book_epd_contains_position_from_fixture() {
    let book = OpeningBook::load_epd(&test_fixture_path("test_book.epd")).unwrap();
    // Position from test_book.epd: 1.e4 e5 2.Nf3 (after 2 plies)
    let pos = from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3");
    assert!(
        book.contains(&pos),
        "Position from 1.e4 e5 2.Nf3 should be in test book"
    );
}

#[test]
fn opening_book_epd_probe_returns_none() {
    let book = OpeningBook::load_epd(&test_fixture_path("test_book.epd")).unwrap();
    let pos = Chess::default();
    // EPD books store positions without moves, so probe returns None
    let mv = book.probe(&pos);
    assert!(
        mv.is_none(),
        "EPD books don't store moves, probe returns None"
    );
}

#[test]
fn opening_book_auto_detects_epd_format_from_fixture() {
    let book = OpeningBook::load(&test_fixture_path("test_book.epd"));
    assert!(book.is_ok());
    let book = book.unwrap();
    assert!(!book.is_empty());
}

// PGN format tests using fixture

#[test]
fn opening_book_loads_pgn_fixture() {
    let book = OpeningBook::load_pgn(&test_fixture_path("test_book.pgn"));
    assert!(book.is_ok());
    let book = book.unwrap();
    assert!(!book.is_empty());
}

#[test]
fn opening_book_probe_startpos_returns_move_from_fixture() {
    let book = OpeningBook::load_pgn(&test_fixture_path("test_book.pgn")).unwrap();
    let pos = Chess::default();
    let mv = book.probe(&pos);
    assert!(
        mv.is_some(),
        "Book should have a move for the starting position"
    );
}

#[test]
fn opening_book_probe_returns_legal_move_from_fixture() {
    let book = OpeningBook::load_pgn(&test_fixture_path("test_book.pgn")).unwrap();
    let pos = Chess::default();
    if let Some(mv) = book.probe(&pos) {
        assert!(pos.is_legal(&mv), "Book move must be legal");
    }
}

// Optional: tests for large book files in opening_books/ directory
// These only run if you download the full book files locally (not in CI)
