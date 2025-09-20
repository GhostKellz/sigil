use thiserror::Error;

#[derive(Error, Debug)]
pub enum SigilError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Task execution failed: {message}")]
    TaskExecution { message: String },

    #[error("System command failed: {command} - {error}")]
    SystemCommand { command: String, error: String },

    #[error("Module error: {module} - {message}")]
    Module { module: String, message: String },

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Resource not found: {resource}")]
    ResourceNotFound { resource: String },

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    #[error("Invalid configuration: {field} - {reason}")]
    InvalidConfig { field: String, reason: String },
}

pub type Result<T> = std::result::Result<T, SigilError>;

impl SigilError {
    pub fn task_execution<S: Into<String>>(message: S) -> Self {
        SigilError::TaskExecution {
            message: message.into(),
        }
    }

    pub fn system_command<S: Into<String>>(command: S, error: S) -> Self {
        SigilError::SystemCommand {
            command: command.into(),
            error: error.into(),
        }
    }

    pub fn module<S: Into<String>>(module: S, message: S) -> Self {
        SigilError::Module {
            module: module.into(),
            message: message.into(),
        }
    }

    pub fn resource_not_found<S: Into<String>>(resource: S) -> Self {
        SigilError::ResourceNotFound {
            resource: resource.into(),
        }
    }

    pub fn permission_denied<S: Into<String>>(operation: S) -> Self {
        SigilError::PermissionDenied {
            operation: operation.into(),
        }
    }

    pub fn invalid_config<S: Into<String>>(field: S, reason: S) -> Self {
        SigilError::InvalidConfig {
            field: field.into(),
            reason: reason.into(),
        }
    }
}
