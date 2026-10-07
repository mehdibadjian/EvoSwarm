#!/usr/bin/env python3
"""myloop-lean: Deterministic sprint status ledger manager, anti-cheat gate, and incident trigger."""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    import yaml
except ImportError:
    yaml = None


VALID_STATUSES = [
    "backlog",
    "ready-for-dev",
    "in-progress",
    "review",
    "done",
    "blocked",
    "optional",
]

ALLOWED_TRANSITIONS = {
    "backlog": ["ready-for-dev", "blocked"],
    "ready-for-dev": ["in-progress", "blocked", "backlog"],
    "in-progress": ["review", "blocked", "ready-for-dev"],
    "review": ["done", "in-progress", "blocked"],
    "blocked": ["backlog", "ready-for-dev", "in-progress", "review"],
    "done": ["review"],  # Allow reopening if review finds regressions
    "optional": ["done", "in-progress"],
}


class SprintLedger:
    """Manages sprint-status.yaml reading, updating, and querying."""

    def __init__(self, ledger_path: Path):
        self.path = Path(ledger_path)
        if not self.path.exists():
            raise FileNotFoundError(f"Sprint ledger not found at {self.path}")
        self._load()

    def _load(self):
        self.raw_content = self.path.read_text(encoding="utf-8")
        if yaml:
            self.data = yaml.safe_load(self.raw_content) or {}
        else:
            self.data = {}

    def get_status(self, story_key: str) -> Optional[str]:
        dev_status = self.data.get("development_status", {})
        return dev_status.get(story_key)

    def get_tier(self, story_key: str) -> str:
        tiers = self.data.get("execution_tiers", {})
        return tiers.get(story_key, "standard")

    def get_next_actionable_story(self) -> Optional[Dict[str, str]]:
        """Finds the first story marked 'ready-for-dev'."""
        dev_status = self.data.get("development_status", {})
        for key, status in dev_status.items():
            if status == "ready-for-dev":
                return {
                    "key": key,
                    "status": status,
                    "tier": self.get_tier(key),
                }
        return None

    def update_status(self, story_key: str, new_status: str) -> bool:
        """Updates story status atomically while preserving comments."""
        if new_status not in VALID_STATUSES:
            raise ValueError(f"Invalid status '{new_status}'. Allowed: {VALID_STATUSES}")

        current_status = self.get_status(story_key)
        if current_status:
            allowed = list(ALLOWED_TRANSITIONS.get(current_status, []))
            if story_key.startswith("epic-") and current_status == "in-progress" and new_status == "done":
                allowed.append("done")
            if new_status not in allowed:
                raise ValueError(
                    f"Invalid status transition: cannot move '{story_key}' from '{current_status}' to '{new_status}'"
                )

        # Update file using line-preserving regex replacement
        pattern = re.compile(
            rf"^([ \t]*{re.escape(story_key)}[ \t]*:[ \t]*)([^\n#]+)(.*)$",
            re.MULTILINE,
        )

        match = pattern.search(self.raw_content)
        if match:
            prefix = match.group(1)
            comment = match.group(3)
            if comment and not comment.startswith(" "):
                comment = " " + comment
            new_line = f"{prefix}{new_status}{comment}"
            new_content = self.raw_content[: match.start()] + new_line + self.raw_content[match.end() :]
        else:
            # If not found in raw content, locate development_status: section and append
            dev_idx = self.raw_content.find("development_status:")
            if dev_idx != -1:
                next_newline = self.raw_content.find("\n", dev_idx)
                insert_pos = next_newline + 1 if next_newline != -1 else len(self.raw_content)
                new_entry = f"  {story_key}: {new_status}\n"
                new_content = self.raw_content[:insert_pos] + new_entry + self.raw_content[insert_pos:]
            else:
                new_content = self.raw_content + f"\ndevelopment_status:\n  {story_key}: {new_status}\n"

        self.path.write_text(new_content, encoding="utf-8")
        self._load()
        return True

    def set_tier(self, story_key: str, tier: str) -> bool:
        """Sets execution tier for a story."""
        tier_idx = self.raw_content.find("execution_tiers:")
        if tier_idx == -1:
            self.raw_content += f"\nexecution_tiers:\n  {story_key}: {tier}\n"
        else:
            after_tiers = self.raw_content[tier_idx:]
            next_section = re.search(r"^[a-zA-Z0-9_-]+:", after_tiers[len("execution_tiers:"):], re.MULTILINE)
            section_len = next_section.start() + len("execution_tiers:") if next_section else len(after_tiers)
            tiers_content = after_tiers[:section_len]

            key_pattern = re.compile(rf"^([ \t]*{re.escape(story_key)}[ \t]*:[ \t]*)([^\n#]+)(.*)$", re.MULTILINE)
            match = key_pattern.search(tiers_content)
            if match:
                start_in_raw = tier_idx + match.start()
                end_in_raw = tier_idx + match.end()
                prefix = match.group(1)
                comment = match.group(3)
                new_line = f"{prefix}{tier}{comment}"
                self.raw_content = self.raw_content[:start_in_raw] + new_line + self.raw_content[end_in_raw:]
            else:
                next_nl = self.raw_content.find("\n", tier_idx)
                ins_pos = next_nl + 1 if next_nl != -1 else len(self.raw_content)
                entry = f"  {story_key}: {tier}\n"
                self.raw_content = self.raw_content[:ins_pos] + entry + self.raw_content[ins_pos:]

        self.path.write_text(self.raw_content, encoding="utf-8")
        self._load()
        return True

    def format_status_board(self) -> str:
        """Renders an ASCII status table."""
        lines = []
        lines.append(f"Sprint Status: {self.data.get('project', 'Unknown Project')}")
        lines.append("=" * 60)
        lines.append(f"{'Item / Story Key':<35} | {'Status':<12} | {'Tier':<8}")
        lines.append("-" * 60)

        dev_status = self.data.get("development_status", {})
        for key, status in dev_status.items():
            tier = self.get_tier(key) if not key.startswith("epic-") else "-"
            lines.append(f"{key:<35} | {status:<12} | {tier:<8}")
        lines.append("=" * 60)
        return "\n".join(lines)


