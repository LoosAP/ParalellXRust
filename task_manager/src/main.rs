use std::fmt::{self, Display};
use std::fs;
use std::io::{self, BufRead, Write};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const SAVE_FILE: &str = "tasks.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pending;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Completed;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task<State> {
    id: u32,
    description: String,
    #[serde(skip)]
    _phantom: PhantomData<State>,
}

impl Task<Pending> {
    pub fn new(id: u32, description: String) -> Self {
        Task {
            id,
            description,
            _phantom: PhantomData,
        }
    }

    pub fn complete(self) -> Task<Completed> {
        Task {
            id: self.id,
            description: self.description,
            _phantom: PhantomData,
        }
    }
}

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

impl Display for AnyTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyTask::Pending(t) => write!(f, "[ ] {:<4} {}", t.id, t.description),
            AnyTask::Completed(t) => write!(f, "[x] {:<4} {}", t.id, t.description),
        }
    }
}

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

#[derive(Debug, Default, Serialize, Deserialize)]
struct TaskManager {
    tasks: Vec<AnyTask>,
    next_id: u32,
}

impl TaskManager {
    fn load() -> Self {
        fs::read_to_string(SAVE_FILE)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| TaskManager {
                tasks: Vec::new(),
                next_id: 1,
            })
    }

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

    fn complete_task(&mut self, id: u32) {
        let task_pos = self
            .tasks
            .iter()
            .position(|t| t.id() == id && matches!(t, AnyTask::Pending(_)));

        if let Some(pos) = task_pos {
            let old_task = self.tasks.remove(pos);
            if let AnyTask::Pending(pending_task) = old_task {
                let completed_task = pending_task.complete();
                self.tasks.push(AnyTask::Completed(completed_task));
                println!("Completed task {}.", id);
            }
        } else {
            println!("Error: Task {} not found or is already completed.", id);
        }
    }

    fn list_tasks(&self) {
        if self.tasks.is_empty() {
            println!("No tasks yet. Add one with 'add <description>'.");
            return;
        }
        println!("---------------- TASKS ----------------");
        self.tasks.iter().for_each(|t| println!("{}", t));
        println!("---------------------------------------");
    }
}

fn print_help() {
    println!("\nAvailable Commands:");
    println!("  add <description>    - Add a new task");
    println!("  complete <id>        - Mark a task as complete");
    println!("  list                 - Show all tasks");
    println!("  help                 - Show this help message");
    println!("  exit / quit          - Exit the application\n");
}

fn main() {
    let task_manager = Arc::new(Mutex::new(TaskManager::load()));

    let saver_manager = Arc::clone(&task_manager);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(15));
            let manager = saver_manager.lock().unwrap();
            if let Err(e) = manager.save() {
                eprintln!("[AUTOSAVE ERROR] Failed to save tasks: {}", e);
            }
        }
    });

    println!("Welcome to the Stateful Task Manager!");
    print_help();

    let stdin = io::stdin();
    let mut handle = stdin.lock();


    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        match handle.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let input = buffer.trim();
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    break;
                }

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

    println!("Saving tasks and exiting...");
    task_manager.lock().unwrap().save().expect("Failed to save tasks on exit.");
    println!("Goodbye!");
}