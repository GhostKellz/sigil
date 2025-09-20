use crate::cli::TaskCommands;
use crate::config::Config;
use crate::error::{Result, SigilError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::fs;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskDefinition {
    pub name: String,
    pub description: Option<String>,
    pub command: TaskCommand,
    pub parameters: HashMap<String, TaskParameter>,
    pub timeout_seconds: Option<u64>,
    pub retry_count: Option<u32>,
    pub environment: Option<HashMap<String, String>>,
    pub working_directory: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskCommand {
    Shell { script: String },
    System { command: String, args: Vec<String> },
    Module { module: String, action: String, params: HashMap<String, String> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskParameter {
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub parameter_type: ParameterType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Float,
    Path,
    Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskInstance {
    pub id: Uuid,
    pub definition_name: String,
    pub status: TaskStatus,
    pub parameters: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

pub async fn handle_command(cmd: &TaskCommands, config: &Config) -> Result<()> {
    match cmd {
        TaskCommands::List => {
            list_tasks(config).await?;
        }
        TaskCommands::Run { name, params } => {
            run_task(name, params, config).await?;
        }
        TaskCommands::Status { task } => {
            show_task_status(task, config).await?;
        }
        TaskCommands::Create { name, file } => {
            create_task(name, file.as_deref(), config).await?;
        }
    }
    Ok(())
}

pub async fn list_tasks(config: &Config) -> Result<()> {
    let tasks_dir = &config.tasks.definitions_dir;
    
    if !tasks_dir.exists() {
        println!("📂 No tasks directory found. Use 'sigil task create' to create your first task.");
        return Ok(());
    }
    
    println!("📋 Available Tasks:");
    println!("==================");
    
    let mut entries = fs::read_dir(tasks_dir).await?;
    let mut found_tasks = false;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                match load_task_definition(name, config).await {
                    Ok(task_def) => {
                        println!("🔧 {}", task_def.name);
                        if let Some(desc) = &task_def.description {
                            println!("   {}", desc);
                        }
                        println!("   Command: {:?}", task_def.command);
                        if !task_def.parameters.is_empty() {
                            println!("   Parameters: {}", task_def.parameters.len());
                        }
                        println!();
                        found_tasks = true;
                    }
                    Err(e) => {
                        warn!("⚠️  Failed to load task '{}': {}", name, e);
                    }
                }
            }
        }
    }
    
    if !found_tasks {
        println!("No valid task definitions found.");
    }
    
    Ok(())
}

pub async fn run_task(name: &str, params: &[String], config: &Config) -> Result<()> {
    info!("🚀 Running task: {}", name);
    
    let task_def = load_task_definition(name, config).await?;
    let parsed_params = parse_parameters(params)?;
    
    // Validate parameters
    validate_parameters(&task_def, &parsed_params)?;
    
    // Create task instance
    let mut task_instance = TaskInstance {
        id: Uuid::new_v4(),
        definition_name: name.to_string(),
        status: TaskStatus::Pending,
        parameters: parsed_params,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
        output: None,
        error: None,
        retry_count: 0,
    };
    
    // Save task state
    save_task_instance(&task_instance, config).await?;
    
    println!("📋 Task '{}' started with ID: {}", name, task_instance.id);
    
    // Execute task
    let result = execute_task_instance(&mut task_instance, &task_def, config).await;
    
    // Update final state
    save_task_instance(&task_instance, config).await?;
    
    match result {
        Ok(_) => {
            println!("✅ Task '{}' completed successfully", name);
            if let Some(output) = &task_instance.output {
                if !output.trim().is_empty() {
                    println!("📄 Output:\n{}", output);
                }
            }
        }
        Err(e) => {
            println!("❌ Task '{}' failed: {}", name, e);
            if let Some(error) = &task_instance.error {
                println!("💥 Error:\n{}", error);
            }
            return Err(e);
        }
    }
    
    Ok(())
}

pub async fn show_task_status(task_id: &str, config: &Config) -> Result<()> {
    // Try to parse as UUID first, otherwise search by name
    let task_instance = if let Ok(uuid) = Uuid::parse_str(task_id) {
        load_task_instance_by_id(&uuid, config).await?
    } else {
        // Find most recent instance by name
        find_latest_task_instance_by_name(task_id, config).await?
    };
    
    println!("📊 Task Status");
    println!("==============");
    println!("ID: {}", task_instance.id);
    println!("Name: {}", task_instance.definition_name);
    println!("Status: {:?}", task_instance.status);
    println!("Created: {}", task_instance.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
    
    if let Some(started) = task_instance.started_at {
        println!("Started: {}", started.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    
    if let Some(completed) = task_instance.completed_at {
        println!("Completed: {}", completed.format("%Y-%m-%d %H:%M:%S UTC"));
        
        if let Some(started) = task_instance.started_at {
            let duration = completed.signed_duration_since(started);
            println!("Duration: {}s", duration.num_seconds());
        }
    }
    
    if task_instance.retry_count > 0 {
        println!("Retries: {}", task_instance.retry_count);
    }
    
    if !task_instance.parameters.is_empty() {
        println!("Parameters:");
        for (key, value) in &task_instance.parameters {
            println!("  {}: {}", key, value);
        }
    }
    
    if let Some(output) = &task_instance.output {
        if !output.trim().is_empty() {
            println!("Output:\n{}", output);
        }
    }
    
    if let Some(error) = &task_instance.error {
        println!("Error:\n{}", error);
    }
    
    Ok(())
}

pub async fn create_task(name: &str, file_path: Option<&str>, config: &Config) -> Result<()> {
    let tasks_dir = &config.tasks.definitions_dir;
    fs::create_dir_all(tasks_dir).await?;
    
    let task_file = tasks_dir.join(format!("{}.toml", name));
    
    if task_file.exists() {
        return Err(SigilError::task_execution(format!("Task '{}' already exists", name)));
    }
    
    let task_def = if let Some(file) = file_path {
        // Load from specified file
        let content = fs::read_to_string(file).await?;
        toml::from_str::<TaskDefinition>(&content)?
    } else {
        // Create a sample task definition
        TaskDefinition {
            name: name.to_string(),
            description: Some(format!("Sample task: {}", name)),
            command: TaskCommand::Shell {
                script: "echo 'Hello from Sigil task!'".to_string(),
            },
            parameters: {
                let mut params = HashMap::new();
                params.insert("message".to_string(), TaskParameter {
                    description: "Message to display".to_string(),
                    required: false,
                    default_value: Some("Hello World".to_string()),
                    parameter_type: ParameterType::String,
                });
                params
            },
            timeout_seconds: Some(60),
            retry_count: Some(3),
            environment: None,
            working_directory: None,
        }
    };
    
    let content = toml::to_string_pretty(&task_def)?;
    fs::write(&task_file, content).await?;
    
    println!("✅ Created task definition: {}", task_file.display());
    println!("💡 Edit the file to customize the task, then run with: sigil task run {}", name);
    
    Ok(())
}

async fn load_task_definition(name: &str, config: &Config) -> Result<TaskDefinition> {
    let task_file = config.tasks.definitions_dir.join(format!("{}.toml", name));
    
    if !task_file.exists() {
        return Err(SigilError::resource_not_found(format!("Task definition: {}", name)));
    }
    
    let content = fs::read_to_string(&task_file).await?;
    let task_def: TaskDefinition = toml::from_str(&content)?;
    
    Ok(task_def)
}

async fn execute_task_instance(
    instance: &mut TaskInstance,
    definition: &TaskDefinition,
    _config: &Config,
) -> Result<()> {
    instance.status = TaskStatus::Running;
    instance.started_at = Some(Utc::now());
    
    let result = match &definition.command {
        TaskCommand::Shell { script } => {
            execute_shell_command(script, &instance.parameters, definition).await
        }
        TaskCommand::System { command, args } => {
            execute_system_command(command, args, &instance.parameters).await
        }
        TaskCommand::Module { module, action, params } => {
            execute_module_command(module, action, params, &instance.parameters).await
        }
    };
    
    instance.completed_at = Some(Utc::now());
    
    match result {
        Ok(output) => {
            instance.status = TaskStatus::Completed;
            instance.output = Some(output);
        }
        Err(e) => {
            instance.status = TaskStatus::Failed;
            instance.error = Some(e.to_string());
            return Err(e);
        }
    }
    
    Ok(())
}

async fn execute_shell_command(
    script: &str,
    parameters: &HashMap<String, String>,
    definition: &TaskDefinition,
) -> Result<String> {
    // Substitute parameters in script
    let mut expanded_script = script.to_string();
    for (key, value) in parameters {
        expanded_script = expanded_script.replace(&format!("${{{}}}", key), value);
    }
    
    let mut command = Command::new("bash");
    command.arg("-c").arg(&expanded_script);
    
    if let Some(env) = &definition.environment {
        for (key, value) in env {
            command.env(key, value);
        }
    }
    
    if let Some(work_dir) = &definition.working_directory {
        command.current_dir(work_dir);
    }
    
    let output = command.output()
        .map_err(|e| SigilError::task_execution(format!("Failed to execute shell command: {}", e)))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(SigilError::task_execution(format!("Shell command failed: {}", error)))
    }
}

async fn execute_system_command(
    command: &str,
    args: &[String],
    parameters: &HashMap<String, String>,
) -> Result<String> {
    // Substitute parameters in command and args
    let mut expanded_args = Vec::new();
    for arg in args {
        let mut expanded_arg = arg.clone();
        for (key, value) in parameters {
            expanded_arg = expanded_arg.replace(&format!("${{{}}}", key), value);
        }
        expanded_args.push(expanded_arg);
    }
    
    let output = Command::new(command)
        .args(&expanded_args)
        .output()
        .map_err(|e| SigilError::task_execution(format!("Failed to execute system command: {}", e)))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(SigilError::task_execution(format!("System command failed: {}", error)))
    }
}

async fn execute_module_command(
    module: &str,
    action: &str,
    params: &HashMap<String, String>,
    _user_params: &HashMap<String, String>,
) -> Result<String> {
    // This would integrate with the module system
    // For now, return a placeholder
    Ok(format!("Module command executed: {} {} with params: {:?}", module, action, params))
}

fn parse_parameters(params: &[String]) -> Result<HashMap<String, String>> {
    let mut parsed = HashMap::new();
    
    for param in params {
        if let Some((key, value)) = param.split_once('=') {
            parsed.insert(key.to_string(), value.to_string());
        } else {
            return Err(SigilError::task_execution(format!("Invalid parameter format: '{}'. Use key=value", param)));
        }
    }
    
    Ok(parsed)
}

fn validate_parameters(
    definition: &TaskDefinition,
    parameters: &HashMap<String, String>,
) -> Result<()> {
    for (param_name, param_def) in &definition.parameters {
        if param_def.required && !parameters.contains_key(param_name) {
            if param_def.default_value.is_none() {
                return Err(SigilError::task_execution(format!("Required parameter '{}' not provided", param_name)));
            }
        }
    }
    
    Ok(())
}

async fn save_task_instance(instance: &TaskInstance, config: &Config) -> Result<()> {
    let state_dir = &config.tasks.state_dir;
    fs::create_dir_all(state_dir).await?;
    
    let instance_file = state_dir.join(format!("{}.json", instance.id));
    let content = serde_json::to_string_pretty(instance)?;
    fs::write(&instance_file, content).await?;
    
    Ok(())
}

async fn load_task_instance_by_id(id: &Uuid, config: &Config) -> Result<TaskInstance> {
    let instance_file = config.tasks.state_dir.join(format!("{}.json", id));
    
    if !instance_file.exists() {
        return Err(SigilError::resource_not_found(format!("Task instance: {}", id)));
    }
    
    let content = fs::read_to_string(&instance_file).await?;
    let instance: TaskInstance = serde_json::from_str(&content)?;
    
    Ok(instance)
}

async fn find_latest_task_instance_by_name(name: &str, config: &Config) -> Result<TaskInstance> {
    let state_dir = &config.tasks.state_dir;
    
    if !state_dir.exists() {
        return Err(SigilError::resource_not_found(format!("No task instances found for: {}", name)));
    }
    
    let mut entries = fs::read_dir(state_dir).await?;
    let mut latest_instance: Option<TaskInstance> = None;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(instance) = serde_json::from_str::<TaskInstance>(&content) {
                    if instance.definition_name == name {
                        match &latest_instance {
                            None => latest_instance = Some(instance),
                            Some(current) => {
                                if instance.created_at > current.created_at {
                                    latest_instance = Some(instance);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    latest_instance.ok_or_else(|| SigilError::resource_not_found(format!("No task instances found for: {}", name)))
}