def is_test_file(path: str) -> bool:
    """Checks if a file path belongs to a test suite."""
    name = Path(path).name.lower()
    return (
        name.startswith("test_")
        or name.endswith("_test.py")
        or name.endswith(".test.ts")
        or name.endswith(".spec.ts")
        or name.endswith(".test.js")
        or name.endswith(".spec.js")
        or "/tests/" in path
        or "/test/" in path
    )


def detect_test_tampering(diff_text: str) -> Dict[str, Any]:
    """Detects deleted or weakened test assertions in a git diff."""
    lines = diff_text.splitlines()
    is_current_test = False
    deleted_assertions = []

    for line in lines:
        if line.startswith("diff --git "):
            parts = line.split()
            current_file = parts[-1] if len(parts) >= 4 else ""
            is_current_test = is_test_file(current_file)
            continue

        if is_current_test and line.startswith("-") and not line.startswith("---"):
            trimmed = line[1:].strip()
            # Detect deletion of assertion statements or test cases
            if (
                trimmed.startswith("assert ")
                or trimmed.startswith("assert(")
                or "assert " in trimmed
                or trimmed.startswith("self.assert")
                or trimmed.startswith("expect(")
            ):
                deleted_assertions.append(trimmed)

    return {
        "tampered": len(deleted_assertions) > 0,
        "deleted_assertions": deleted_assertions,
    }


def validate_artifact_chain(story_dir: Path) -> Dict[str, Any]:
    """Validates presence and non-emptiness of intent.md, spec.md, and plan.md."""
    expected = ["intent.md", "spec.md", "plan.md"]
    missing = []

    for filename in expected:
        f = story_dir / filename
        if not f.exists() or len(f.read_text(encoding="utf-8").strip()) == 0:
            missing.append(filename)

    return {
        "valid": len(missing) == 0,
        "missing_artifacts": missing,
    }


def create_incident(
    ledger_path: Path,
    summary: str,
    tier: str = "flash",
    stories_dir: Optional[Path] = None,
) -> Dict[str, Any]:
    """Scaffolds an incident intent.md and registers in sprint ledger."""
    if stories_dir is None:
        stories_dir = ledger_path.parent / "docs" / "stories"

    incidents_dir = stories_dir / "incidents"
    incidents_dir.mkdir(parents=True, exist_ok=True)

    timestamp = int(time.time())
    key = f"INC-{timestamp}"

    intent_file = incidents_dir / f"{key}-intent.md"
    content = f"""# {key}: Incident Intent

## Stage 1: Intent
- **Incident Key:** {key}
- **Summary:** {summary}
- **Status:** ready-for-dev
- **Execution Tier:** {tier}

## Problem Description
{summary}

## Scope & Target
- Investigate root cause and write regression test reproducing the failure.
- Implement minimal fix following strict TDD.
- Verify zero regressions with anti-cheat verification.
"""
    intent_file.write_text(content, encoding="utf-8")

    # Register in ledger
    ledger = SprintLedger(ledger_path)
    ledger.update_status(key, "ready-for-dev")
    ledger.set_tier(key, tier)

    return {
        "key": key,
        "intent_file": intent_file,
        "summary": summary,
        "tier": tier,
    }


