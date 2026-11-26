//! Terminal color theme and Windows ANSI (VT) support.
//!
//! Colors auto-disable when stdout is not a terminal (pipes, test harness,
//! CI) and respect `NO_COLOR`. `FORCE_COLOR=1` forces them on, which the
//! screenshot pipeline uses to capture colored transcripts.
//!
//! Zero dependencies: on Windows the VT processing flag is enabled through
//! two direct kernel32 calls.

use std::io::IsTerminal;
use std::sync::OnceLock;

// ANSI SGR sequences. Named constants keep the UI code readable and make it
// impossible to forget the reset when adding a new style.
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const BRIGHT_GREEN: &str = "\x1b[92m";
pub const BRIGHT_YELLOW: &str = "\x1b[93m";

/// Foreground colors cycled by pid for tables and the Gantt legend.
pub const PID_FG: [&str; 6] = [RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN];

/// Whether ANSI colors should be emitted.
///
/// Detection order: `NO_COLOR` wins, then `FORCE_COLOR`, then a terminal
/// check, then Windows VT support. The result is cached because probing the
/// console is a syscall and the answer cannot change mid-process.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        let forced = std::env::var_os("FORCE_COLOR").is_some();
        if !forced && !std::io::stdout().is_terminal() {
            return false;
        }
        #[cfg(windows)]
        {
            let vt = enable_windows_vt();
            if !vt && !forced {
                return false;
            }
        }
        true
    })
}

/// Interactive console (for clear-screen + pauses): a real terminal, unlike
/// the screenshot pipeline which sets FORCE_COLOR but writes to a file.
pub fn interactive() -> bool {
    std::io::stdout().is_terminal()
}

/// Wrap `text` in ANSI `style` when colors are enabled.
pub fn paint(text: &str, style: &str) -> String {
    if enabled() && !style.is_empty() {
        format!("{style}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Pick the foreground color for a pid (deterministic cycling).
pub fn pid_fg(pid: u32) -> &'static str {
    PID_FG[pid as usize % PID_FG.len()]
}

/// Gantt cell: black text on the pid's background color (41..=46).
pub fn gantt_cell(pid: u32, padded_text: &str) -> String {
    if !enabled() {
        return padded_text.to_string();
    }
    let bg = 41 + (pid as usize % PID_FG.len());
    format!("\x1b[30;{bg}m{padded_text}{RESET}")
}

/// `[██████    ]` progress bar, colored by fill ratio.
pub fn bar(used: u32, total: u32, width: usize) -> String {
    let width = width.max(1);
    let filled = if total == 0 {
        0
    } else {
        ((u64::from(used) * width as u64) / u64::from(total)) as usize
    };
    let filled = filled.min(width);
    // Solid blocks + spaces: the shade glyph (░) falls back to a different
    // font in some terminals and looks like noise.
    let bar = format!("[{}{}]", "█".repeat(filled), " ".repeat(width - filled));
    let color = if used * 100 >= total * 90 {
        RED
    } else if used * 100 >= total * 70 {
        YELLOW
    } else {
        GREEN
    };
    paint(&bar, color)
}

/// Color a process-state label (padding is applied by the caller).
pub fn state_style(state: crate::process::State) -> &'static str {
    use crate::process::State::*;
    match state {
        New => CYAN,
        Ready => GREEN,
        Running => BRIGHT_YELLOW,
        Waiting => BLUE,
        Terminated => DIM,
    }
}

/// Enable `ENABLE_VIRTUAL_TERMINAL_PROCESSING` on the stdout console.
///
/// Windows consoles ignore ANSI escapes unless this flag is set. Two direct
/// kernel32 calls avoid pulling in a Windows bindings crate.
#[cfg(windows)]
fn enable_windows_vt() -> bool {
    use std::os::windows::io::AsRawHandle;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleMode(handle: isize, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: isize, mode: u32) -> i32;
    }
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    let stdout = std::io::stdout();
    let handle = stdout.as_raw_handle() as isize;
    let mut mode: u32 = 0;
    // SAFETY: handle is the process stdout handle; the calls only touch the
    // console mode word.
    unsafe {
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        } else {
            false
        }
    }
}
