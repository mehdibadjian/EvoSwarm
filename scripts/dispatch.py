#!/usr/bin/env python3
"""Model-Agnostic Story Dispatch Engine for myloop-lean.

Compiles persona rules, system invariants, skill instructions, and story artifacts
into standardized OpenAI-compatible payloads for heterogeneous models (Gemini,
DeepSeek-R1, Qwen 2.5 Coder, OpenAI, and local Ollama/vLLM instances).
"""

import argparse
import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path
from typing import Any, Dict, Optional, Tuple


class ProviderRegistry:
    """Registry and resolver for LLM providers and models."""

    DEFAULT_ENDPOINTS = {
        "deepseek": "https://api.deepseek.com/v1",
        "qwen": "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        "openai": "https://api.openai.com/v1",
        "local": "http://localhost:11434/v1",
        "gemini": "https://generativelanguage.googleapis.com/v1beta/openai/",
    }

    PERSONA_DEFAULTS = {
        "architect": ("deepseek", "deepseek-reasoner"),
        "fortress-architect": ("deepseek", "deepseek-reasoner"),
        "reviewer": ("deepseek", "deepseek-reasoner"),
        "developer": ("qwen", "qwen2.5-coder-32b-instruct"),
        "qa-engineer": ("qwen", "qwen2.5-coder-32b-instruct"),
        "velocity-king": ("qwen", "qwen2.5-coder-32b-instruct"),
        "product-manager": ("deepseek", "deepseek-chat"),
    }

    TIER_DEFAULTS = {
        "pro": ("deepseek", "deepseek-reasoner"),
        "frontier": ("deepseek", "deepseek-reasoner"),
        "flash": ("qwen", "qwen2.5-coder-32b-instruct"),
        "cheap": ("qwen", "qwen2.5-coder-32b-instruct"),
        "standard": ("openai", "gpt-4o"),
    }

    def resolve(
        self,
        persona: Optional[str] = None,
        tier: str = "flash",
        provider_override: Optional[str] = None,
        model_override: Optional[str] = None,
        base_url_override: Optional[str] = None,
    ) -> Tuple[str, str, str]:
        """Resolves provider, model, and base URL."""
        # 1. Base resolution by persona or tier
        provider = None
        model = None

        if persona and persona.lower() in self.PERSONA_DEFAULTS:
            provider, model = self.PERSONA_DEFAULTS[persona.lower()]
        elif tier.lower() in self.TIER_DEFAULTS:
            provider, model = self.TIER_DEFAULTS[tier.lower()]
        else:
            provider = "qwen"
            model = "qwen2.5-coder-32b-instruct"

        # 2. Overrides
        if provider_override:
            provider = provider_override
        if model_override:
            model = model_override

        # 3. Base URL resolution
        if base_url_override:
            base_url = base_url_override
        elif provider == "local" or os.environ.get("OPENAI_BASE_URL"):
            base_url = os.environ.get("OPENAI_BASE_URL", self.DEFAULT_ENDPOINTS["local"])
        else:
            base_url = self.DEFAULT_ENDPOINTS.get(provider, self.DEFAULT_ENDPOINTS["openai"])

        return provider, model, base_url


