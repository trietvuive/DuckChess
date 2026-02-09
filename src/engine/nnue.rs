//! NNUE (Efficiently Updatable Neural Network) Evaluation
//!
//! This implements a simple NNUE architecture for chess evaluation.
//! The network uses a HalfKP feature set where features are indexed by:
//! (king_square, piece_square, piece_type, piece_color)
//!
//! Architecture:
//! - Input: 768 features per perspective (64 king squares * 12 piece types)
//! - Hidden Layer 1: 256 neurons (ClippedReLU activation)
//! - Hidden Layer 2: 32 neurons (ClippedReLU activation)
//! - Output: 1 neuron (evaluation score)

use crate::core::board::{Board, Color, Piece, PieceType, Square};

/// Number of input features per side (simplified HalfKP)
/// 64 squares * 10 piece types (excluding kings) = 640 per perspective
/// We use a simpler 768-feature set: 64 squares * 12 pieces
pub const INPUT_SIZE: usize = 768;

/// Hidden layer 1 size
pub const HIDDEN1_SIZE: usize = 256;

/// Hidden layer 2 size  
pub const HIDDEN2_SIZE: usize = 32;

/// Output size
pub const OUTPUT_SIZE: usize = 1;

/// Scale factor for quantization
pub const WEIGHT_SCALE: i32 = 64;
pub const ACTIVATION_SCALE: i32 = 127;

/// NNUE network weights and biases
pub struct NNUENetwork {
    /// Input -> Hidden1 weights [INPUT_SIZE][HIDDEN1_SIZE]
    pub input_weights: Vec<Vec<i16>>,
    /// Hidden1 biases [HIDDEN1_SIZE]
    pub hidden1_biases: Vec<i16>,
    /// Hidden1 -> Hidden2 weights [HIDDEN1_SIZE * 2][HIDDEN2_SIZE] (both perspectives)
    pub hidden2_weights: Vec<Vec<i16>>,
    /// Hidden2 biases [HIDDEN2_SIZE]
    pub hidden2_biases: Vec<i16>,
    /// Hidden2 -> Output weights [HIDDEN2_SIZE]
    pub output_weights: Vec<i16>,
    /// Output bias
    pub output_bias: i16,
}

impl NNUENetwork {
    /// Create a new NNUE network with randomized initial weights
    /// In a real engine, these would be loaded from a trained network file
    pub fn new() -> Self {
        // Initialize with simple piece-square table inspired weights
        let mut input_weights = vec![vec![0i16; HIDDEN1_SIZE]; INPUT_SIZE];
        let hidden1_biases = vec![0i16; HIDDEN1_SIZE];
        let mut hidden2_weights = vec![vec![0i16; HIDDEN2_SIZE]; HIDDEN1_SIZE * 2];
        let hidden2_biases = vec![0i16; HIDDEN2_SIZE];
        let mut output_weights = vec![0i16; HIDDEN2_SIZE];
        let output_bias = 0i16;

        // Initialize input weights based on simple piece values and centrality
        // This gives the engine basic understanding without training
        let piece_values = [100, 320, 330, 500, 900, 0]; // P, N, B, R, Q, K
        
        for sq in 0..64 {
            let file = sq % 8;
            let rank = sq / 8;
            let center_dist = ((3.5 - file as f32).abs() + (3.5 - rank as f32).abs()) as i16;
            
            for piece in 0..12 {
                let piece_type = piece % 6;
                let color = piece / 6;
                let feature_idx = sq * 12 + piece;
                
                let base_value = piece_values[piece_type] as i16;
                let sign = if color == 0 { 1 } else { -1 };
                
                // Distribute piece value across hidden neurons with some variation
                for h in 0..HIDDEN1_SIZE {
                    let variation = ((h as i32 * 17 + feature_idx as i32 * 31) % 21) as i16 - 10;
                    input_weights[feature_idx][h] = (sign * (base_value / 4 + variation - center_dist)) as i16;
                }
            }
        }

        // Initialize hidden2 weights
        for h1 in 0..(HIDDEN1_SIZE * 2) {
            for h2 in 0..HIDDEN2_SIZE {
                hidden2_weights[h1][h2] = (((h1 * 7 + h2 * 13) % 31) as i16 - 15) * 2;
            }
        }

        // Initialize output weights
        for h2 in 0..HIDDEN2_SIZE {
            output_weights[h2] = ((h2 as i16 % 3) - 1) * 16 + 8;
        }

        NNUENetwork {
            input_weights,
            hidden1_biases,
            hidden2_weights,
            hidden2_biases,
            output_weights,
            output_bias,
        }
    }

