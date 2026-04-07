//! Quantized NNUE evaluation with SIMD acceleration.
//!
//! **Architecture**: `(768 → HIDDEN_SIZE) × 2 → 1` with SCReLU activation.
//!
//! **Quantization** (bullet-lib compatible):
//! - Feature layer: i16 weights/biases at scale QA = 255
//! - Output layer:  i16 weights at scale QB = 64, i32 bias at scale QA × QB
//!
//! **Binary format** (`net.bin`, little-endian, no header):
//! ```text
//! l0w : i16 × (HIDDEN_SIZE × INPUT_SIZE)   feature weights (row-major)
//! l0b : i16 × HIDDEN_SIZE                  feature biases
//! l1w : i16 × (2 × HIDDEN_SIZE)            output weights [us ++ them]
//! l1b : i32 × 1                            output bias
//! ```
//!
//! Train a net with `bullet_lib` (see `src/bin/train.rs`) and concatenate the
//! quantized parameter files into `net.bin`:
//! ```bash
//! cat l0w.bin l0b.bin l1w.bin l1b.bin > net.bin
//! ```

mod simd;

use shakmaty::{Chess, Color, Position, Role, Square};
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Architecture constants
// ---------------------------------------------------------------------------

const INPUT_SIZE: usize = 768; // 64 squares × 6 piece types × 2 colors
const HIDDEN_SIZE: usize = 256;

const QA: i16 = 255; // feature-layer quantization scale
const QB: i16 = 64; // output-layer weight quantization scale
const EVAL_SCALE: i32 = 400; // sigmoid-logit → centipawn conversion

static NET_DATA: &[u8] = include_bytes!("net.bin");

// ---------------------------------------------------------------------------
// Weight storage
// ---------------------------------------------------------------------------

struct NnueWeights {
    /// Feature weights transposed to `[INPUT_SIZE][HIDDEN_SIZE]` for
    /// cache-friendly accumulator updates: activating feature `f` adds the
    /// contiguous block `ft_weights[f * HIDDEN_SIZE .. (f+1) * HIDDEN_SIZE]`.
    ft_weights: Vec<i16>,

    /// Feature biases `[HIDDEN_SIZE]`.
    ft_biases: Vec<i16>,

    /// Output weights `[2 * HIDDEN_SIZE]` — first half for side-to-move
    /// accumulator, second half for non-side-to-move.
    out_weights: Vec<i16>,

    /// Output bias (scalar, at scale QA × QB).
    out_bias: i32,
}

static WEIGHTS: LazyLock<NnueWeights> = LazyLock::new(load_weights);

fn load_weights() -> NnueWeights {
    let expected = HIDDEN_SIZE * INPUT_SIZE * 2 // l0w
                 + HIDDEN_SIZE * 2              // l0b
                 + 2 * HIDDEN_SIZE * 2          // l1w
                 + 4; // l1b
    assert!(
        NET_DATA.len() >= expected,
        "NNUE net.bin too small: expected {expected} bytes, got {}",
        NET_DATA.len()
    );

    let mut off = 0;

    // l0w stored row-major (HIDDEN_SIZE, INPUT_SIZE) — transpose for inference
    let raw_ft = read_i16_slice(&mut off, HIDDEN_SIZE * INPUT_SIZE);
    let ft_biases = read_i16_slice(&mut off, HIDDEN_SIZE);
    let out_weights = read_i16_slice(&mut off, 2 * HIDDEN_SIZE);
    let out_bias = read_i32(&mut off);

    // Transpose l0w: (HIDDEN_SIZE, INPUT_SIZE) → (INPUT_SIZE, HIDDEN_SIZE)
    let mut ft_weights = vec![0i16; INPUT_SIZE * HIDDEN_SIZE];
    for h in 0..HIDDEN_SIZE {
        for f in 0..INPUT_SIZE {
            ft_weights[f * HIDDEN_SIZE + h] = raw_ft[h * INPUT_SIZE + f];
        }
    }

    NnueWeights {
        ft_weights,
        ft_biases,
        out_weights,
        out_bias,
    }
}

