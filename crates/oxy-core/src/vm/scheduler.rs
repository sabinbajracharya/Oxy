//! Cooperative task scheduler for the Oxy VM.
//!
//! Manages spawned tasks, timers (sleep), and task dependencies so that
//! the VM can switch between tasks at yield points without blocking the
//! host OS thread.

use std::collections::{BinaryHeap, HashMap, VecDeque};

/// Unique identifier for a scheduled task.
pub type TaskId = usize;

/// Snapshot of a task's execution state when it yields.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TaskSnapshot {
    pub ip: usize,
    pub stack: Vec<crate::types::Value>,
    /// JIT execution state (used instead of stack when running under Cranelift).
    #[allow(dead_code)]
    pub jit_state: Option<JitTaskState>,
}

/// Execution state for a JIT-compiled task.
#[derive(Debug, Clone)]
pub struct JitTaskState {
    /// The JIT function's entry bytecode IP (used to look up the native fn pointer).
    pub entry_ip: usize,
    /// Bytecode IP to resume at within the function (0 = start from beginning).
    pub resume_ip: usize,
    /// Locals (copied from JitContext buffer).
    pub locals: Vec<crate::types::Value>,
    /// Operand stack values.
    pub operand_stack: Vec<crate::types::Value>,
    /// Number of local slots.
    pub local_count: usize,
    /// Yield reason (1=sleep, 2=await_task, 3=select).
    pub yield_reason: u32,
    /// Associated data (ms for sleep, task_id for await, etc.).
    pub yield_data: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
mod clock {
    use std::time::Instant;

    pub type TimeMark = Instant;

    pub fn now() -> TimeMark {
        Instant::now()
    }

    pub fn delay_from_now(ms: u64) -> TimeMark {
        Instant::now()
            .checked_add(std::time::Duration::from_millis(ms))
            .unwrap_or(Instant::now())
    }

    pub fn duration_until(mark: &TimeMark) -> std::time::Duration {
        let now = Instant::now();
        if *mark > now {
            mark.duration_since(now)
        } else {
            std::time::Duration::ZERO
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod clock {
    use std::cell::Cell;

    /// On WASM: monotonically-increasing counter. `Instant::now()` may be
    /// unavailable or unreliable depending on the runtime, and we can't
    /// block anyway, so a simple counter gives correct ordering without
    /// depending on host time APIs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct TimeMark(u64);

    pub fn now() -> TimeMark {
        thread_local! {
            static WASM_TICK: Cell<u64> = const { Cell::new(0) };
        }
        WASM_TICK.with(|t| {
            let v = t.get();
            t.set(v + 1);
            TimeMark(v)
        })
    }

    pub fn delay_from_now(_ms: u64) -> TimeMark {
        // WASM can't block — treat all delays as immediate next tick.
        now()
    }

    pub fn duration_until(_mark: &TimeMark) -> std::time::Duration {
        // WASM can't block — report zero so the event loop doesn't try.
        std::time::Duration::ZERO
    }
}

pub use clock::TimeMark;

/// What a task is currently doing (or waiting for).
#[derive(Debug, Clone)]
pub enum TaskStatus {
    /// Ready to run.
    Ready,
    /// Currently executing on the VM.
    Running,
    /// Blocked on another task (`.await` on an incomplete JoinHandle).
    WaitingOnTask(TaskId),
    /// Blocked on any of several tasks (`select(handle1, handle2, ...)`).
    #[allow(dead_code)]
    WaitingOnMultiple(Vec<TaskId>),
    /// Blocked on a timer (`sleep(ms)`).
    #[allow(dead_code)]
    WaitingOnTimer(TimeMark),
    /// Finished with a result.
    Done(crate::types::Value),
}

/// A managed task within the scheduler.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub snapshot: Option<TaskSnapshot>,
    pub status: TaskStatus,
    /// Total simulated time (sum of `sleep` durations, in ms) the task's body
    /// spent before completing. JIT tasks run eagerly to completion, so real
    /// wall-clock ordering can't be observed — this virtual duration lets
    /// `select` pick the task that *would* finish first.
    pub virtual_time: u64,
}

impl Task {
    fn new(id: TaskId) -> Self {
        Self {
            id,
            snapshot: None,
            status: TaskStatus::Ready,
            virtual_time: 0,
        }
    }
}

/// Ordering for the timer heap: earliest wake time first.
#[derive(Debug, Clone)]
struct TimerEntry(TimeMark, TaskId);

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for TimerEntry {}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0) // reverse: BinaryHeap is max-heap, we want min-heap
    }
}