    /// Get feature index for a piece on a square from a perspective
    #[inline]
    pub fn feature_index(piece: Piece, sq: Square, perspective: Color) -> usize {
        let piece_idx = piece.color as usize * 6 + piece.piece_type as usize;
        let sq_idx = if perspective == Color::White {
            sq.index()
        } else {
            sq.flip_vertical().index()
        };
        sq_idx * 12 + piece_idx
    }
}

impl Default for NNUENetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Accumulator for incremental NNUE updates
/// Stores the hidden layer activations for both perspectives
#[derive(Clone)]
pub struct NNUEAccumulator {
    /// White's perspective accumulator [HIDDEN1_SIZE]
    pub white: Vec<i16>,
    /// Black's perspective accumulator [HIDDEN1_SIZE]
    pub black: Vec<i16>,
    /// Whether the accumulator is valid
    pub valid: bool,
}

impl NNUEAccumulator {
    pub fn new() -> Self {
        NNUEAccumulator {
            white: vec![0; HIDDEN1_SIZE],
            black: vec![0; HIDDEN1_SIZE],
            valid: false,
        }
    }

    /// Refresh the accumulator from scratch for a position
    pub fn refresh(&mut self, board: &Board, network: &NNUENetwork) {
        // Reset to biases
        self.white.copy_from_slice(&network.hidden1_biases);
        self.black.copy_from_slice(&network.hidden1_biases);

        // Add all pieces
        for sq in 0..64 {
            if let Some(piece) = board.piece_at[sq] {
                self.add_piece(piece, Square(sq as u8), network);
            }
        }

        self.valid = true;
    }

    /// Add a piece to the accumulator
    pub fn add_piece(&mut self, piece: Piece, sq: Square, network: &NNUENetwork) {
        let white_idx = NNUENetwork::feature_index(piece, sq, Color::White);
        let black_idx = NNUENetwork::feature_index(piece, sq, Color::Black);

        for h in 0..HIDDEN1_SIZE {
            self.white[h] += network.input_weights[white_idx][h];
            self.black[h] += network.input_weights[black_idx][h];
        }
    }

    /// Remove a piece from the accumulator
    pub fn remove_piece(&mut self, piece: Piece, sq: Square, network: &NNUENetwork) {
        let white_idx = NNUENetwork::feature_index(piece, sq, Color::White);
        let black_idx = NNUENetwork::feature_index(piece, sq, Color::Black);

        for h in 0..HIDDEN1_SIZE {
            self.white[h] -= network.input_weights[white_idx][h];
            self.black[h] -= network.input_weights[black_idx][h];
        }
    }

    /// Move a piece (remove from old square, add to new square)
    pub fn move_piece(&mut self, piece: Piece, from: Square, to: Square, network: &NNUENetwork) {
        self.remove_piece(piece, from, network);
        self.add_piece(piece, to, network);
    }
}

impl Default for NNUEAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// NNUE Evaluator
pub struct NNUEEvaluator {
    pub network: NNUENetwork,
}

impl NNUEEvaluator {
    /// Create a new NNUE evaluator
    pub fn new() -> Self {
        NNUEEvaluator {
            network: NNUENetwork::new(),
        }
    }

    /// Clipped ReLU activation function
    #[inline]
    fn clipped_relu(x: i16) -> i16 {
        x.clamp(0, ACTIVATION_SCALE as i16)
    }

