//! Task registry for Oxy's async runtime.
//!
//! `spawn`/`await` run task bodies **eagerly to completion**: JIT functions are
//! native code that can't be paused mid-execution and resumed, so there is no
//! cooperative yielding, ready queue, or timer wheel. This registry simply
//! holds each spawned task's eagerly-computed result plus its *virtual*
//! completion time — the summed `sleep` durations the body would have taken —
//! which `select` uses to pick the task that *would* have finished first.

use std::collections::HashMap;

/// Unique identifier for a spawned task.
pub type TaskId = usize;

/// A spawned task: its eagerly-computed result (once complete) and the virtual
/// time its body accumulated via `sleep`.
struct Task {
    result: Option<crate::types::Value>,
    virtual_time: u64,
}

/// Registry of spawned tasks and their results.
pub struct Scheduler {
    /// All tasks, indexed by id.
    tasks: HashMap<TaskId, Task>,
    /// Counter for assigning unique task ids.
    next_id: TaskId,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 0,
        }
    }

    /// Register a new task and return its id.
    pub fn create_task(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(
            id,
            Task {
                result: None,
                virtual_time: 0,
            },
        );
        id
    }

    /// Store a task's eagerly-computed result.
    pub fn complete(&mut self, id: TaskId, result: crate::types::Value) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.result = Some(result);
        }
    }

    /// The result of a completed task, if it has one.
    pub fn task_result(&self, id: TaskId) -> Option<crate::types::Value> {
        self.tasks.get(&id).and_then(|t| t.result.clone())
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
}

// SAFETY: The Oxy VM (and JIT) are single-threaded. Rc<RefCell<...>> in Value
// is never shared across threads. The Mutex wrapping in static storage is for
// safe in-thread access, not for cross-thread synchronization.
unsafe impl Send for Scheduler {}
