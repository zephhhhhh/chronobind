use std::fmt::Debug;
use std::sync::mpsc::Receiver as MPSCReceiver;
use std::sync::{Arc, Mutex};

use crate::ui::messages::AppMessage;

/// Shared thread-safe MPSC receiver.
pub type SharedRx<T> = Arc<Mutex<MPSCReceiver<T>>>;

/// Wraps a standard MPSC receiver in a thread-safe shared receiver.
fn wrap_rx<T>(rx: MPSCReceiver<T>) -> SharedRx<T> {
    Arc::new(Mutex::new(rx))
}

/// Boxed pointer to a backend task.
pub type BackendTaskPtr = Box<dyn BackendTask>;

/// Trait representing an asynchronous backend task, with an interface for polling progression.
pub trait BackendTask: Debug + Send + Sync {
    /// Returns the name of the task.
    #[must_use]
    fn task_name(&self) -> String;
    /// Returns the label to use when displaying the task progress.
    /// I.e. The text to show alongside the progress percentage.
    #[must_use]
    fn task_label(&self) -> Option<String> {
        None
    }
    /// Returns the text to use when displaying the task progress.
    #[must_use]
    fn progress_display(&self) -> Option<String> {
        let percentage = self.progress().map_or(0.0, |p| p * 100.0);
        if let Some(completed) = self.completed_count()
            && let Some(total) = self.total_count()
        {
            Some(format!("{completed}/{total} ({percentage:.2}%)"))
        } else {
            Some(format!("{percentage:.2}%"))
        }
    }
    /// Returns a complete formatted string to use when displaying the task progress.
    #[must_use]
    fn progress_formatted(&self, display_label: bool) -> String {
        let progress_label = self.progress_display().unwrap_or_else(|| {
            let progress = self.progress().map_or(0.0, |p| p * 100.0);
            format!("{progress:.2}%")
        });
        if display_label && let Some(label) = self.task_label() {
            format!("{label} - {progress_label}")
        } else {
            progress_label
        }
    }

    /// Poll the task for updates.
    fn poll(&mut self);

    /// Returns `true` if the task has started.
    #[must_use]
    fn started(&self) -> bool;
    /// Returns `true` if the task has finished.
    #[must_use]
    fn finished(&self) -> bool;
    /// Returns any error message from the task.
    #[must_use]
    fn error(&self) -> Option<String>;

    /// Returns the number of items completed.
    #[must_use]
    fn completed_count(&self) -> Option<usize>;
    /// Returns the total number of items to be completed.
    #[must_use]
    fn total_count(&self) -> Option<usize>;
    /// Returns the progress of the task as a float between 0.0 and 1.0.
    #[must_use]
    fn progress(&self) -> Option<f32>;
    /// Returns a progress percentage as a `u16` for the `Gauge`.
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn progress_ui(&self) -> u16 {
        self.progress()
            .map_or(0, |progress| (progress * 100.0).clamp(0.0, 100.0) as u16)
    }

    /// Returns the next task to be completed, if applicable.
    #[must_use]
    fn next_task(&mut self) -> Option<BackendTaskPtr>;

    /// Pushes a message to be sent after all tasks are complete, if applicable.
    fn add_on_all_complete(&mut self, msg: AppMessage);
    /// Pushes a message to be sent after this task is complete, if applicable.
    fn add_after_message(&mut self, msg: AppMessage);
    /// Returns a message to be sent after the task completes, if applicable.
    #[must_use]
    fn after_messages(&mut self) -> Option<Vec<AppMessage>>;
}

/// Represents progress updates for I/O operations.
#[derive(Debug)]
pub enum IOProgress {
    /// IO operation has started with a total number of items to complete.
    Started { total: usize },
    /// IO operation has advanced with the number of completed items and total items.
    Advanced { completed: usize, total: usize },
    /// IO operation has finished.
    Finished,
    /// IO operation encountered an error with an attached message.
    Error(String),
}

