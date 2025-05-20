use shakmaty::{Chess, Position, Move, Role, Square};
use std::io::{self, BufRead};
use rand::seq::SliceRandom;

fn main() {
    let stdin = io::stdin();
    let mut position = Chess::default();
    let mut rng = rand::thread_rng();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();

        match parts[0] {
            "uci" => {
                println!("id name RandomChess");
                println!("id author DuckChess");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "position" => {
                // Reset position
                position = Chess::default();
                
                // If there are moves after "position", apply them
                if parts.len() > 2 && parts[1] == "startpos" {
                    if parts.len() > 3 && parts[2] == "moves" {
                        for move_str in &parts[3..] {
                            if let Ok(chess_move) = parse_move(move_str, &position) {
                                position.play_unchecked(&chess_move);
                            }
                        }
                    }
                }
            }
            "go" => {
                // Generate all legal moves
                let legal_moves = position.legal_moves();
                
                // Choose a random move
                if let Some(random_move) = legal_moves.choose(&mut rng) {
                    println!("bestmove {}", format_move(random_move));
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn parse_move(move_str: &str, position: &Chess) -> Result<Move, String> {
    if move_str.len() != 4 && move_str.len() != 5 {
        return Err("Invalid move format".to_string());
    }

    let from = Square::from_ascii(move_str[0..2].as_bytes())
        .map_err(|_| "Invalid from square".to_string())?;
    let to = Square::from_ascii(move_str[2..4].as_bytes())
        .map_err(|_| "Invalid to square".to_string())?;

    let mut chess_move = Move::Normal {
        role: position.board().piece_at(from).map(|p| p.role).unwrap_or(Role::Pawn),
        from,
        to,
        capture: position.board().piece_at(to).map(|p| p.role),
        promotion: None,
    };

    // Handle promotion
    if move_str.len() == 5 {
        let promotion = match move_str.chars().nth(4).unwrap() {
            'q' => Some(Role::Queen),
            'r' => Some(Role::Rook),
            'b' => Some(Role::Bishop),
            'n' => Some(Role::Knight),
            _ => return Err("Invalid promotion piece".to_string()),
        };
        chess_move = Move::Normal {
            role: position.board().piece_at(from).map(|p| p.role).unwrap_or(Role::Pawn),
            from,
            to,
            capture: position.board().piece_at(to).map(|p| p.role),
            promotion,
        };
    }

    Ok(chess_move)
}

fn format_move(chess_move: &Move) -> String {
    match chess_move {
        Move::Normal { from, to, promotion, .. } => {
            let mut result = format!("{}{}", from, to);
            if let Some(role) = promotion {
                result.push(match role {
                    Role::Queen => 'q',
                    Role::Rook => 'r',
                    Role::Bishop => 'b',
                    Role::Knight => 'n',
                    _ => 'q',
                });
            }
            result
        }
        Move::EnPassant { from, to } => format!("{}{}", from, to),
        Move::Castle { king, rook } => {
            if rook.file() > king.file() {
                "e1g1" // Kingside castle
            } else {
                "e1c1" // Queenside castle
            }.to_string()
        }
        Move::Put { role, to } => {
            let piece = match role {
                Role::Queen => 'q',
                Role::Rook => 'r',
                Role::Bishop => 'b',
                Role::Knight => 'n',
                Role::Pawn => 'p',
                _ => 'q',
            };
            format!("{}@{}", piece, to)
        }
    }
}