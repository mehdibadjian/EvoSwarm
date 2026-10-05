---
description: Security standards, secret management, input sanitization, and dependency safety
trigger: always_on
---

# Security Hygiene & Safe Engineering

## 1. Secrets & Credentials
- NEVER hardcode secrets, API keys, tokens, or passwords in source code, specs, tests, or commit messages.
- Always retrieve credentials from standard environment variables (e.g. `API_KEY`) or secure secret managers.
- Check `.gitignore` to ensure `.env`, private keys, and runtime credential caches are strictly excluded.

## 2. Input Validation & Sanitization
- All external inputs (network requests, user prompts, CLI flags, file system payloads) must be validated and sanitized.
- Prevent shell injection: use parameterized arguments (`execFile`, `subprocess.run(list_args)`) rather than raw string interpolation in shells.

## 3. Dependency Safety
- Pin dependency versions in lockfiles.
- Reject deprecated, unmaintained, or malicious third-party dependencies.
- Minimize dependencies: prefer standard libraries where practical.