    /// Evaluate a position using the NNUE network
    pub fn evaluate(&self, board: &Board, accumulator: &NNUEAccumulator) -> i32 {
        // Get the correct perspective based on side to move
        let (us, them) = match board.side_to_move {
            Color::White => (&accumulator.white, &accumulator.black),
            Color::Black => (&accumulator.black, &accumulator.white),
        };

        // Apply ClippedReLU to hidden layer 1 and concatenate both perspectives
        let mut hidden1_output = [0i16; HIDDEN1_SIZE * 2];
        for i in 0..HIDDEN1_SIZE {
            hidden1_output[i] = Self::clipped_relu(us[i]);
            hidden1_output[HIDDEN1_SIZE + i] = Self::clipped_relu(them[i]);
        }

        // Hidden layer 2
        let mut hidden2 = self.network.hidden2_biases.clone();
        for h1 in 0..(HIDDEN1_SIZE * 2) {
            let activation = hidden1_output[h1] as i32;
            for h2 in 0..HIDDEN2_SIZE {
                hidden2[h2] = (hidden2[h2] as i32 + activation * self.network.hidden2_weights[h1][h2] as i32 / WEIGHT_SCALE) as i16;
            }
        }

        // Apply ClippedReLU to hidden layer 2
        for h2 in 0..HIDDEN2_SIZE {
            hidden2[h2] = Self::clipped_relu(hidden2[h2]);
        }

        // Output layer
        let mut output = self.network.output_bias as i32;
        for h2 in 0..HIDDEN2_SIZE {
            output += hidden2[h2] as i32 * self.network.output_weights[h2] as i32 / WEIGHT_SCALE;
        }

        // Scale output to centipawns
        output
    }

    /// Quick evaluation without full NNUE (for when accumulator is invalid)
    /// Falls back to simple piece-square tables
    pub fn evaluate_simple(&self, board: &Board) -> i32 {
        let mut score = 0i32;

        // Piece values
        const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

        // Simple piece-square bonuses for centrality
        for sq in 0..64 {
            if let Some(piece) = board.piece_at[sq] {
                let value = PIECE_VALUES[piece.piece_type as usize];
                let sign = if piece.color == Color::White { 1 } else { -1 };

                // Base material
                score += sign * value;

                // Centrality bonus (stronger for minor pieces)
                let file = sq % 8;
                let rank = sq / 8;
                let center_bonus = match piece.piece_type {
                    PieceType::Knight | PieceType::Bishop => {
                        let center_dist = (3.5 - file as f32).abs() + (3.5 - rank as f32).abs();
                        (15.0 - center_dist * 3.0) as i32
                    }
                    PieceType::Pawn => {
                        // Pawns: bonus for advancement
                        let advancement = if piece.color == Color::White { rank } else { 7 - rank };
                        advancement as i32 * 5
                    }
                    PieceType::King => {
                        // King: prefer corners in middlegame (simplified)
                        if board.pieces(PieceType::Queen).count() > 0 {
                            let edge_dist = (file.min(7 - file)).min(rank.min(7 - rank)) as i32;
                            -edge_dist * 5
                        } else {
                            // Endgame: centralize
                            let center_dist = (3.5 - file as f32).abs() + (3.5 - rank as f32).abs();
                            (15.0 - center_dist * 3.0) as i32
                        }
                    }
                    _ => 0,
                };
                score += sign * center_bonus;
            }
        }

        // Bonus for bishop pair
        let white_bishops = board.pieces_of(PieceType::Bishop, Color::White).count();
        let black_bishops = board.pieces_of(PieceType::Bishop, Color::Black).count();
        if white_bishops >= 2 {
            score += 30;
        }
        if black_bishops >= 2 {
            score -= 30;
        }

        // Penalty for doubled pawns
        for file in 0..8 {
            let file_mask = crate::core::bitboard::Bitboard::file_mask(file);
            let white_pawns_on_file = (board.pieces_of(PieceType::Pawn, Color::White) & file_mask).count();
            let black_pawns_on_file = (board.pieces_of(PieceType::Pawn, Color::Black) & file_mask).count();
            if white_pawns_on_file > 1 {
                score -= (white_pawns_on_file - 1) as i32 * 15;
            }
            if black_pawns_on_file > 1 {
                score += (black_pawns_on_file - 1) as i32 * 15;
            }
        }

        // Return score from side to move's perspective
        if board.side_to_move == Color::White {
            score
        } else {
            -score
        }
    }