def compile_prompt_context(
    workspace_root: Path,
    story_key: str,
    persona: str = "developer",
    skill_name: Optional[str] = None,
) -> Dict[str, str]:
    """Compiles rules, persona instructions, skill runbooks, and story artifacts."""
    system_sections = []

    # 1. Persona System Role
    agents_md = workspace_root / "AGENTS.md"
    if agents_md.exists():
        system_sections.append(f"## Orchestration Context (from AGENTS.md)\n{agents_md.read_text(encoding='utf-8').strip()}")

    system_sections.append(f"## Active Persona\nYou are acting as the {persona.upper()}. Follow persona responsibilities strictly.")

    # 2. Universal Rules from .agents/rules/*.md
    rules_dir = workspace_root / ".agents" / "rules"
    if rules_dir.exists():
        for rule_file in sorted(rules_dir.glob("*.md")):
            content = rule_file.read_text(encoding="utf-8").strip()
            system_sections.append(f"## Rule: {rule_file.name}\n{content}")

    # 3. Skill Runbook
    if skill_name:
        skill_file = workspace_root / ".agents" / "skills" / skill_name / "SKILL.md"
        if skill_file.exists():
            content = skill_file.read_text(encoding="utf-8").strip()
            system_sections.append(f"## Active Skill Runbook: {skill_name}\n{content}")

    system_prompt = "\n\n".join(system_sections)

    # 4. User Message with Story Context & Artifacts
    user_sections = [f"# Task Dispatch for Story: {story_key}"]

    story_dir = workspace_root / "docs" / "stories" / story_key
    story_single = workspace_root / "docs" / "stories" / f"{story_key}.md"

    if story_dir.is_dir():
        for art in ["intent.md", "spec.md", "plan.md"]:
            art_file = story_dir / art
            if art_file.exists():
                user_sections.append(f"## Artifact: {art}\n{art_file.read_text(encoding='utf-8').strip()}")
    elif story_single.exists():
        user_sections.append(f"## Story Specification\n{story_single.read_text(encoding='utf-8').strip()}")
    else:
        user_sections.append(f"Execute lifecycle task for story key '{story_key}'.")

    user_message = "\n\n".join(user_sections)

    return {
        "system_prompt": system_prompt,
        "user_message": user_message,
    }


class OpenAICompatibleClient:
    """Lightweight, zero-dependency client for OpenAI-compatible APIs."""

    def __init__(self, base_url: str, api_key: Optional[str] = None):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key or ""

    def format_headers(self) -> Dict[str, str]:
        headers = {
            "Content-Type": "application/json",
        }
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def format_payload(
        self,
        model: str,
        system_prompt: str,
        user_message: str,
        temperature: float = 0.2,
    ) -> Dict[str, Any]:
        return {
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message},
            ],
            "temperature": temperature,
        }

    def send_completion(self, payload: Dict[str, Any], timeout: int = 60) -> Dict[str, Any]:
        endpoint = f"{self.base_url}/chat/completions"
        req_data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            endpoint,
            data=req_data,
            headers=self.format_headers(),
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=timeout) as response:
                res_body = response.read().decode("utf-8")
                return json.loads(res_body)
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8") if e.fp else str(e)
            raise RuntimeError(f"HTTP Error {e.code}: {err_body}") from e
        except Exception as e:
            raise RuntimeError(f"Network error communicating with {endpoint}: {e}") from e


def main():
    parser = argparse.ArgumentParser(description="Model-Agnostic Dispatch Engine for myloop-lean")
    parser.add_argument("story_key", help="Key of the story to dispatch")
    parser.add_argument("--persona", default="developer", help="Persona to dispatch (e.g. developer, reviewer, architect)")
    parser.add_argument("--tier", default="flash", help="Execution tier (flash, pro, frontier)")
    parser.add_argument("--skill", default=None, help="Target skill to bundle (e.g. build, code-review)")
    parser.add_argument("--provider", default=None, help="Provider override (deepseek, qwen, openai, local)")
    parser.add_argument("--model", default=None, help="Model override")
    parser.add_argument("--dry-run", action="store_true", help="Print request payload as JSON without sending")
    parser.add_argument("--export", default=None, help="Export compiled request payload to a file")

    args = parser.parse_args()
    workspace_root = Path.cwd()

    # Resolve model and provider
    registry = ProviderRegistry()
    provider, model, base_url = registry.resolve(
        persona=args.persona,
        tier=args.tier,
        provider_override=args.provider,
        model_override=args.model,
    )

    # Determine default skill if not specified
    skill = args.skill
    if not skill:
        if args.persona in ["developer"]:
            skill = "build"
        elif args.persona in ["reviewer"]:
            skill = "code-review"

    # Compile prompt context
    context = compile_prompt_context(
        workspace_root=workspace_root,
        story_key=args.story_key,
        persona=args.persona,
        skill_name=skill,
    )

    # Prepare client and payload
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

    # Send completion if not dry run
    if not api_key and provider != "local":
        print(f"Error: API key for provider '{provider}' not set in environment.", file=sys.stderr)
        sys.exit(1)

    print(f"Dispatching story '{args.story_key}' to provider '{provider}' (model: {model})...")
    res = client.send_completion(payload)
    print("Response received:")
    content = res.get("choices", [{}])[0].get("message", {}).get("content", "")
    print(content)


if __name__ == "__main__":
    main()
