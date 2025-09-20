use crate::cli::SystemCommands;
use crate::config::Config;
use crate::error::{Result, SigilError};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub uptime: String,
    pub load_average: String,
    pub memory_info: MemoryInfo,
    pub cpu_info: CpuInfo,
    pub disk_usage: Vec<DiskInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub cores: u32,
    pub usage_percent: f64,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskInfo {
    pub filesystem: String,
    pub size: String,
    pub used: String,
    pub available: String,
    pub usage_percent: String,
    pub mount_point: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub status: String,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f64>,
}

pub async fn handle_command(cmd: &SystemCommands, config: &Config) -> Result<()> {
    match cmd {
        SystemCommands::Monitor { 
            service, 
            restart_if_high_cpu, 
            cpu_threshold 
        } => {
            if let Some(service_name) = service {
                monitor_service(service_name, *restart_if_high_cpu, *cpu_threshold).await?;
            } else {
                monitor_system(config).await?;
            }
        }
        SystemCommands::Exec { command, args } => {
            execute_command(command, args).await?;
        }
        SystemCommands::Info => {
            let info = get_system_info().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
    }
    Ok(())
}

pub async fn monitor_system(config: &Config) -> Result<()> {
    info!("🖥️  Starting system monitoring...");
    
    loop {
        let info = get_system_info().await?;
        
        println!("=== System Status ===");
        println!("Hostname: {}", info.hostname);
        println!("Uptime: {}", info.uptime);
        println!("Load Average: {}", info.load_average);
        println!("Memory: {:.1}% used ({} GB / {} GB)", 
                 info.memory_info.usage_percent,
                 info.memory_info.used / 1024 / 1024 / 1024,
                 info.memory_info.total / 1024 / 1024 / 1024);
        println!("CPU: {:.1}% usage", info.cpu_info.usage_percent);
        
        if info.cpu_info.usage_percent > config.modules.system.default_cpu_threshold as f64 {
            warn!("⚠️  High CPU usage detected: {:.1}%", info.cpu_info.usage_percent);
        }
        
        if info.memory_info.usage_percent > config.modules.system.default_memory_threshold as f64 {
            warn!("⚠️  High memory usage detected: {:.1}%", info.memory_info.usage_percent);
        }
        
        println!("--- Disk Usage ---");
        for disk in &info.disk_usage {
            println!("{}: {} ({}% used)", disk.mount_point, disk.size, disk.usage_percent);
        }
        
        println!();
        sleep(Duration::from_secs(config.modules.system.monitor_interval_seconds)).await;
    }
}

pub async fn monitor_service(service_name: &str, restart_if_high_cpu: bool, cpu_threshold: u8) -> Result<()> {
    info!("🔍 Monitoring service: {}", service_name);
    
    loop {
        let status = get_service_status(service_name).await?;
        
        println!("=== Service Status: {} ===", service_name);
        println!("Active: {}", if status.active { "✅ Yes" } else { "❌ No" });
        println!("Enabled: {}", if status.enabled { "✅ Yes" } else { "❌ No" });
        println!("Status: {}", status.status);
        
        if let Some(cpu_usage) = status.cpu_usage {
            println!("CPU Usage: {:.1}%", cpu_usage);
            
            if restart_if_high_cpu && cpu_usage > cpu_threshold as f64 {
                warn!("🚨 High CPU usage for {}: {:.1}% > {}%", service_name, cpu_usage, cpu_threshold);
                info!("🔄 Restarting service: {}", service_name);
                restart_service(service_name).await?;
            }
        }
        
        if let Some(memory_usage) = status.memory_usage {
            println!("Memory Usage: {} MB", memory_usage / 1024 / 1024);
        }
        
        println!();
        sleep(Duration::from_secs(30)).await;
    }
}

pub async fn get_system_info() -> Result<SystemInfo> {
    let hostname = get_command_output("hostname", &[]).await?;
    let uptime = get_command_output("uptime", &["-p"]).await?;
    let load_average = get_command_output("cat", &["/proc/loadavg"]).await?;
    
    let memory_info = get_memory_info().await?;
    let cpu_info = get_cpu_info().await?;
    let disk_usage = get_disk_usage().await?;
    
    Ok(SystemInfo {
        hostname: hostname.trim().to_string(),
        uptime: uptime.trim().to_string(),
        load_average: load_average.trim().to_string(),
        memory_info,
        cpu_info,
        disk_usage,
    })
}

async fn get_memory_info() -> Result<MemoryInfo> {
    let meminfo = get_command_output("cat", &["/proc/meminfo"]).await?;
    
    let mut total = 0u64;
    let mut available = 0u64;
    
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_memory_line(line)?;
        } else if line.starts_with("MemAvailable:") {
            available = parse_memory_line(line)?;
        }
    }
    
    let used = total - available;
    let usage_percent = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
    
    Ok(MemoryInfo {
        total: total * 1024, // Convert from KB to bytes
        available: available * 1024,
        used: used * 1024,
        usage_percent,
    })
}

