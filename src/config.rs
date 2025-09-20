use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use crate::cli::ConfigCommands;
use crate::error::SigilError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub logging: LoggingConfig,
    pub modules: ModulesConfig,
    pub secrets: SecretsConfig,
    pub tasks: TasksConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
    pub default_shell: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_enabled: bool,
    pub console_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModulesConfig {
    pub system: SystemModuleConfig,
    pub aws: Option<AwsConfig>,
    pub azure: Option<AzureConfig>,
    pub proxmox: Option<ProxmoxConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemModuleConfig {
    pub enabled: bool,
    pub monitor_interval_seconds: u64,
    pub default_cpu_threshold: u8,
    pub default_memory_threshold: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwsConfig {
    pub region: String,
    pub profile: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AzureConfig {
    pub subscription_id: Option<String>,
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxmoxConfig {
    pub endpoint: String,
    pub username: String,
    pub password: Option<String>,
    pub token_id: Option<String>,
    pub token_secret: Option<String>,
    pub verify_ssl: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecretsConfig {
    pub backend: String, // "env", "vault", "file"
    pub vault_endpoint: Option<String>,
    pub vault_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TasksConfig {
    pub definitions_dir: PathBuf,
    pub state_dir: PathBuf,
    pub max_concurrent_tasks: usize,
    pub default_retry_count: u32,
    pub default_timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let config_dir = home_dir.join(".config/sigil");
        let data_dir = home_dir.join(".local/share/sigil");
        let log_dir = data_dir.join("logs");

        Config {
            general: GeneralConfig {
                data_dir: data_dir.clone(),
                config_dir: config_dir.clone(),
                log_dir,
                default_shell: "/bin/bash".to_string(),
                timeout_seconds: 300,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file_enabled: true,
                console_enabled: true,
            },
            modules: ModulesConfig {
                system: SystemModuleConfig {
                    enabled: true,
                    monitor_interval_seconds: 30,
                    default_cpu_threshold: 80,
                    default_memory_threshold: 85,
                },
                aws: None,
                azure: None,
                proxmox: None,
            },
            secrets: SecretsConfig {
                backend: "env".to_string(),
                vault_endpoint: None,
                vault_token: None,
            },
            tasks: TasksConfig {
                definitions_dir: config_dir.join("tasks"),
                state_dir: data_dir.join("state"),
                max_concurrent_tasks: 5,
                default_retry_count: 3,
                default_timeout_seconds: 600,
            },
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub async fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(&config_path, content).await?;
        
        Ok(())
    }

    pub fn get_config_path() -> PathBuf {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home_dir.join(".config/sigil/config.toml")
    }

    pub fn get_value(&self, key: &str) -> Option<String> {
        // Simple key-value retrieval for CLI commands
        // This would be more sophisticated in a real implementation
        match key {
            "general.data_dir" => Some(self.general.data_dir.display().to_string()),
            "general.default_shell" => Some(self.general.default_shell.clone()),
            "logging.level" => Some(self.logging.level.clone()),
            _ => None,
        }
    }

    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        // Simple key-value setting for CLI commands
        match key {
            "general.default_shell" => {
                self.general.default_shell = value.to_string();
            }
            "logging.level" => {
                self.logging.level = value.to_string();
            }
            _ => {
                return Err(SigilError::invalid_config(key, "Unknown configuration key").into());
            }
        }
        Ok(())
    }
}

pub async fn handle_command(cmd: &ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Show => {
            let config = Config::load().await?;
            let content = toml::to_string_pretty(&config)?;
            println!("{}", content);
        }
        ConfigCommands::Init => {
            let config = Config::default();
            config.save().await?;
            println!("✅ Configuration initialized at: {:?}", Config::get_config_path());
        }
        ConfigCommands::Set { key, value } => {
            let mut config = Config::load().await?;
            config.set_value(key, value)?;
            config.save().await?;
            println!("✅ Set {} = {}", key, value);
        }
        ConfigCommands::Get { key } => {
            let config = Config::load().await?;
            if let Some(value) = config.get_value(key) {
                println!("{}", value);
            } else {
                eprintln!("❌ Configuration key '{}' not found", key);
            }
        }
    }
    Ok(())
}