def get_git_diff(work_dir: Path) -> str:
    """Captures uncommitted or HEAD diff."""
    try:
        proc = subprocess.run(
            ["git", "diff", "HEAD"],
            cwd=str(work_dir),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        return proc.stdout
    except Exception:
        return ""


def run_verification(
    test_command: str,
    work_dir: Path,
    check_anti_cheat: bool = False,
) -> Dict[str, Any]:
    """Runs tests, captures exit code, and optionally executes anti-cheat inspection."""
    try:
        proc = subprocess.run(
            test_command,
            shell=True,
            cwd=str(work_dir),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=300,
        )
        test_passed = proc.returncode == 0
        anti_cheat_res = None

        if check_anti_cheat:
            diff_text = get_git_diff(work_dir)
            anti_cheat_res = detect_test_tampering(diff_text)
            if anti_cheat_res["tampered"]:
                test_passed = False

        return {
            "passed": test_passed,
            "test_exit_code": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "anti_cheat": anti_cheat_res,
        }
    except Exception as e:
        return {
            "passed": False,
            "test_exit_code": -1,
            "stdout": "",
            "stderr": str(e),
            "anti_cheat": None,
        }


def check_git_status(work_dir: Path) -> Dict[str, Any]:
    """Checks git status for uncommitted changes."""
    try:
        proc = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=str(work_dir),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        output = proc.stdout.strip()
        uncommitted = [line for line in output.splitlines() if line] if output else []
        return {
            "clean": len(uncommitted) == 0,
            "dirty": len(uncommitted) > 0,
            "uncommitted_files": uncommitted,
        }
    except Exception as e:
        return {
            "clean": False,
            "dirty": True,
            "uncommitted_files": [f"Error checking git: {e}"],
        }


def main():
    parser = argparse.ArgumentParser(description="myloop-lean sprint ledger manager & verification gate")
    parser.add_argument(
        "--ledger",
        default="sprint-status.yaml",
        help="Path to sprint-status.yaml",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # next
    subparsers.add_parser("next", help="Get next ready-for-dev story")

    # update
    update_p = subparsers.add_parser("update", help="Update story status")
    update_p.add_argument("story_key", help="Key of the story")
    update_p.add_argument("--status", required=True, choices=VALID_STATUSES, help="New status")

    # status
    subparsers.add_parser("status", help="Print sprint status board")

    # verify
    verify_p = subparsers.add_parser("verify", help="Run verification gate")
    verify_p.add_argument("--cmd", default="python3 -m pytest tests/", help="Test command to run")
    verify_p.add_argument("--anti-cheat", action="store_true", help="Enable anti-cheat test tampering detection")

    # incident
    inc_p = subparsers.add_parser("incident", help="Scaffold incident intent.md and register in sprint ledger")
    inc_p.add_argument("summary", help="Description of the incident or bug")
    inc_p.add_argument("--tier", default="flash", choices=["flash", "pro", "cheap", "standard", "frontier"])

    # validate-chain
    chain_p = subparsers.add_parser("validate-chain", help="Validate three-stage artifact chain for a story")
    chain_p.add_argument("story_dir", help="Path to story directory containing intent.md, spec.md, plan.md")

    # dispatch
    disp_p = subparsers.add_parser("dispatch", help="Dispatch story to heterogeneous models")
    disp_p.add_argument("story_key", help="Key of the story to dispatch")
    disp_p.add_argument("--persona", default="developer", help="Persona to dispatch (e.g. developer, reviewer)")
    disp_p.add_argument("--skill", default=None, help="Target skill to bundle")
    disp_p.add_argument("--provider", default=None, help="Provider override (deepseek, qwen, openai, local)")
    disp_p.add_argument("--model", default=None, help="Model override")
    disp_p.add_argument("--dry-run", action="store_true", help="Print request payload as JSON without sending")
    disp_p.add_argument("--export", default=None, help="Export compiled payload to file")

    args = parser.parse_args()

    ledger_path = Path(args.ledger)
    if args.command in ["next", "update", "status"]:
        if not ledger_path.exists():
            print(f"Error: ledger file '{ledger_path}' not found.", file=sys.stderr)
            sys.exit(1)
        ledger = SprintLedger(ledger_path)

    if args.command == "next":
        story = ledger.get_next_actionable_story()
        if story:
            print(f"NEXT_STORY={story['key']}")
            print(f"TIER={story['tier']}")
        else:
            print("NO_STORIES_READY")

    elif args.command == "update":
        try:
            ledger.update_status(args.story_key, args.status)
            print(f"Updated '{args.story_key}' to status '{args.status}'")
        except ValueError as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "status":
        print(ledger.format_status_board())

    elif args.command == "verify":
        work_dir = Path.cwd()
        print(f"Running verification with command: {args.cmd}")
        res = run_verification(args.cmd, work_dir, check_anti_cheat=args.anti_cheat)
        git_res = check_git_status(work_dir)

        if res["anti_cheat"] and res["anti_cheat"]["tampered"]:
            print("VERIFICATION FAILED: Anti-cheat detected test tampering!", file=sys.stderr)
            for d in res["anti_cheat"]["deleted_assertions"]:
                print(f"  Removed assertion: {d}", file=sys.stderr)
            sys.exit(1)

        if not res["passed"]:
            print("VERIFICATION FAILED: Tests exited non-zero.", file=sys.stderr)
            print(res["stdout"])
            print(res["stderr"], file=sys.stderr)
            sys.exit(1)

        print("Tests passed successfully.")
        if git_res["dirty"]:
            print("Warning: Uncommitted changes in working tree:")
            for f in git_res["uncommitted_files"][:5]:
                print(f"  {f}")
        else:
            print("Git working tree is clean.")
        print("VERIFICATION PASSED.")

    elif args.command == "incident":
        inc = create_incident(ledger_path, args.summary, tier=args.tier)
        print(f"Incident created: {inc['key']}")
        print(f"Intent file: {inc['intent_file']}")
        print(f"Registered in sprint ledger with tier '{inc['tier']}'")

    elif args.command == "validate-chain":
        story_dir = Path(args.story_dir)
        res = validate_artifact_chain(story_dir)
        if res["valid"]:
            print(f"Artifact chain valid for: {story_dir}")
        else:
            print(f"Artifact chain incomplete! Missing: {res['missing_artifacts']}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "dispatch":
        from dispatch import ProviderRegistry, compile_prompt_context, OpenAICompatibleClient
        work_dir = Path.cwd()
        ledger = SprintLedger(ledger_path) if ledger_path.exists() else None
        tier = ledger.get_tier(args.story_key) if ledger else "flash"
        registry = ProviderRegistry()
        provider, model, base_url = registry.resolve(
            persona=args.persona,
            tier=tier,
            provider_override=args.provider,
            model_override=args.model,
        )
        skill = args.skill
        if not skill:
            if args.persona in ["developer"]:
                skill = "build"
            elif args.persona in ["reviewer"]:
                skill = "code-review"
        context = compile_prompt_context(
            workspace_root=work_dir,
            story_key=args.story_key,
            persona=args.persona,
            skill_name=skill,
        )
        api_key_env_map = {
            "deepseek": "DEEPSEEK_API_KEY",
            "openai": "OPENAI_API_KEY",
            "qwen": "DASHSCOPE_API_KEY",
            "gemini": "GEMINI_API_KEY",
        }
        api_key = os.environ.get(api_key_env_map.get(provider, "OPENAI_API_KEY"), "")
        client = OpenAICompatibleClient(base_url=base_url, api_key=api_key)
        payload = client.format_payload(
            model=model,
            system_prompt=context["system_prompt"],
            user_message=context["user_message"],
        )
        if args.export:
            export_path = Path(args.export)
            export_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
            print(f"Payload exported to {export_path}")
        if args.dry_run:
            print(json.dumps(payload, indent=2))
            sys.exit(0)
        if not api_key and provider != "local":
            print(f"Error: API key for provider '{provider}' not set in environment.", file=sys.stderr)
            sys.exit(1)
        print(f"Dispatching story '{args.story_key}' to provider '{provider}' (model: {model})...")
        res = client.send_completion(payload)
        content = res.get("choices", [{}])[0].get("message", {}).get("content", "")
        print(content)


if __name__ == "__main__":
    main()
