mod context;

use core::arch::global_asm;

use crate::syscall::syscall;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::set_next_trigger;

global_asm!(include_str!("trap.S"));

pub use context::TrapContext;

const EXC_U_ECALL: usize = 8;
const EXC_ILLEGAL_INSTRUCTION: usize = 2;
const EXC_STORE_FAULT: usize = 7;
const EXC_STORE_PAGE_FAULT: usize = 15;
const IRQ_S_TIMER: usize = 5;

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

pub fn enable_timer_interrupt() {
    unsafe {
        let mut sie: usize;
        core::arch::asm!("csrr {}, sie", out(reg) sie);
        sie |= 1 << 5;
        core::arch::asm!("csrw sie, {}", in(reg) sie);
    }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = read_scause();
    let stval = read_stval();
    let code = scause & 0xfff;
    let is_interrupt = (scause >> 63) != 0;

    if is_interrupt {
        match code {
            IRQ_S_TIMER => {
                set_next_trigger();
                suspend_current_and_run_next();
            }
            _ => {
                panic!(
                    "Unsupported interrupt scause = {:#x}, stval = {:#x}!",
                    scause, stval
                );
            }
        }
    } else {
        match code {
            EXC_U_ECALL => {
                cx.sepc += 4;
                cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
                // 保证每次 syscall 返回 U 态后仍开启 S 级中断（SIE <- SPIE）
                cx.sstatus |= 1 << 5;
            }
            EXC_STORE_FAULT | EXC_STORE_PAGE_FAULT => {
                println!(
                    "[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                    stval, cx.sepc
                );
                exit_current_and_run_next();
            }
            EXC_ILLEGAL_INSTRUCTION => {
                println!("[kernel] IllegalInstruction in application, core dumped.");
                exit_current_and_run_next();
            }
            _ => {
                panic!(
                    "Unsupported trap code {:#x}, stval = {:#x}!",
                    code, stval
                );
            }
        }
    }
    cx
}
