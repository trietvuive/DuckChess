//! NNUE training binary using bullet_lib.
//!
//! Requires the `train` feature (which pulls in bullet_lib):
//!
//! ```bash
//! cargo run --bin train_nnue --features train --release -- \
//!     --data data/test77-jan2022.binpack \
//!     --superbatches 400 \
//!     --lr 0.001 \
//!     --wdl 0.75
//! ```
//!
//! The trained `quantised.bin` is saved to `checkpoints/<net>/`. Copy it as
//! the engine's `net.bin`:
//!
//! ```bash
//! cp checkpoints/duckchess_nnue-<N>/quantised.bin src/engine/eval/nnue/net.bin
//! ```
//!
//! To chart an existing training log:
//!
//! ```bash
//! cargo run --bin train_nnue --features train -- --plot checkpoints/duckchess_nnue-60/log.txt
//! ```
//!
//! Architecture: `(768 → 256) × 2 → 1` with SCReLU, dual perspective.

#[cfg(feature = "train")]
use bullet_lib::{
    game::{
        formats::sfbinpack::{
            TrainingDataEntry,
            chess::{r#move::MoveType, piecetype::PieceType},
        },
        inputs::Chess768,
    },
    nn::optimiser::AdamW,
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainerBuilder, loader},
};

#[cfg(feature = "train")]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Some(log_path) = find_arg(&args, "--plot") {
        plot_log(&log_path);
        return;
    }

    let dataset_path =
        find_arg(&args, "--data").unwrap_or_else(|| "data/test77-jan2022.binpack".into());
    let superbatches: usize = find_arg(&args, "--superbatches")
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);
    let initial_lr: f32 = find_arg(&args, "--lr")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.001);
    let wdl_proportion: f32 = find_arg(&args, "--wdl")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.75);
    let threads: usize = find_arg(&args, "--threads")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);
    let batches_per_sb: usize = find_arg(&args, "--batches-per-sb")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);

    let hl_size = 256;
    let final_lr = initial_lr * 0.3f32.powi(5);

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .use_threads(threads)
        .inputs(Chess768)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(255),
            SavedFormat::id("l0b").round().quantise::<i16>(255),
            SavedFormat::id("l1w").round().quantise::<i16>(64),
            SavedFormat::id("l1b").round().quantise::<i32>(255 * 64),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs| {
            let l0 = builder.new_affine("l0", 768, hl_size);
            let l1 = builder.new_affine("l1", 2 * hl_size, 1);

            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden)
        });

    let schedule = TrainingSchedule {
        net_id: "duckchess_nnue".to_string(),
        eval_scale: 400.0,
        steps: TrainingSteps {
            batch_size: 16_384,
            batches_per_superbatch: batches_per_sb,
            start_superbatch: 1,
            end_superbatch: superbatches,
        },
        wdl_scheduler: wdl::ConstantWDL {
            value: wdl_proportion,
        },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr,
            final_lr,
            final_superbatch: superbatches,
        },
        save_rate: 20,
    };

    let settings = LocalSettings {
        threads,
        test_set: None,
        output_directory: "checkpoints",
        batch_queue_size: 64,
    };

    let buffer_size_mb = 512;
    fn filter(entry: &TrainingDataEntry) -> bool {
        entry.ply >= 16
            && !entry.pos.is_checked(entry.pos.side_to_move())
            && entry.score.unsigned_abs() <= 10_000
            && entry.mv.mtype() == MoveType::Normal
            && entry.pos.piece_at(entry.mv.to()).piece_type() == PieceType::None
    }
    let dataloader = loader::SfBinpackLoader::new(&dataset_path, buffer_size_mb, threads, filter);

    trainer.run(&schedule, &settings, &dataloader);

    let log_path = format!("checkpoints/duckchess_nnue-{superbatches}/log.txt");
    if std::path::Path::new(&log_path).exists() {
        println!();
        plot_log(&log_path);
    }
}

// ---------------------------------------------------------------------------
// Training log chart
// ---------------------------------------------------------------------------

