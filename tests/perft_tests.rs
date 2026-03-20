//! Move-generation regression: known perft counts (shakmaty rules). Not affected by eval changes.

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

struct Case {
    fen: &'static str,
    depth: u32,
    expect: u64,
}

#[test]
fn perft_regression_suite() {
    const CASES: &[Case] = &[
        Case {
            fen: "startpos",
            depth: 1,
            expect: 20,
        },
        Case {
            fen: "startpos",
            depth: 2,
            expect: 400,
        },
        Case {
            fen: "startpos",
            depth: 3,
            expect: 8902,
        },
        Case {
            fen: "startpos",
            depth: 4,
            expect: 197281,
        },
        Case {
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 1,
            expect: 48,
        },
        Case {
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 2,
            expect: 2039,
        },
        Case {
            fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            depth: 3,
            expect: 97862,
        },
        Case {
            fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            depth: 1,
            expect: 14,
        },
        Case {
            fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            depth: 2,
            expect: 191,
        },
        Case {
            fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            depth: 3,
            expect: 2812,
        },
        Case {
            fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            depth: 1,
            expect: 6,
        },
        Case {
            fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            depth: 2,
            expect: 264,
        },
        Case {
            fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            depth: 1,
            expect: 44,
        },
        Case {
            fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            depth: 2,
            expect: 1486,
        },
        Case {
            fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            depth: 1,
            expect: 46,
        },
        Case {
            fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            depth: 2,
            expect: 2079,
        },
    ];

    for c in CASES {
        let pos = if c.fen == "startpos" {
            Chess::default()
        } else {
            from_fen(c.fen)
        };
        assert_eq!(
            perft(&pos, c.depth),
            c.expect,
            "fen={} d={}",
            c.fen,
            c.depth
        );
    }
}
