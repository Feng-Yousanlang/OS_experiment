mod context;

use core::arch::global_asm;

use crate::batch::run_next_app;
use crate::syscall::syscall;

global_asm!(include_str!("trap.S"));

pub use context::TrapContext;

const EXC_U_ECALL: usize = 8;
const EXC_ILLEGAL_INSTRUCTION: usize = 2;
const EXC_STORE_FAULT: usize = 7;
const EXC_STORE_PAGE_FAULT: usize = 15;

fn read_scause() -> usize {
    let scause: usize;
    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) scause);
    }
    scause
}

fn read_stval() -> usize {
    let stval: usize;
    unsafe {
        core::arch::asm!("csrr {}, stval", out(reg) stval);
    }
    stval
}

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    let addr = __alltraps as *const () as usize;
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) addr);
    }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = read_scause();
    let stval = read_stval();
    let code = scause & 0xfff;
    let is_interrupt = (scause >> 63) != 0;

    if !is_interrupt {
        match code {
            EXC_U_ECALL => {
                cx.sepc += 4;
                cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            }
            EXC_STORE_FAULT | EXC_STORE_PAGE_FAULT => {
                println!("[kernel] PageFault in application, core dumped.");
                run_next_app();
            }
            EXC_ILLEGAL_INSTRUCTION => {
                println!("[kernel] IllegalInstruction in application, core dumped.");
                run_next_app();
            }
            _ => {
                panic!(
                    "Unsupported trap code {:#x}, stval = {:#x}!",
                    code, stval
                );
            }
        }
    } else {
        panic!(
            "Unsupported interrupt scause = {:#x}, stval = {:#x}!",
            scause, stval
        );
    }
    cx
}