#[cfg(feature = "train")]
fn plot_log(path: &str) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read {path}: {e}");
            std::process::exit(1);
        }
    };

    let mut superbatch_losses: Vec<(usize, f64)> = Vec::new();
    let mut current_sb = 0usize;
    let mut sb_sum = 0.0f64;
    let mut sb_count = 0usize;

    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 3 {
            continue;
        }
        let sb: usize = match parts[0].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let loss: f64 = match parts[2].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if sb != current_sb {
            if sb_count > 0 {
                superbatch_losses.push((current_sb, sb_sum / sb_count as f64));
            }
            current_sb = sb;
            sb_sum = 0.0;
            sb_count = 0;
        }
        sb_sum += loss;
        sb_count += 1;
    }
    if sb_count > 0 {
        superbatch_losses.push((current_sb, sb_sum / sb_count as f64));
    }

    if superbatch_losses.is_empty() {
        eprintln!("No training data found in {path}");
        return;
    }

    let first_loss = superbatch_losses.first().unwrap().1;
    let final_loss = superbatch_losses.last().unwrap().1;
    let min_loss = superbatch_losses
        .iter()
        .map(|&(_, l)| l)
        .fold(f64::INFINITY, f64::min);
    let max_loss = superbatch_losses
        .iter()
        .map(|&(_, l)| l)
        .fold(f64::NEG_INFINITY, f64::max);
    let total_sb = superbatch_losses.len();
    let reduction_pct = (1.0 - final_loss / first_loss) * 100.0;

    let chart_width = 50;
    let box_inner = 10 + 1 + chart_width; // y-axis label + "│" + chart area

    println!("  Training Loss Summary");
    println!("  {}", "─".repeat(box_inner + 2));
    println!(
        "  Superbatches : {:<8} First loss : {:.6}",
        total_sb, first_loss
    );
    println!(
        "  Min loss     : {:<8.6} Final loss : {:.6}",
        min_loss, final_loss
    );
    println!(
        "  Max loss     : {:<8.6} Reduction  : {:.1}%",
        max_loss, reduction_pct
    );
    println!();

    draw_chart(&superbatch_losses, chart_width, 18);
}

#[cfg(feature = "train")]
fn draw_chart(data: &[(usize, f64)], width: usize, height: usize) {
    if data.is_empty() {
        return;
    }

    let min_y = data.iter().map(|&(_, y)| y).fold(f64::INFINITY, f64::min);
    let max_y = data
        .iter()
        .map(|&(_, y)| y)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_range = if (max_y - min_y).abs() < 1e-12 {
        1.0
    } else {
        max_y - min_y
    };

    let n = data.len();
    let denom = n.saturating_sub(1).max(1);

    let mut grid = vec![vec![' '; width]; height];

    for (i, &(_, y)) in data.iter().enumerate() {
        let x = (i * (width - 1) / denom).min(width - 1);
        let row = ((max_y - y) / y_range * (height - 1) as f64).round() as usize;
        let row = row.min(height - 1);
        grid[row][x] = '█';

        if i > 0 {
            let prev_y = data[i - 1].1;
            let prev_x = ((i - 1) * (width - 1) / denom).min(width - 1);
            let prev_row = ((max_y - prev_y) / y_range * (height - 1) as f64).round() as usize;
            let prev_row = prev_row.min(height - 1);
            let (r0, r1) = if prev_row < row {
                (prev_row, row)
            } else {
                (row, prev_row)
            };
            let mid_x = ((prev_x + x) / 2).min(width - 1);
            for r in r0..=r1 {
                if grid[r][mid_x] == ' ' {
                    grid[r][mid_x] = '│';
                }
            }
        }
    }

    for (i, row) in grid.iter().enumerate() {
        let y_val = max_y - (i as f64 / (height - 1) as f64) * y_range;
        let line: String = row.iter().collect();
        println!("  {y_val:>8.5} │{line}");
    }

    let first_sb = data.first().unwrap().0;
    let last_sb = data.last().unwrap().0;
    println!("           └{:─<w$}", "", w = width);
    println!(
        "            {:<w$}{:>0}",
        format!("sb {first_sb}"),
        format!("sb {last_sb}"),
        w = width - format!("sb {last_sb}").len()
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "train")]
fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[cfg(not(feature = "train"))]
fn main() {
    eprintln!("Error: the `train` feature is required to build the training binary.");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  cargo run --bin train_nnue --features train --release -- --data <path>");
    eprintln!();
    eprintln!("Run with --plot <log.txt> to chart an existing training log.");
    std::process::exit(1);
}
