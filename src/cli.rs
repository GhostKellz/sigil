use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sigil",
    about = "Your DevOps familiar for automation, scripting, and cloud orchestration",
    long_about = "Sigil is a Rust-based CLI tool designed for Linux-focused scripting, \
                  homelab automation, and hybrid cloud orchestration across Proxmox, AWS, Azure, and beyond.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// System operations and monitoring
    #[command(subcommand)]
    System(SystemCommands),

    /// Task management and execution
    #[command(subcommand)]
    Task(TaskCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Show version information
    Version,
}

#[derive(Subcommand)]
pub enum SystemCommands {
    /// Monitor system resources and services
    Monitor {
        /// Service name to monitor
        service: Option<String>,
        
        /// Restart service if CPU usage exceeds threshold
        #[arg(long)]
        restart_if_high_cpu: bool,
        
        /// CPU threshold percentage (default: 80)
        #[arg(long, default_value = "80")]
        cpu_threshold: u8,
    },

    /// Execute system commands
    Exec {
        /// Command to execute
        command: String,
        
        /// Arguments for the command
        args: Vec<String>,
    },

    /// System information
    Info,
}

#[derive(Subcommand)]
pub enum TaskCommands {
    /// List available tasks
    List,

    /// Run a specific task
    Run {
        /// Task name to run
        name: String,
        
        /// Task parameters in key=value format
        #[arg(short, long)]
        params: Vec<String>,
    },

    /// Show task status
    Status {
        /// Task ID or name
        task: String,
    },

    /// Create a new task definition
    Create {
        /// Task name
        name: String,
        
        /// Task definition file
        #[arg(short, long)]
        file: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Initialize configuration file
    Init,

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        
        /// Configuration value
        value: String,
    },

    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
}