fn read_i16_slice(off: &mut usize, count: usize) -> Vec<i16> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(i16::from_le_bytes([NET_DATA[*off], NET_DATA[*off + 1]]));
        *off += 2;
    }
    v
}

fn read_i32(off: &mut usize) -> i32 {
    let val = i32::from_le_bytes([
        NET_DATA[*off],
        NET_DATA[*off + 1],
        NET_DATA[*off + 2],
        NET_DATA[*off + 3],
    ]);
    *off += 4;
    val
}

// ---------------------------------------------------------------------------
// Feature mapping — Chess768 (same as bullet's `Chess768` input type)
// ---------------------------------------------------------------------------

/// Map `(piece role, piece color, square)` to a feature index in `[0, 768)`.
///
/// Must match bullet's `Chess768` layout:
///   `color_offset + piece_type * 64 + square`
/// where color_offset is 0 for own pieces and 384 for opponent pieces,
/// and the square is rank-mirrored for Black's perspective.
#[inline]
fn feature_index(role: Role, piece_color: Color, square: Square, perspective: Color) -> usize {
    let sq = if perspective == Color::White {
        square as usize
    } else {
        (square as usize) ^ 56
    };
    let piece_type = match role {
        Role::Pawn => 0,
        Role::Knight => 1,
        Role::Bishop => 2,
        Role::Rook => 3,
        Role::Queen => 4,
        Role::King => 5,
    };
    let color_offset = if piece_color == perspective { 0 } else { 384 };
    color_offset + piece_type * 64 + sq
}

// ---------------------------------------------------------------------------
// Accumulator
// ---------------------------------------------------------------------------

/// Full accumulator refresh from scratch for one perspective.
fn refresh_accumulator(pos: &Chess, perspective: Color) -> Vec<i16> {
    let w = &*WEIGHTS;
    let mut acc = w.ft_biases.clone();

    for sq in Square::ALL {
        if let Some(piece) = pos.board().piece_at(sq) {
            let f = feature_index(piece.role, piece.color, sq, perspective);
            let base = f * HIDDEN_SIZE;
            simd::vec_add_i16(&mut acc, &w.ft_weights[base..base + HIDDEN_SIZE]);
        }
    }

    acc
}

// ---------------------------------------------------------------------------
// Inference
// ---------------------------------------------------------------------------

/// NNUE score for the **side to move**, **before** shared
/// [`super::common::finalize_leaf`].
///
/// Quantized inference pipeline:
/// 1. Refresh accumulators for both perspectives.
/// 2. SCReLU activation: `clamp(x, 0, QA)²`
/// 3. Dot product with output weights (us + them halves).
/// 4. Add output bias (scaled up by QA to match).
/// 5. Divide by `QA² × QB` and multiply by `EVAL_SCALE`.
pub(crate) fn raw_stm_nnue(pos: &Chess) -> i32 {
    let white_acc = refresh_accumulator(pos, Color::White);
    let black_acc = refresh_accumulator(pos, Color::Black);

    let (us_acc, them_acc) = if pos.turn() == Color::White {
        (&white_acc, &black_acc)
    } else {
        (&black_acc, &white_acc)
    };

    let w = &*WEIGHTS;

    let us_dot = simd::screlu_dot(us_acc, &w.out_weights[..HIDDEN_SIZE]);
    let them_dot = simd::screlu_dot(them_acc, &w.out_weights[HIDDEN_SIZE..]);

    // us_dot and them_dot are at scale QA² × QB.
    // out_bias is at scale QA × QB — multiply by QA to match.
    let raw = us_dot + them_dot + w.out_bias * QA as i32;

    // Convert to centipawns: raw / (QA² × QB) × EVAL_SCALE
    // Use i64 to avoid overflow with large trained-net activations.
    let divisor = QA as i64 * QA as i64 * QB as i64;
    (raw as i64 * EVAL_SCALE as i64 / divisor) as i32
}