async fn get_cpu_info() -> Result<CpuInfo> {
    let cpuinfo = get_command_output("cat", &["/proc/cpuinfo"]).await?;
    let cores = cpuinfo.lines().filter(|line| line.starts_with("processor")).count() as u32;
    
    // Simple CPU usage calculation (this would be more sophisticated in practice)
    let load_avg = get_command_output("cat", &["/proc/loadavg"]).await?;
    let load_1min: f64 = load_avg.split_whitespace()
        .next()
        .unwrap_or("0.0")
        .parse()
        .unwrap_or(0.0);
    
    let usage_percent = (load_1min / cores as f64) * 100.0;
    
    Ok(CpuInfo {
        cores,
        usage_percent: usage_percent.min(100.0),
        temperature: None, // Would require additional sensors
    })
}

async fn get_disk_usage() -> Result<Vec<DiskInfo>> {
    let df_output = get_command_output("df", &["-h", "--output=source,size,used,avail,pcent,target"]).await?;
    
    let mut disks = Vec::new();
    
    for line in df_output.lines().skip(1) { // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            disks.push(DiskInfo {
                filesystem: parts[0].to_string(),
                size: parts[1].to_string(),
                used: parts[2].to_string(),
                available: parts[3].to_string(),
                usage_percent: parts[4].to_string(),
                mount_point: parts[5].to_string(),
            });
        }
    }
    
    Ok(disks)
}

async fn get_service_status(service_name: &str) -> Result<ServiceStatus> {
    let status_output = get_command_output("systemctl", &["status", service_name]).await
        .unwrap_or_else(|_| "inactive".to_string());
    
    let is_active_output = get_command_output("systemctl", &["is-active", service_name]).await
        .unwrap_or_else(|_| "inactive".to_string());
    
    let is_enabled_output = get_command_output("systemctl", &["is-enabled", service_name]).await
        .unwrap_or_else(|_| "disabled".to_string());
    
    let active = is_active_output.trim() == "active";
    let enabled = is_enabled_output.trim() == "enabled";
    
    Ok(ServiceStatus {
        name: service_name.to_string(),
        active,
        enabled,
        status: status_output.lines().next().unwrap_or("unknown").to_string(),
        memory_usage: None, // Would require additional parsing
        cpu_usage: None,    // Would require additional parsing
    })
}

async fn restart_service(service_name: &str) -> Result<()> {
    let output = Command::new("sudo")
        .arg("systemctl")
        .arg("restart")
        .arg(service_name)
        .output()
        .map_err(|e| SigilError::system_command("sudo systemctl restart", &e.to_string()))?;
    
    if output.status.success() {
        info!("✅ Successfully restarted service: {}", service_name);
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("❌ Failed to restart service {}: {}", service_name, error);
        return Err(SigilError::system_command("systemctl restart", &error.to_string()));
    }
    
    Ok(())
}

pub async fn execute_command(command: &str, args: &[String]) -> Result<()> {
    info!("🚀 Executing: {} {}", command, args.join(" "));
    
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| SigilError::system_command(command, &e.to_string()))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            println!("{}", stdout);
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("❌ Command failed: {}", stderr);
        return Err(SigilError::system_command(command, &stderr.to_string()));
    }
    
    Ok(())
}

async fn get_command_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| SigilError::system_command(command, &e.to_string()))?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(SigilError::system_command(command, &error.to_string()))
    }
}

fn parse_memory_line(line: &str) -> Result<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>()
            .map_err(|e| SigilError::system_command("parse_memory", &e.to_string()))
    } else {
        Err(SigilError::system_command("parse_memory", "Invalid format"))
    }
}
