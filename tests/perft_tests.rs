use shakmaty::{fen::Fen, CastlingMode, Chess, Position};

fn perft(pos: &Chess, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = pos.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    moves
        .iter()
        .map(|mv| perft(&pos.clone().play(mv).unwrap(), depth - 1))
        .sum()
}

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn test_perft_startpos_depth_1() {
    assert_eq!(perft(&Chess::default(), 1), 20);
}

#[test]
fn test_perft_startpos_depth_2() {
    assert_eq!(perft(&Chess::default(), 2), 400);
}

#[test]
fn test_perft_startpos_depth_3() {
    assert_eq!(perft(&Chess::default(), 3), 8902);
}

#[test]
fn test_perft_startpos_depth_4() {
    assert_eq!(perft(&Chess::default(), 4), 197281);
}

#[test]
fn test_perft_startpos_depth_5() {
    assert_eq!(perft(&Chess::default(), 5), 4865609);
}

#[test]
fn test_perft_kiwipete_depth_1() {
    let pos = from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(perft(&pos, 1), 48);
}

#[test]
fn test_perft_kiwipete_depth_2() {
    let pos = from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(perft(&pos, 2), 2039);
}

#[test]
fn test_perft_kiwipete_depth_3() {
    let pos = from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(perft(&pos, 3), 97862);
}

#[test]
fn test_perft_kiwipete_depth_4() {
    let pos = from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(perft(&pos, 4), 4085603);
}

#[test]
fn test_perft_position3_depth_1() {
    let pos = from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
    assert_eq!(perft(&pos, 1), 14);
}

#[test]
fn test_perft_position3_depth_2() {
    let pos = from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
    assert_eq!(perft(&pos, 2), 191);
}

#[test]
fn test_perft_position3_depth_3() {
    let pos = from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
    assert_eq!(perft(&pos, 3), 2812);
}

#[test]
fn test_perft_position3_depth_4() {
    let pos = from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
    assert_eq!(perft(&pos, 4), 43238);
}

#[test]
fn test_perft_position4_depth_1() {
    let pos = from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
    assert_eq!(perft(&pos, 1), 6);
}

#[test]
fn test_perft_position4_depth_2() {
    let pos = from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
    assert_eq!(perft(&pos, 2), 264);
}

#[test]
fn test_perft_position4_depth_3() {
    let pos = from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
    assert_eq!(perft(&pos, 3), 9467);
}

#[test]
fn test_perft_position5_depth_1() {
    let pos = from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
    assert_eq!(perft(&pos, 1), 44);
}

#[test]
fn test_perft_position5_depth_2() {
    let pos = from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
    assert_eq!(perft(&pos, 2), 1486);
}

#[test]
fn test_perft_position5_depth_3() {
    let pos = from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
    assert_eq!(perft(&pos, 3), 62379);
}

#[test]
fn test_perft_position6_depth_1() {
    let pos = from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");
    assert_eq!(perft(&pos, 1), 46);
}

#[test]
fn test_perft_position6_depth_2() {
    let pos = from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");
    assert_eq!(perft(&pos, 2), 2079);
}

#[test]
fn test_perft_position6_depth_3() {
    let pos = from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10");
    assert_eq!(perft(&pos, 3), 89890);
}
