mod context;
mod switch;
mod task;

use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use core::mem::MaybeUninit;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

struct TaskManagerInner {
    num_app: usize,
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

static mut TASK_MANAGER: MaybeUninit<TaskManagerInner> = MaybeUninit::uninit();
static mut TASK_MANAGER_INIT: bool = false;

unsafe fn inner() -> &'static TaskManagerInner {
    TASK_MANAGER.assume_init_ref()
}

unsafe fn inner_mut() -> &'static mut TaskManagerInner {
    TASK_MANAGER.assume_init_mut()
}

pub fn init() {
    unsafe {
        assert!(!TASK_MANAGER_INIT);
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx_ptr: 0,
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for i in 0..num_app {
            tasks[i].task_cx_ptr = init_app_cx(i) as *const _ as usize;
            tasks[i].task_status = TaskStatus::Ready;
        }
        TASK_MANAGER.write(TaskManagerInner {
            num_app,
            tasks,
            current_task: 0,
        });
        TASK_MANAGER_INIT = true;
    }
}

pub fn run_first_task() -> ! {
    unsafe {
        inner_mut().tasks[0].task_status = TaskStatus::Running;
        let next_task_cx_ptr2 = inner().tasks[0].get_task_cx_ptr2();
        let unused: usize = 0;
        __switch(&unused as *const _, next_task_cx_ptr2);
    }
    panic!("Unreachable in run_first_task!");
}

fn mark_current_suspended() {
    unsafe {
        let current = inner_mut().current_task;
        inner_mut().tasks[current].task_status = TaskStatus::Ready;
    }
}

fn mark_current_exited() {
    unsafe {
        let current = inner_mut().current_task;
        inner_mut().tasks[current].task_status = TaskStatus::Exited;
    }
}

fn find_next_task() -> Option<usize> {
    unsafe {
        let inner = inner();
        let current = inner.current_task;
        (current + 1..current + inner.num_app + 1)
            .map(|id| id % inner.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }
}

fn run_next_task() {
    if let Some(next) = find_next_task() {
        unsafe {
            let current = inner_mut().current_task;
            inner_mut().tasks[next].task_status = TaskStatus::Running;
            inner_mut().current_task = next;
            let current_task_cx_ptr2 = inner().tasks[current].get_task_cx_ptr2();
            let next_task_cx_ptr2 = inner().tasks[next].get_task_cx_ptr2();
            __switch(current_task_cx_ptr2, next_task_cx_ptr2);
        }
    } else {
        panic!("All applications completed!");
    }
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() -> ! {
    mark_current_exited();
    run_next_task();
    panic!("Unreachable in exit_current_and_run_next!");
}
