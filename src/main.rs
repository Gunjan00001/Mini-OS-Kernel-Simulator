use mini_os_kernel::kernel::Kernel;
use mini_os_kernel::ui;

fn main() {
    // 1024 units of contiguous memory, page size 4, 8 page frames.
    let mut kernel = Kernel::new(1024, 4, 8);
    ui::main_menu(&mut kernel);
}