/// State of an I/O task.
#[derive(Debug, Default, Clone)]
pub struct IOTaskState {
    /// Total number of items to be completed.
    pub total: usize,
    /// Number of items completed.
    pub completed: usize,
    /// Whether the task has started.
    pub started: bool,
    /// Whether the task has finished.
    pub finished: bool,
    /// Any error message from the task.
    pub error: Option<String>,
}

/// Backend IO task.
#[derive(Debug)]
pub struct IOTask {
    /// Name of the task.
    pub name: Option<String>,
    /// Label for the task.
    pub label: Option<String>,
    /// Thread-safe receiver for IO progress updates.
    pub rx: Option<SharedRx<IOProgress>>,
    /// Current state of the I/O task.
    pub state: IOTaskState,
    /// Optional next task to be executed after this one.
    pub next: Option<BackendTaskPtr>,
    /// Optional messages to be sent after task completion.
    pub after_messages: Vec<AppMessage>,
}

impl IOTask {
    /// Default name for I/O tasks.
    pub const DEFAULT_NAME: &'static str = "I/O Task";

    /// Creates a new `IOTask` with the provided MPSC receiver.
    #[must_use]
    pub fn new(rx: MPSCReceiver<IOProgress>) -> Self {
        Self {
            name: None,
            label: None,
            rx: Some(wrap_rx(rx)),
            state: IOTaskState::default(),
            next: None,
            after_messages: Vec::new(),
        }
    }

    /// Adds a task to be executed after this has been completed.
    #[must_use]
    pub fn then<T: BackendTask + 'static>(mut self, next: T) -> Self {
        self.next = Some(Box::new(next));
        self
    }

    /// Adds a message to be sent after task completion.
    #[must_use]
    pub fn on_completion(mut self, msg: AppMessage) -> Self {
        self.add_after_message(msg);
        self
    }

    /// Adds a message to be sent after all tasks are complete.
    #[must_use]
    pub fn on_all_complete(mut self, msg: AppMessage) -> Self {
        self.add_on_all_complete(msg);
        self
    }

    /// Assign a name to the task.
    #[must_use]
    pub fn name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Assign a label to the task.
    #[must_use]
    pub fn label<T: Into<String>>(mut self, label: T) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl BackendTask for IOTask {
    fn task_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| Self::DEFAULT_NAME.to_string())
    }
    fn task_label(&self) -> Option<String> {
        self.label.clone()
    }

    fn poll(&mut self) {
        if let Some(rx) = &self.rx
            && let Ok(receiver) = rx.try_lock()
        {
            while let Ok(progress) = receiver.try_recv() {
                match progress {
                    IOProgress::Started { total } => {
                        self.state.started = true;
                        self.state.total = total;
                    }
                    IOProgress::Advanced { completed, total } => {
                        self.state.completed = completed;
                        self.state.total = total;
                    }
                    IOProgress::Finished => {
                        self.state.finished = true;
                    }
                    IOProgress::Error(msg) => {
                        self.state.error = Some(msg);
                        self.state.finished = true;
                    }
                }
            }
        }
    }

    fn started(&self) -> bool {
        self.state.started
    }
    fn finished(&self) -> bool {
        self.state.finished
    }
    fn error(&self) -> Option<String> {
        self.state.error.clone()
    }

    fn completed_count(&self) -> Option<usize> {
        Some(self.state.completed)
    }
    fn total_count(&self) -> Option<usize> {
        Some(self.state.total)
    }
    #[allow(clippy::cast_precision_loss)]
    fn progress(&self) -> Option<f32> {
        if self.state.total == 0 {
            None
        } else {
            Some(self.state.completed as f32 / self.state.total as f32)
        }
    }

    fn next_task(&mut self) -> Option<BackendTaskPtr> {
        std::mem::take(&mut self.next)
    }

    fn add_on_all_complete(&mut self, msg: AppMessage) {
        if let Some(next) = self.next.as_mut() {
            next.as_mut().add_on_all_complete(msg);
        } else {
            self.add_after_message(msg);
        }
    }
    fn add_after_message(&mut self, msg: AppMessage) {
        self.after_messages.push(msg);
    }
    fn after_messages(&mut self) -> Option<Vec<AppMessage>> {
        if self.after_messages.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.after_messages))
        }
    }
}
