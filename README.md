<div align="center">

# claudectx

**Launch Claude Code with different profiles**

<br/>

<img src="img/demo.svg" alt="claudectx demo" width="700"/>

<br/>
<br/>

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/claudectx.svg)](https://crates.io/crates/claudectx)
[![CI](https://github.com/FGRibreau/claudectx/workflows/CI/badge.svg)](https://github.com/FGRibreau/claudectx/actions)
[![GitHub release](https://img.shields.io/github/release/FGRibreau/claudectx.svg)](https://github.com/FGRibreau/claudectx/releases)
[![GitHub stars](https://img.shields.io/github/stars/FGRibreau/claudectx.svg)](https://github.com/FGRibreau/claudectx/stargazers)

</div>

---

## What is this?

**claudectx** manages multiple Claude Code accounts (Claude Max, Claude Team, personal) and launches Claude with the selected profile. Each profile stores only account-specific fields; your settings, MCP servers, and preferences stay in `~/.claude.json` untouched. Inspired by [kubectx](https://github.com/FGRibreau/kubectx-rs).

## How it works

1. **Save**: `claudectx save work` extracts account fields from `~/.claude.json` into `~/.claudectx/work.claude.json`
2. **Switch**: `claudectx work` patches `~/.claude.json` in-place with the profile's account fields, then launches `claude`

Only account-specific fields (OAuth account, userID, subscription caches, etc.) are stored in profiles. Everything else in `~/.claude.json` (settings, MCP servers, API keys) is preserved across switches.

## Features

- **In-place patching** - Switches accounts without losing settings or MCP config
- **Direct launch** - Launches Claude automatically after switching
- **Slim profiles** - Store only account credentials, not entire configs
- **Login workflow** - `claudectx login` to add new accounts interactively
- **Quick switch** - Interactive selection with arrow keys
- **Pass-through args** - Forward arguments to Claude: `claudectx work -- --dangerously-skip-permissions`
- **Auto-slugify** - Profile names are normalized (`FG@Work` → `fg-work`)
- **Zero config** - Works out of the box

---

## Sponsors

<table>
  <tr>
    <td align="center" width="200">
        <a href="https://getnatalia.com/">
        <img src="assets/sponsors/natalia.svg" height="60" alt="Natalia"/><br/>
        <b>Natalia</b>
        </a><br/>
        <sub>24/7 AI voice and whatsapp agent for customer services</sub>
    </td>
    <td align="center" width="200">
      <a href="https://nobullshitconseil.com/">
        <img src="assets/sponsors/nobullshitconseil.svg" height="60" alt="NoBullshitConseil"/><br/>
        <b>NoBullshitConseil</b>
      </a><br/>
      <sub>360° tech consulting</sub>
    </td>
    <td align="center" width="200">
      <a href="https://www.hook0.com/">
        <img src="assets/sponsors/hook0.png" height="60" alt="Hook0"/><br/>
        <b>Hook0</b>
      </a><br/>
      <sub>Open-Source Webhooks-as-a-Service</sub>
    </td>
    <td align="center" width="200">
      <a href="https://france-nuage.fr/">
        <img src="assets/sponsors/france-nuage.png" height="60" alt="France-Nuage"/><br/>
        <b>France-Nuage</b>
      </a><br/>
      <sub>Sovereign cloud hosting in France</sub>
    </td>
  </tr>
</table>

> **Interested in sponsoring?** [Get in touch](mailto:sponsoring@fgribreau.com)

---

## Quick Start

```sh
# 1. Install
brew install FGRibreau/tap/claudectx

# 2. Save your current account as a profile
claudectx save work

# 3. Launch Claude with the profile
claudectx work
```

---

## Installation

### macOS

```sh
# Homebrew (recommended)
brew install FGRibreau/tap/claudectx

# or with Cargo
cargo install claudectx
```

### Linux

```sh
# Debian/Ubuntu (.deb)
curl -LO https://github.com/FGRibreau/claudectx/releases/latest/download/claudectx_0.1.0_amd64.deb
sudo dpkg -i claudectx_0.1.0_amd64.deb

# Cargo (all distros)
cargo install claudectx

# or download binary
curl -LO https://github.com/FGRibreau/claudectx/releases/latest/download/claudectx_linux_x86_64.tar.gz
tar -xzf claudectx_linux_x86_64.tar.gz
sudo mv claudectx /usr/local/bin/
```

### Windows

```powershell
# Chocolatey
choco install claudectx

# Scoop
scoop bucket add extras
scoop install claudectx

# or with Cargo
cargo install claudectx
```

### All platforms

| Platform | Method | Command |
|----------|--------|---------|
| macOS | Homebrew | `brew install FGRibreau/tap/claudectx` |
| macOS | Cargo | `cargo install claudectx` |
| Linux | Debian/Ubuntu | `sudo dpkg -i claudectx_*_amd64.deb` |
| Linux | Cargo | `cargo install claudectx` |
| Windows | Chocolatey | `choco install claudectx` |
| Windows | Scoop | `scoop install claudectx` |
| Windows | Cargo | `cargo install claudectx` |
| All | Binary | [Download from Releases](https://github.com/FGRibreau/claudectx/releases) |

<details>
<summary>Available binaries</summary>

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | `claudectx_linux_x86_64.tar.gz` |
| Linux | x86_64 (static) | `claudectx_linux_x86_64_musl.tar.gz` |
| Linux | ARM64 | `claudectx_linux_aarch64.tar.gz` |
| Linux | ARM64 (static) | `claudectx_linux_aarch64_musl.tar.gz` |
| Linux | ARMv7 | `claudectx_linux_armv7.tar.gz` |
| Linux | x86_64 (.deb) | `claudectx_*_amd64.deb` |
| Linux | ARM64 (.deb) | `claudectx_*_arm64.deb` |
| macOS | Intel | `claudectx_darwin_x86_64.tar.gz` |
| macOS | Apple Silicon | `claudectx_darwin_aarch64.tar.gz` |
| Windows | x86_64 | `claudectx_windows_x86_64.zip` |

</details>

<details>
<summary>Manual installation from source</summary>

```sh
git clone https://github.com/FGRibreau/claudectx.git
cd claudectx
cargo build --release
sudo cp target/release/claudectx /usr/local/bin/
```

</details>

---

## Usage

| Command | Description |
|---------|-------------|
| `claudectx` | Interactive profile selection, then launch Claude |
| `claudectx <profile>` | Switch to profile and launch Claude |
| `claudectx <profile> -- <args>` | Launch Claude with profile and extra arguments |
| `claudectx list` | List all saved profiles (* marks current) |
| `claudectx save <name>` | Save current account as profile |
| `claudectx delete <name>` | Delete a profile |
| `claudectx login` | Login to a new Claude account and save it as a profile |

### Examples

```sh
# Launch Claude with "work" profile
claudectx work

# Launch with extra arguments
claudectx work -- --dangerously-skip-permissions

# Save current account as "personal" profile
claudectx save personal

# Login to a new account and save it as a profile
claudectx login

# Interactive selection then launch
claudectx

# List all profiles (* marks current)
claudectx list
# Output:
# work - FG @ Company *
# personal - FG @ Personal
```

---

## Configuration

### Storage

Profiles are stored as individual JSON files in `~/.claudectx/`:

```
~/.claudectx/
├── work.claude.json
├── personal.claude.json
└── side-project.claude.json
```

When you run `claudectx <profile>`:
1. Account-specific fields in `~/.claude.json` are patched in-place from the profile
2. Claude is launched and reads from the updated config

Each profile is a **slim** JSON file containing only account-specific fields:
- `oauthAccount` (email, organization, UUID)
- `userID`
- Subscription and cache fields (`groveConfigCache`, `s1mAccessCache`, etc.)

Your portable settings (MCP servers, API keys, preferences) stay in `~/.claude.json` and are never overwritten.

### Profile Names

Profile names are automatically slugified:
- `My Work Profile` → `my-work-profile`
- `FG@Company` → `fg-company`
- `Test Name` → `test-name`

---

## License

[MIT](LICENSE)

---

<div align="center">

**Like claudectx?** Check out [kubectx-rs](https://github.com/FGRibreau/kubectx-rs) for Kubernetes context switching

</div>
