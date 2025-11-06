use std::fmt::{self, Display};
use std::fs;
use std::io::{self, BufRead, Write};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "tasks.json";

//===========================================================================
// SECTION 1: TASK DEFINITIONS with PHANTOM TYPES
// Technique: Phantom Type Parameters
// We use generic structs `Task<State>` to encode a task's status
// into its type, preventing invalid operations at compile time.
//===========================================================================

/// Marker struct for a task that is pending.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pending;

/// Marker struct for a task that is completed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Completed;

/// A Task with a generic `State`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task<State> {
    id: u32,
    description: String,
    #[serde(skip)] // The state is encoded in the AnyTask enum, not here.
    _phantom: PhantomData<State>,
}

// Methods that only exist on `Task<Pending>`.
impl Task<Pending> {
    pub fn new(id: u32, description: String) -> Self {
        Task {
            id,
            description,
            _phantom: PhantomData,
        }
    }

    /// Consumes a pending task and returns a new completed task.
    /// This state transition is enforced by the type system.
    pub fn complete(self) -> Task<Completed> {
        Task {
            id: self.id,
            description: self.description,
            _phantom: PhantomData,
        }
    }
}

/// An enum to store tasks of different states in the same collection.
/// This is necessary because `Vec<Task<Pending>>` and `Vec<Task<Completed>>` are different types.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum AnyTask {
    Pending(Task<Pending>),
    Completed(Task<Completed>),
}

impl AnyTask {
    fn id(&self) -> u32 {
        match self {
            AnyTask::Pending(t) => t.id,
            AnyTask::Completed(t) => t.id,
        }
    }
}

// Custom Display for pretty-printing tasks.
impl Display for AnyTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyTask::Pending(t) => write!(f, "[ ] {:<4} {}", t.id, t.description),
            AnyTask::Completed(t) => write!(f, "[x] {:<4} {}", t.id, t.description),
        }
    }
}

//===========================================================================
// SECTION 2: COMMAND PARSING
// Technique: Error Handling & `TryFrom`/`TryInto`
// We define a custom error type and implement `TryFrom` to safely
// parse user input strings into structured `Command` enums.
//===========================================================================

#[derive(Debug)]
pub enum CommandError {
    InvalidCommand,
    MissingArgument(String),
    InvalidArgument(String),
}

impl Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::InvalidCommand => write!(f, "Invalid command. Try 'help'."),
            CommandError::MissingArgument(arg) => write!(f, "Missing argument for '{}'", arg),
            CommandError::InvalidArgument(val) => write!(f, "Invalid argument: '{}'", val),
        }
    }
}

pub enum Command {
    Add(String),
    Complete(u32),
    List,
    Help,
}

impl TryFrom<String> for Command {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.trim().splitn(2, ' ');
        let command_name = parts.next().ok_or(CommandError::InvalidCommand)?;
        let args = parts.next();

        match command_name.to_lowercase().as_str() {
            "add" => {
                let description = args
                    .ok_or(CommandError::MissingArgument("add".to_string()))?
                    .to_string();
                Ok(Command::Add(description))
            }
            "complete" => {
                let id_str = args.ok_or(CommandError::MissingArgument("complete".to_string()))?;
                let id = id_str
                    .parse::<u32>()
                    .map_err(|_| CommandError::InvalidArgument(id_str.to_string()))?;
                Ok(Command::Complete(id))
            }
            "list" => Ok(Command::List),
            "help" => Ok(Command::Help),
            "" => Err(CommandError::InvalidCommand),
            _ => Err(CommandError::InvalidCommand),
        }
    }
}

//===========================================================================
// SECTION 3: TASK MANAGER
// Manages all task-related state and logic.
//===========================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
struct TaskManager {
    tasks: Vec<AnyTask>,
    next_id: u32,
}