    /// Full evaluation: use NNUE if accumulator is valid, otherwise simple eval
    pub fn evaluate_full(&self, board: &Board, accumulator: &mut NNUEAccumulator) -> i32 {
        if !accumulator.valid {
            accumulator.refresh(board, &self.network);
        }
        
        // Blend NNUE and simple eval for more robust evaluation
        let nnue_score = self.evaluate(board, accumulator);
        let simple_score = self.evaluate_simple(board);
        
        // Weight towards simple eval since our NNUE isn't trained
        (simple_score * 3 + nnue_score) / 4
    }
}

impl Default for NNUEEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

// Global evaluator instance
static EVALUATOR: std::sync::OnceLock<NNUEEvaluator> = std::sync::OnceLock::new();

/// Get the global NNUE evaluator
pub fn evaluator() -> &'static NNUEEvaluator {
    EVALUATOR.get_or_init(NNUEEvaluator::new)
}

/// Evaluate a position
pub fn evaluate(board: &Board) -> i32 {
    evaluator().evaluate_simple(board)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nnue_network_creation() {
        let network = NNUENetwork::new();
        assert_eq!(network.input_weights.len(), INPUT_SIZE);
        assert_eq!(network.input_weights[0].len(), HIDDEN1_SIZE);
        assert_eq!(network.hidden1_biases.len(), HIDDEN1_SIZE);
    }

    #[test]
    fn test_accumulator_refresh() {
        let board = Board::startpos();
        let network = NNUENetwork::new();
        let mut acc = NNUEAccumulator::new();

        acc.refresh(&board, &network);
        assert!(acc.valid);
    }

    #[test]
    fn test_feature_index() {
        let piece = Piece::new(PieceType::Pawn, Color::White);
        let sq = Square::E4;
        
        let white_idx = NNUENetwork::feature_index(piece, sq, Color::White);
        let black_idx = NNUENetwork::feature_index(piece, sq, Color::Black);
        
        // Different perspectives should give different indices
        assert_ne!(white_idx, black_idx);
    }

    #[test]
    fn test_evaluate_startpos() {
        let board = Board::startpos();
        let evaluator = NNUEEvaluator::new();
        
        // Starting position should be roughly equal
        let score = evaluator.evaluate_simple(&board);
        assert!(score.abs() < 100, "Starting position eval: {}", score);
    }

    #[test]
    fn test_evaluate_material_advantage() {
        // White up a queen
        let board = Board::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let evaluator = NNUEEvaluator::new();
        
        let score = evaluator.evaluate_simple(&board);
        assert!(score > 800, "White up a queen should have high eval: {}", score);
    }

    #[test]
    fn test_nnue_full_evaluation() {
        let board = Board::startpos();
        let evaluator = NNUEEvaluator::new();
        let mut accumulator = NNUEAccumulator::new();

        let score = evaluator.evaluate_full(&board, &mut accumulator);
        assert!(score.abs() < 100, "Starting position NNUE eval: {}", score);
    }

    #[test]
    fn test_clipped_relu() {
        assert_eq!(NNUEEvaluator::clipped_relu(-100), 0);
        assert_eq!(NNUEEvaluator::clipped_relu(0), 0);
        assert_eq!(NNUEEvaluator::clipped_relu(50), 50);
        assert_eq!(NNUEEvaluator::clipped_relu(127), 127);
        assert_eq!(NNUEEvaluator::clipped_relu(200), 127);
    }
}

