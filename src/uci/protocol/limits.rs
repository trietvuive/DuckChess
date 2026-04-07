use crate::engine::search::SearchLimits;
use vampirc_uci::uci::{UciSearchControl, UciTimeControl};

/// Build SearchLimits from vampirc-parsed go command (time_control, search_control).
pub(crate) fn go_to_limits(
    time_control: Option<&UciTimeControl>,
    search_control: Option<&UciSearchControl>,
) -> SearchLimits {
    let mut limits = SearchLimits::default();

    if let Some(sc) = search_control {
        limits.depth = sc.depth.map(i32::from);
        limits.nodes = sc.nodes;
    }

    if let Some(tc) = time_control {
        match tc {
            UciTimeControl::Infinite => limits.infinite = true,
            UciTimeControl::MoveTime(d) => {
                limits.movetime = Some(duration_to_millis(d));
            }
            UciTimeControl::TimeLeft {
                white_time,
                black_time,
                white_increment,
                black_increment,
                moves_to_go,
            } => {
                limits.wtime = white_time.as_ref().map(duration_to_millis);
                limits.btime = black_time.as_ref().map(duration_to_millis);
                limits.winc = white_increment.as_ref().map(duration_to_millis);
                limits.binc = black_increment.as_ref().map(duration_to_millis);
                limits.movestogo = moves_to_go.map(u32::from);
            }
            _ => {}
        }
    }

    limits
}

fn duration_to_millis(d: &vampirc_uci::Duration) -> u64 {
    d.num_milliseconds().max(0) as u64
}

/// If the line contains "multipv N", return Some(N) clamped to 1..=5.
pub(crate) fn parse_multipv_from_line(line: &str) -> Option<u32> {
    let line = line.to_lowercase();
    let mut rest = line.as_str();
    while let Some(idx) = rest.find("multipv") {
        rest = &rest[idx + 7..];
        let rest = rest.trim_start();
        let num: Option<u32> = rest.split_whitespace().next().and_then(|s| s.parse().ok());
        if let Some(n) = num {
            return Some(n.clamp(1, 5));
        }
    }
    None
}

/// Parse `go` tokens (after `go`) into limits; used by the manual `cmd_go` path.
pub(crate) fn limits_from_go_tokens(parts: &[&str], default_multipv: u32) -> SearchLimits {
    let mut limits = SearchLimits {
        multi_pv: default_multipv,
        ..SearchLimits::default()
    };
    let mut i = 1usize;

    while i < parts.len() {
        match parts[i] {
            "depth" if i + 1 < parts.len() => {
                limits.depth = parts[i + 1].parse().ok();
                i += 2;
            }
            "nodes" if i + 1 < parts.len() => {
                limits.nodes = parts[i + 1].parse().ok();
                i += 2;
            }
            "movetime" if i + 1 < parts.len() => {
                limits.movetime = parts[i + 1].parse().ok();
                i += 2;
            }
            "wtime" if i + 1 < parts.len() => {
                limits.wtime = parts[i + 1].parse().ok();
                i += 2;
            }
            "btime" if i + 1 < parts.len() => {
                limits.btime = parts[i + 1].parse().ok();
                i += 2;
            }
            "winc" if i + 1 < parts.len() => {
                limits.winc = parts[i + 1].parse().ok();
                i += 2;
            }
            "binc" if i + 1 < parts.len() => {
                limits.binc = parts[i + 1].parse().ok();
                i += 2;
            }
            "movestogo" if i + 1 < parts.len() => {
                limits.movestogo = parts[i + 1].parse().ok();
                i += 2;
            }
            "multipv" if i + 1 < parts.len() => {
                if let Ok(n) = parts[i + 1].parse::<u32>() {
                    limits.multi_pv = n.clamp(1, 5);
                }
                i += 2;
            }
            "infinite" => {
                limits.infinite = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    limits
}
