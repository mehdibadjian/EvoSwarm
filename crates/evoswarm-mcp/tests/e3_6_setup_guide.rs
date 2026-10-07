//! e3-6 setup guide — acceptance criteria (story §4).
//!
//! Gate: `cargo test -p evoswarm-mcp --test e3_6_setup_guide`.
//!
//! Validates:
//! - AC1: README and `docs/guides/mcp-setup.md` contain step-by-step instructions for running a
//!   sample job within 15 minutes on a fresh machine.
//! - AC2: README and `docs/guides/mcp-setup.md` include valid JSON config snippets for Claude Code
//!   MCP integration (`claude mcp add` / `mcpServers`), and explicit remediation fixes for every
//!   E0-2 host self-check failure (UserNamespaces, CgroupV2, and Linger).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("parent of crates")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn extract_json_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("```json") {
            let mut json_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                json_lines.push(lines[i]);
                i += 1;
            }
            blocks.push(json_lines.join("\n"));
        }
        i += 1;
    }
    blocks
}

#[test]
fn test_mcp_setup_guide_files_exist() {
    let root = repo_root();
    let readme_path = root.join("README.md");
    let guide_path = root.join("docs/guides/mcp-setup.md");

    assert!(readme_path.is_file(), "README.md must exist");
    assert!(
        guide_path.is_file(),
        "docs/guides/mcp-setup.md must exist: {:?}",
        guide_path
    );

    let readme = fs::read_to_string(&readme_path).expect("read README.md");
    let guide = fs::read_to_string(&guide_path).expect("read mcp-setup.md");

    assert!(!readme.trim().is_empty(), "README.md must not be empty");
    assert!(!guide.trim().is_empty(), "docs/guides/mcp-setup.md must not be empty");
    assert!(
        readme.contains("docs/guides/mcp-setup.md") || readme.contains("mcp-setup.md"),
        "README.md must link to the MCP setup guide"
    );
}

#[test]
fn test_claude_config_json_syntax() {
    let root = repo_root();
    let guide_path = root.join("docs/guides/mcp-setup.md");
    assert!(guide_path.is_file(), "docs/guides/mcp-setup.md must exist");

    let guide = fs::read_to_string(&guide_path).expect("read mcp-setup.md");
    let json_blocks = extract_json_blocks(&guide);

    assert!(
        !json_blocks.is_empty(),
        "docs/guides/mcp-setup.md must contain at least one json block"
    );

    let mut found_mcp_config = false;
    for block in &json_blocks {
        if let Ok(val) = serde_json::from_str::<Value>(block) {
            if let Some(mcp_servers) = val.get("mcpServers") {
                if let Some(evoswarm) = mcp_servers.get("evoswarm") {
                    assert!(
                        evoswarm.get("command").is_some(),
                        "evoswarm config must specify command"
                    );
                    found_mcp_config = true;
                }
            }
        }
    }

    assert!(
        found_mcp_config,
        "docs/guides/mcp-setup.md must contain a valid mcpServers.evoswarm JSON snippet"
    );

    // Also assert claude mcp add CLI command is documented
    assert!(
        guide.contains("claude mcp add"),
        "docs/guides/mcp-setup.md must describe the `claude mcp add` command"
    );
}

#[test]
fn test_troubleshooting_fixes_for_all_e0_2_failures() {
    let root = repo_root();
    let guide_path = root.join("docs/guides/mcp-setup.md");
    let readme_path = root.join("README.md");

    let guide = fs::read_to_string(&guide_path).unwrap_or_default();
    let readme = fs::read_to_string(&readme_path).unwrap_or_default();
    let combined = format!("{guide}\n{readme}");

    // E0-2 Failure 1: UserNamespaces (sysctl / apparmor)
    assert!(
        combined.contains("kernel.unprivileged_userns_clone") || combined.contains("unprivileged_userns"),
        "Guide must provide fix for UserNamespaces failure (kernel.unprivileged_userns_clone)"
    );
    assert!(
        combined.contains("AppArmor") || combined.contains("apparmor"),
        "Guide must explain AppArmor resolution for unprivileged bwrap sandboxes"
    );

    // E0-2 Failure 2: CgroupV2 (memory / pids delegation)
    assert!(
        combined.contains("cgroup") && (combined.contains("memory") || combined.contains("pids")),
        "Guide must provide fix for CgroupV2 controller delegation failure"
    );
    assert!(
        combined.contains("systemd") || combined.contains("delegate"),
        "Guide must document systemd cgroup delegation configuration"
    );

    // E0-2 Failure 3: Linger (systemd user lingering)
    assert!(
        combined.contains("loginctl enable-linger"),
        "Guide must document `loginctl enable-linger` for the Linger check failure"
    );
}

#[test]
fn test_sample_job_walkthrough_documented() {
    let root = repo_root();
    let guide_path = root.join("docs/guides/mcp-setup.md");
    let guide = fs::read_to_string(&guide_path).unwrap_or_default();

    assert!(
        guide.contains("evolve"),
        "Guide must walk through running an evolve job"
    );
    assert!(
        guide.contains("status") || guide.contains("result"),
        "Guide must explain checking job status or inspecting job results"
    );
    assert!(
        guide.contains("15 minute") || guide.contains("15-minute"),
        "Guide must highlight the 15-minute quickstart target"
    );
}