/// Cooperative task scheduler.
pub struct Scheduler {
    /// All tasks, indexed by id.
    tasks: HashMap<TaskId, Task>,
    /// Tasks that are ready to execute.
    ready: VecDeque<TaskId>,
    /// Tasks waiting on timers, ordered by wake time.
    timers: BinaryHeap<TimerEntry>,
    /// Counter for assigning unique task ids.
    next_id: TaskId,
    /// The currently-running task (if any).
    current: Option<TaskId>,
}

#[allow(dead_code)]
impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            ready: VecDeque::new(),
            timers: BinaryHeap::new(),
            next_id: 0,
            current: None,
        }
    }

    /// Reset all state — used between JitVm invocations when reusing the global scheduler.
    pub fn reset(&mut self) {
        self.tasks.clear();
        self.ready.clear();
        self.timers.clear();
        self.next_id = 0;
        self.current = None;
    }

    /// Register a new task. Returns its id. The task is NOT added to
    /// the ready queue — call `make_ready` when it should be scheduled.
    pub fn create_task(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        let task = Task::new(id);
        self.tasks.insert(id, task);
        id
    }

    /// Add a task to the ready queue.
    pub fn make_ready(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = TaskStatus::Ready;
        }
        self.ready.push_back(id);
    }

    /// Store the initial snapshot for a freshly-created task and make it ready.
    pub fn save_new_task(&mut self, id: TaskId, snapshot: TaskSnapshot) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.snapshot = Some(snapshot);
        }
        self.make_ready(id);
    }

    /// Get the currently-running task id.
    pub fn current_task(&self) -> Option<TaskId> {
        self.current
    }

    /// Set the currently-running task.
    pub fn set_current(&mut self, id: TaskId) {
        self.current = Some(id);
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = TaskStatus::Running;
        }
    }

    /// Clear the current task (called after a task completes).
    pub fn clear_current(&mut self) {
        self.current = None;
    }

    /// Save the current JIT task's execution state.
    #[allow(dead_code)]
    pub fn save_current_jit(&mut self, jit_state: JitTaskState) {
        if let Some(id) = self.current {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.snapshot = Some(TaskSnapshot {
                    ip: jit_state.resume_ip,
                    stack: vec![],
                    jit_state: Some(jit_state),
                });
            }
        }
    }

    /// Save the current task's execution state.
    pub fn save_current(&mut self, snapshot: TaskSnapshot) {
        if let Some(id) = self.current {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.snapshot = Some(snapshot);
            }
        }
    }

    /// Yield the current task because it's waiting on a timer.
    pub fn yield_for_timer(&mut self, wake: TimeMark) {
        if let Some(id) = self.current.take() {
            self.timers.push(TimerEntry(wake, id));
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnTimer(wake);
            }
        }
    }

    /// Yield the current task with JIT state and a timer.
    #[allow(dead_code)]
    pub fn yield_jit_for_timer(&mut self, jit_state: JitTaskState, wake: TimeMark) {
        if let Some(id) = self.current.take() {
            self.timers.push(TimerEntry(wake, id));
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnTimer(wake);
                task.snapshot = Some(TaskSnapshot {
                    ip: jit_state.resume_ip,
                    stack: vec![],
                    jit_state: Some(jit_state),
                });
            }
        }
    }

    /// Yield the current task because it's waiting on another task.
    pub fn yield_for_task(&mut self, waited: TaskId) {
        if let Some(id) = self.current.take() {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnTask(waited);
            }
        }
    }

    /// Yield the current JIT task waiting on another task.
    #[allow(dead_code)]
    pub fn yield_jit_for_task(&mut self, waited: TaskId, jit_state: JitTaskState) {
        if let Some(id) = self.current.take() {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnTask(waited);
                task.snapshot = Some(TaskSnapshot {
                    ip: jit_state.resume_ip,
                    stack: vec![],
                    jit_state: Some(jit_state),
                });
            }
        }
    }

    /// Yield the current task because it's waiting on any of several tasks
    /// (used by `select`).
    pub fn yield_for_multiple(&mut self, task_ids: Vec<TaskId>) {
        if let Some(id) = self.current.take() {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnMultiple(task_ids);
            }
        }
    }

    /// Yield the current JIT task waiting on multiple tasks.
    #[allow(dead_code)]
    pub fn yield_jit_for_multiple(&mut self, task_ids: Vec<TaskId>, jit_state: JitTaskState) {
        if let Some(id) = self.current.take() {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::WaitingOnMultiple(task_ids);
                task.snapshot = Some(TaskSnapshot {
                    ip: jit_state.resume_ip,
                    stack: vec![],
                    jit_state: Some(jit_state),
                });
            }
        }
    }

    /// Mark a task as complete with a result. Returns the ids of any tasks
    /// that were waiting on this one (they should be moved to ready).
    pub fn complete(&mut self, id: TaskId, result: crate::types::Value) -> Vec<TaskId> {
        let mut woken = Vec::new();
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = TaskStatus::Done(result);
        }
        // Find tasks waiting on this one and move them to ready.
        for task in self.tasks.values_mut() {
            match &task.status {
                TaskStatus::WaitingOnTask(w) if *w == id => {
                    task.status = TaskStatus::Ready;
                    woken.push(task.id);
                }
                TaskStatus::WaitingOnMultiple(ids) if ids.contains(&id) => {
                    task.status = TaskStatus::Ready;
                    woken.push(task.id);
                }
                _ => {}
            }
        }
        for &wid in &woken {
            self.ready.push_back(wid);
        }
        woken
    }

    /// Check if a task is done, returning its result if so.
    pub fn task_result(&self, id: TaskId) -> Option<crate::types::Value> {
        self.tasks.get(&id).and_then(|t| match &t.status {
            TaskStatus::Done(v) => Some(v.clone()),
            _ => None,
        })
    }

    /// Record the simulated time a task's body spent (sum of its `sleep`s).
    pub fn set_virtual_time(&mut self, id: TaskId, ms: u64) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.virtual_time = ms;
        }
    }

    /// The simulated completion time recorded for a task (0 if unknown).
    pub fn task_virtual_time(&self, id: TaskId) -> u64 {
        self.tasks.get(&id).map_or(0, |t| t.virtual_time)
    }

    /// Get the snapshot for a task, removing it.
    pub fn take_snapshot(&mut self, id: TaskId) -> Option<TaskSnapshot> {
        self.tasks.get_mut(&id).and_then(|t| t.snapshot.take())
    }

    /// Pick the next ready task to run. Checks timers first.
    pub fn next_ready(&mut self) -> Option<TaskId> {
        // Move expired timers to ready.
        let now = clock::now();
        while let Some(entry) = self.timers.peek() {
            if entry.0 <= now {
                let TimerEntry(_, tid) = self.timers.pop().unwrap();
                if let Some(task) = self.tasks.get_mut(&tid) {
                    if matches!(task.status, TaskStatus::WaitingOnTimer(_)) {
                        task.status = TaskStatus::Ready;
                        self.ready.push_back(tid);
                    }
                }
            } else {
                break;
            }
        }
        self.ready.pop_front()
    }

    /// True if all tasks are done.
    pub fn all_done(&self) -> bool {
        self.tasks
            .values()
            .all(|t| matches!(t.status, TaskStatus::Done(_)))
    }

    /// Duration until the next timer fires, if any.
    pub fn next_timer(&self) -> Option<std::time::Duration> {
        self.timers
            .peek()
            .map(|entry| clock::duration_until(&entry.0))
    }
}

// SAFETY: The Oxy VM (and JIT) are single-threaded. Rc<RefCell<...>> in Value
// is never shared across threads. The Mutex wrapping in static storage is for
// safe in-thread access, not for cross-thread synchronization.
unsafe impl Send for Scheduler {}
unsafe impl Send for TaskSnapshot {}
unsafe impl Send for JitTaskState {}
unsafe impl Send for Task {}
