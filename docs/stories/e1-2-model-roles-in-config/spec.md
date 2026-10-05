# Story Specification: Model roles in config

## 1. Functional Specification
### 1. Configuration Schema (`config.toml`)
```toml
[models.roles.mutator]
provider = "anthropic"
model_id = "claude-3-5-haiku-20241022"
max_tokens = 4096
temperature = 0.4
cost_per_million_input = 0.80
cost_per_million_output = 4.00

[models.roles.synthesiser]
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"
max_tokens = 8192
temperature = 0.2
cost_per_million_input = 3.00
cost_per_million_output = 15.00

[models.roles.adversary]
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"
max_tokens = 4096
temperature = 0.7
cost_per_million_input = 3.00
cost_per_million_output = 15.00
```

### 2. Validation Rules
- All three roles are mandatory; omitting one aborts startup with the role named.
- `model_id` must be non-empty and must not contain whitespace.
- `temperature` must lie in $[0.0, 2.0]$; `max_tokens` must be $> 0$.
- Cost fields must be $\ge 0.0$ because they feed the AD-6 budget projection in e1-10.

### 3. Hot-Reload via SIGHUP
- Upon receiving `SIGHUP`, the daemon re-parses `config.toml`.
- If valid, the runtime atomic reference `ArcSwap<ModelConfig>` is swapped immediately.
- If invalid, the error is logged to stderr, the existing configuration remains active, and the daemon stays healthy.
- A swap is never observable as a torn read: in-flight calls keep the config snapshot they started with.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `config.toml` with all three roles populated | `ModelConfig` usable by dispatch, service starts | `ConfigError::MissingRole` naming the role |
| Role with empty `model_id` or temperature outside `[0.0, 2.0]` | Startup abort before any job is accepted | `ConfigError::InvalidField` naming role and field |
| `SIGHUP` with valid edited config | Next generation reads new settings, no restart | Reload logged at info level |
| `SIGHUP` with malformed config | Previous config stays active, daemon healthy | Parse error logged to stderr, no swap |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given config maps `mutator`, `synthesiser` and `adversary` to model ID, max tokens and temperature**, **when a job runs**, **then each dispatch uses its configured role settings**.
- **Given a mandatory role is omitted or its model ID is empty**, **when the service starts**, **then it fails loudly with the offending role named**.
- **Given I edit config.toml and send SIGHUP**, **when the next generation starts**, **then the new settings apply seamlessly without daemon restart**.
- **Given I send SIGHUP with malformed TOML**, **when the daemon handles the signal**, **then the previous config stays active and the daemon remains healthy**.
