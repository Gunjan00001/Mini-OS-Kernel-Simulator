//! Binary entry point.
//!
//! All behaviour lives in the library (`lib.rs`) so the simulator can be
//! driven from integration tests; `main` only builds the default kernel and
//! hands control to the terminal menu.

use mini_os_kernel::kernel::Kernel;
use mini_os_kernel::ui;

fn main() {
    // Default configuration used by the menus and the report screenshots:
    // 1024 units of contiguous memory, 4-byte pages, 8 physical page frames.
    let mut kernel = Kernel::new(1024, 4, 8);
    ui::main_menu(&mut kernel);
}
