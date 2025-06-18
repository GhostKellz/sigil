# Sigil

> Your DevOps familiar for automation, scripting, and cloud orchestration

**Sigil** is a Rust-based CLI tool designed for Linux-focused scripting, homelab automation, and hybrid cloud orchestration across Proxmox, AWS, Azure, and beyond. Think of it as your trusted task runner, automation framework, and DevOps assistant — in one cohesive binary.

---

## ✨ Key Features

* 🖥 **Linux Automation**: Script and execute system tasks, from backups to service restarts
* ☁️ **Cloud Control**: Manage AWS, Azure, Proxmox, and local resources from one interface
* 🔧 **Scripting Runtime**: Write modular automation routines with TOML/JSON/YAML inputs
* 🔁 **Job Runner**: Schedule, monitor, and retry automation tasks with logging
* 🔐 **Secrets Management**: Access and securely inject secrets into tasks (via Vault/env)
* 📊 **TUI Dashboard**: Real-time terminal UI for managing tasks and viewing output
* 📦 **Plugin System (planned)**: Extend via shell, WASM, Zig/Rust plugins

---

## 🚀 Example Use Cases

```bash
# Proxmox snapshot all VMs tagged "prod"
sigil proxmox snapshot --tag prod

# Restart nginx if CPU > 85%
sigil system monitor nginx --restart-if-high-cpu

# Backup a directory to S3 with timestamp
sigil aws s3 upload ./backup s3://mybucket/$(date +%F)

# Check Azure VM health across all regions
sigil azure vm healthcheck --all-regions

# Run a custom script task defined in config
sigil task run my-scheduled-job
```

---

## 🧱 Architecture Overview

```
src/
├── main.rs             # CLI entrypoint (clap-based)
├── runtime/            # Task executor, job scheduler, logger
├── modules/
│   ├── system.rs       # Linux system module
│   ├── proxmox.rs      # Proxmox HTTP API
│   ├── aws.rs          # AWS SDK wrapper
│   ├── azure.rs        # Azure REST calls
├── tui/                # Terminal UI interface
├── config.rs           # Config + secrets handling
└── plugin.rs           # WASM/shell plugin system (WIP)
```

---

## 🛠 Installation

```bash
# Install via cargo
cargo install sigil

# Or from source
git clone https://github.com/ghostkellz/sigil
cd sigil && cargo build --release
```

---

## 🧠 Philosophy

> **Sigil** isn’t just an automation tool — it’s a framework.

* **Predictable**: Declarative inputs, reliable behavior
* **Composable**: Mix native modules with plugins and shell
* **Minimal**: Zero bloat, efficient execution
* **Extensible**: Built for your stack — not locked in

---

## 📌 Roadmap Highlights

* [x] Proxmox snapshot & VM state API
* [x] AWS S3/EC2 module
* [x] Azure compute + resource group query
* [x] Local system monitor module
* [ ] Plugin engine (WASM & shell)
* [ ] Task DSL / scripting mode
* [ ] Encrypted vault backend (Vault, sops)
* [ ] Remote agent mode (multi-node orchestrator)

---

## 🔗 Related Tools

* 🔹 [`zion`](https://github.com/ghostkellz/zion) — Zig package/version manager
* 🔸 [`oxygen`](https://github.com/ghostkellz/oxygen) — Rust developer assistant
* 🔮 [`jarvis`](https://github.com/ghostkellz/jarvis) — AI operator & assistant (coming soon)

---

**Sigil** is your spellbook. Now go automate something. 🧙‍♂️

