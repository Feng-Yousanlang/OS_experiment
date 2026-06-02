/// 与 trap.S 中保存/恢复的 sstatus 原始值一致（避免依赖 riscv crate）
pub type Sstatus = usize;

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        }
        // SPP = 0：sret 后进入 U 态
        sstatus &= !(1 << 8);
        // SPIE = 1：sret 回到 U 态时会把 SIE 置 1，用户态才能收到时钟中断
        sstatus |= 1 << 5;
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}
