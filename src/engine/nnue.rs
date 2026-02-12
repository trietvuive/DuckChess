use shakmaty::{Bitboard, Chess, Color, Position, Role, Square};

const INPUT_SIZE: usize = 768;
const HIDDEN1_SIZE: usize = 256;
const HIDDEN2_SIZE: usize = 32;

static NNUE_DATA: &[u8] = include_bytes!("nnue.bin");

struct NNUEWeights {
    input_weights: Vec<f32>,
    input_biases: Vec<f32>,
    hidden_weights: Vec<f32>,
    hidden_biases: Vec<f32>,
    output_weights: Vec<f32>,
    output_bias: f32,
}

fn load_weights() -> NNUEWeights {
    let mut offset = 0;
    let input_weights = read_f32_vec(&mut offset, HIDDEN1_SIZE * INPUT_SIZE);
    let input_biases = read_f32_vec(&mut offset, HIDDEN1_SIZE);
    let hidden_weights = read_f32_vec(&mut offset, HIDDEN2_SIZE * HIDDEN1_SIZE * 2);
    let hidden_biases = read_f32_vec(&mut offset, HIDDEN2_SIZE);
    let output_weights = read_f32_vec(&mut offset, HIDDEN2_SIZE);
    let output_bias = f32::from_le_bytes([
        NNUE_DATA[offset],
        NNUE_DATA[offset + 1],
        NNUE_DATA[offset + 2],
        NNUE_DATA[offset + 3],
    ]);
    NNUEWeights {
        input_weights,
        input_biases,
        hidden_weights,
        hidden_biases,
        output_weights,
        output_bias,
    }
}

fn read_f32_vec(offset: &mut usize, count: usize) -> Vec<f32> {
    let mut vec = Vec::with_capacity(count);
    for _ in 0..count {
        vec.push(f32::from_le_bytes([
            NNUE_DATA[*offset],
            NNUE_DATA[*offset + 1],
            NNUE_DATA[*offset + 2],
            NNUE_DATA[*offset + 3],
        ]));
        *offset += 4;
    }
    vec
}

use std::sync::LazyLock;
static WEIGHTS: LazyLock<NNUEWeights> = LazyLock::new(load_weights);

fn get_feature_index(
    piece_type: Role,
    piece_color: Color,
    square: Square,
    perspective: Color,
) -> usize {
    let sq = if perspective == Color::White {
        square as usize
    } else {
        (square as usize) ^ 56
    };
    let piece_idx = match piece_type {
        Role::Pawn => 0,
        Role::Knight => 1,
        Role::Bishop => 2,
        Role::Rook => 3,
        Role::Queen => 4,
        Role::King => 5,
    };
    let color_offset = if piece_color == perspective { 0 } else { 6 };
    sq * 12 + piece_idx + color_offset
}

fn compute_accumulator(pos: &Chess, perspective: Color) -> Vec<f32> {
    let mut acc: Vec<f32> = WEIGHTS.input_biases.clone();

    for sq in Square::ALL {
        if let Some(piece) = pos.board().piece_at(sq) {
            let feat_idx = get_feature_index(piece.role, piece.color, sq, perspective);
            for (h, a) in acc.iter_mut().enumerate().take(HIDDEN1_SIZE) {
                *a += WEIGHTS.input_weights[h * INPUT_SIZE + feat_idx];
            }
        }
    }
    acc
}

fn clipped_relu(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn evaluate(pos: &Chess) -> i32 {
    let white_acc = compute_accumulator(pos, Color::White);
    let black_acc = compute_accumulator(pos, Color::Black);

    let (us_acc, them_acc) = if pos.turn() == Color::White {
        (&white_acc, &black_acc)
    } else {
        (&black_acc, &white_acc)
    };

    let mut hidden1 = Vec::with_capacity(HIDDEN2_SIZE);
    for h2 in 0..HIDDEN2_SIZE {
        let mut sum = WEIGHTS.hidden_biases[h2];
        for h1 in 0..HIDDEN1_SIZE {
            let us_val = clipped_relu(us_acc[h1]);
            let them_val = clipped_relu(them_acc[h1]);
            sum += us_val * WEIGHTS.hidden_weights[h2 * (HIDDEN1_SIZE * 2) + h1];
            sum += them_val * WEIGHTS.hidden_weights[h2 * (HIDDEN1_SIZE * 2) + HIDDEN1_SIZE + h1];
        }
        hidden1.push(clipped_relu(sum));
    }

    let mut output = WEIGHTS.output_bias;
    for (h2, &val) in hidden1.iter().enumerate().take(HIDDEN2_SIZE) {
        output += val * WEIGHTS.output_weights[h2];
    }

    (output * 1000.0) as i32
}

pub fn is_insufficient_material(pos: &Chess) -> bool {
    let dominated = pos.board().occupied();
    let dominated_count = dominated.count();

    if dominated_count == 2 {
        return true;
    }
    if dominated_count == 3
        && (pos.board().knights().count() == 1 || pos.board().bishops().count() == 1)
    {
        return true;
    }
    if dominated_count == 4 {
        let bishops = pos.board().bishops();
        if bishops.count() == 2 {
            let light = Bitboard::LIGHT_SQUARES;
            let dark = Bitboard::DARK_SQUARES;
            if (bishops & light).count() == 2 || (bishops & dark).count() == 2 {
                return true;
            }
        }
    }
    false
}