impl TaskManager {
    /// Loads tasks from disk, or creates a new manager if the file doesn't exist.
    fn load() -> Self {
        fs::read_to_string(SAVE_FILE)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| TaskManager {
                tasks: Vec::new(),
                next_id: 1,
            })
    }

    /// Saves the current task list to disk.
    fn save(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(SAVE_FILE, json)
    }

    fn add_task(&mut self, description: String) {
        let new_task = Task::new(self.next_id, description);
        self.tasks.push(AnyTask::Pending(new_task));
        println!("Added task {}.", self.next_id);
        self.next_id += 1;
    }

    /// Finds a pending task, completes it, and updates it in the list.
    fn complete_task(&mut self, id: u32) {
        let task_pos = self
            .tasks
            .iter()
            .position(|t| t.id() == id && matches!(t, AnyTask::Pending(_)));

        if let Some(pos) = task_pos {
            // Remove the pending task, getting ownership of it
            let old_task = self.tasks.remove(pos);
            if let AnyTask::Pending(pending_task) = old_task {
                // The magic of phantom types: .complete() consumes the Task<Pending>
                // and produces a Task<Completed>.
                let completed_task = pending_task.complete();
                // Add the new completed task back to the list
                self.tasks.push(AnyTask::Completed(completed_task));
                println!("Completed task {}.", id);
            }
        } else {
            println!("Error: Task {} not found or is already completed.", id);
        }
    }

    /// Lists all tasks using HOFs and closures.
    /// Technique: Higher-Order Functions & Closures
    fn list_tasks(&self) {
        if self.tasks.is_empty() {
            println!("No tasks yet. Add one with 'add <description>'.");
            return;
        }
        println!("---------------- TASKS ----------------");
        // .iter() is a HOF, .for_each() is a HOF, and |t|... is a closure.
        self.tasks.iter().for_each(|t| println!("{}", t));
        println!("---------------------------------------");
    }
}

/// Prints the help message.
fn print_help() {
    println!("\nAvailable Commands:");
    println!("  add <description>    - Add a new task");
    println!("  complete <id>        - Mark a task as complete");
    println!("  list                 - Show all tasks");
    println!("  help                 - Show this help message");
    println!("  exit / quit          - Exit the application\n");
}

//===========================================================================
// SECTION 4: MAIN APPLICATION LOGIC
//===========================================================================

fn main() {
    // Technique: Arc (Atomic Reference Counting)
    // We wrap the TaskManager in an Arc and a Mutex to allow safe, shared
    // access across multiple threads (main thread and autosave thread).
    let task_manager = Arc::new(Mutex::new(TaskManager::load()));

    // --- Autosave Thread ---
    let saver_manager = Arc::clone(&task_manager);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(15));
            let manager = saver_manager.lock().unwrap();
            if let Err(e) = manager.save() {
                eprintln!("[AUTOSAVE ERROR] Failed to save tasks: {}", e);
            } else {
                 // println!("[AUTOSAVE] Tasks saved successfully."); // Uncomment for debugging
            }
        }
    });

    println!("Welcome to the Stateful Task Manager!");
    print_help();

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    // Technique: `while let`
    // This provides an ergonomic way to loop over lines from standard input.
    // The loop continues as long as `read_line` returns `Ok`.
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        match handle.read_line(&mut buffer) {
            Ok(0) => break, // EOF (Ctrl+D)
            Ok(_) => {
                let input = buffer.trim();
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    break;
                }

                // We use `try_into()` which comes from our `TryFrom` implementation
                match Command::try_from(input.to_string()) {
                    Ok(command) => {
                        let mut manager = task_manager.lock().unwrap();
                        match command {
                            Command::Add(desc) => manager.add_task(desc),
                            Command::Complete(id) => manager.complete_task(id),
                            Command::List => manager.list_tasks(),
                            Command::Help => print_help(),
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(error) => {
                eprintln!("Error reading input: {}", error);
                break;
            }
        }
    }

    // Final save on exit
    println!("Saving tasks and exiting...");
    task_manager.lock().unwrap().save().expect("Failed to save tasks on exit.");
    println!("Goodbye!");
}