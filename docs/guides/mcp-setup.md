# Claude Code MCP Setup & Quickstart Guide

This guide details how to configure **EvoSwarm** as a Model Context Protocol (MCP) server for Claude Code and run your first evolutionary code optimization job in under **15 minutes**.

---

## 1. Prerequisites & Host Self-Check (E0-2)

Before starting the MCP service, verify that your Linux host provides the isolation capabilities required by **The Crucible** sandbox runner:

```bash
evoswarm status
```

When all isolation primitives are functioning, the command outputs:
```
sandbox: ready
```

If the command reports `sandbox: degraded`, follow the [Troubleshooting E0-2 Self-Check Failures](#4-troubleshooting-e0-2-self-check-failures) section below to resolve missing kernel features before proceeding.

---

## 2. Configuring Claude Code MCP

EvoSwarm exposes its tool surface (`evolve`, `job_status`, `job_result`, `cancel_job`) over standard input/output JSON-RPC.

### Option A: Using the Claude CLI
Add the server using the native `claude mcp` command:

```bash
claude mcp add evoswarm -- evoswarm mcp
```

### Option B: JSON Configuration Snippet
Add the server entry to your Claude Code configuration file (`~/.claude/claude_desktop_config.json` or `~/.claude.json`):

```json
{
  "mcpServers": {
    "evoswarm": {
      "command": "evoswarm",
      "args": ["mcp"]
    }
  }
}
```

Verify that Claude Code discovers the tools:
```bash
claude mcp list
```

---

## 3. 15-Minute Quickstart: Running a Sample Job

Follow these steps to submit and inspect your first job through Claude Code:

### Step 1: Open Your Project
Navigate to your repository and launch Claude Code:
```bash
cd /path/to/my-project
claude
```

### Step 2: Request Evolution via Natural Language
Instruct Claude Code to invoke the `evolve` MCP tool:
```
> Please use EvoSwarm to optimize the performance of src/matrix.py while ensuring all pytest tests pass:
  test_command: "pytest tests/test_matrix.py"
```

Claude Code calls the `evolve` tool and receives an immediate durable job ticket:
```json
{
  "job_id": "018f3a5b-7c89-7000-8000-abcdef123456",
  "status": "queued"
}
```

### Step 3: Monitor Job Status
Ask Claude Code to check progress:
```
> What is the status of EvoSwarm job 018f3a5b-7c89-7000-8000-abcdef123456?
```
The `job_status` tool reports the current generation, active candidate count, and elapsed search duration.

### Step 4: Retrieve and Apply Verified Patch
Once completed, fetch the results:
```
> Show me the result of EvoSwarm job 018f3a5b-7c89-7000-8000-abcdef123456
```
The `job_result` tool provides the path to the verified git patch and markdown summary report. Review the diff and apply the patch to your branch:
```bash
git apply artifacts/018f3a5b-7c89-7000-8000-abcdef123456/candidate.patch
pytest tests/test_matrix.py
```

---

## 4. Troubleshooting E0-2 Self-Check Failures

If `evoswarm status` reports `sandbox: degraded`, consult the remediation corresponding to the reported check:

### 1. `UserNamespaces`: Unprivileged User Namespaces Blocked
- **Symptom:** `UserNamespaces: enable unprivileged user namespaces (sysctl kernel.unprivileged_userns_clone=1) and ensure the sandbox tool is not blocked by AppArmor; install util-linux if unshare is missing`
- **Root Cause:** The host kernel disables unprivileged namespace cloning or AppArmor blocks `bwrap`.
- **Fix:**
  ```bash
  # Enable unprivileged user namespace clone
  sudo sysctl -w kernel.unprivileged_userns_clone=1
  echo "kernel.unprivileged_userns_clone=1" | sudo tee /etc/sysctl.d/99-userns.conf

  # If on Ubuntu 24.04+ with restricted AppArmor namespace policies:
  sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
  echo "kernel.apparmor_restrict_unprivileged_userns=0" | sudo tee -a /etc/sysctl.d/99-userns.conf
  ```

### 2. `CgroupV2`: Missing Memory and PIDs Controller Delegation
- **Symptom:** `CgroupV2: delegate memory+pids controllers`
- **Root Cause:** systemd does not delegate cgroup v2 controllers to rootless user sessions.
- **Fix:**
  Create systemd override directory and delegate controllers:
  ```bash
  sudo mkdir -p /etc/systemd/system/user@.service.d/
  cat << 'EOF' | sudo tee /etc/systemd/system/user@.service.d/delegate.conf
  [Service]
  Delegate=memory pids cpu io
  EOF

  sudo systemctl daemon-reload
  ```

### 3. `Linger`: User Session Lingering Disabled
- **Symptom:** `Linger: loginctl enable-linger <user>`
- **Root Cause:** Background tasks terminate when SSH or shell sessions disconnect.
- **Fix:**
  ```bash
  loginctl enable-linger $USER
  ```
