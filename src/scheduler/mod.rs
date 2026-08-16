use crate::task::{Task, TaskStatus};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerPolicy {
    Fifo,
    Lifo,
    Priority,
}

pub struct ScheduledTask {
    pub task_id: Uuid,
    pub priority: i32,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
}

pub struct TaskScheduler {
    tasks: DashMap<Uuid, Arc<RwLock<Task>>>,
    pending_queue: RwLock<VecDeque<Uuid>>,
    running_tasks: DashMap<Uuid, Arc<RwLock<Task>>>,
    completed_tasks: DashMap<Uuid, Arc<RwLock<Task>>>,
    failed_tasks: DashMap<Uuid, Arc<RwLock<Task>>>,
    policy: SchedulerPolicy,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            pending_queue: RwLock::new(VecDeque::new()),
            running_tasks: DashMap::new(),
            completed_tasks: DashMap::new(),
            failed_tasks: DashMap::new(),
            policy: SchedulerPolicy::Fifo,
        }
    }

    pub fn with_policy(mut self, policy: SchedulerPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn submit_task(&self, task: Task) -> Uuid {
        let task_id = task.id;
        let wrapped = Arc::new(RwLock::new(task));
        self.tasks.insert(task_id, wrapped.clone());
        self.pending_queue.write().push_back(task_id);
        task_id
    }

    pub fn get_task(&self, id: Uuid) -> Option<Arc<RwLock<Task>>> {
        self.tasks.get(&id).map(|t| t.value().clone())
    }

    pub fn next_task(&self) -> Option<Arc<RwLock<Task>>> {
        let mut queue = self.pending_queue.write();

        while let Some(task_id) = queue.pop_front() {
            if let Some(task_ref) = self.tasks.get(&task_id) {
                let task = task_ref.value().clone();
                let task_guard = task.read();
                if task_guard.status == TaskStatus::Pending {
                    drop(task_guard);
                    self.running_tasks.insert(task_id, task.clone());
                    return Some(task);
                }
            }
        }
        None
    }

    pub fn complete_task(&self, id: Uuid, result: impl Into<String>) {
        if let Some((_, task)) = self.running_tasks.remove(&id) {
            let mut task_guard = task.write();
            task_guard.set_status(TaskStatus::Completed);
            task_guard.set_result(result);
            drop(task_guard);
            self.completed_tasks.insert(id, task);
        }
    }

    pub fn fail_task(&self, id: Uuid, error: impl Into<String>) {
        if let Some((_, task)) = self.running_tasks.remove(&id) {
            let mut task_guard = task.write();
            task_guard.set_status(TaskStatus::Failed);
            task_guard.set_result(error);
            drop(task_guard);
            self.failed_tasks.insert(id, task);
        }
    }

    pub fn cancel_task(&self, id: Uuid) {
        if let Some(task) = self.tasks.get(&id) {
            let mut task_guard = task.write();
            task_guard.set_status(TaskStatus::Cancelled);
        }
    }

    pub fn get_pending_count(&self) -> usize {
        self.pending_queue.read().len()
    }

    pub fn get_running_count(&self) -> usize {
        self.running_tasks.len()
    }

    pub fn get_completed_count(&self) -> usize {
        self.completed_tasks.len()
    }

    pub fn get_failed_count(&self) -> usize {
        self.failed_tasks.len()
    }

    pub fn list_pending(&self) -> Vec<Uuid> {
        self.pending_queue.read().iter().cloned().collect()
    }

    pub fn list_running(&self) -> Vec<Uuid> {
        self.running_tasks.iter().map(|s| *s.key()).collect()
    }

    pub fn list_completed(&self) -> Vec<Uuid> {
        self.completed_tasks.iter().map(|s| *s.key()).collect()
    }

    pub fn list_failed(&self) -> Vec<Uuid> {
        self.failed_tasks.iter().map(|s| *s.key()).collect()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}
