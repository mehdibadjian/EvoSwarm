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

### 2. Hot-Reload via SIGHUP
- Upon receiving `SIGHUP`, the daemon parses `config.toml`.
- If valid, the runtime atomic reference `ArcSwap<ModelConfig>` is swapped immediately.
- If invalid, the error is logged to stderr, the existing configuration remains active, and the daemon stays healthy.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given config maps `mutator`, `synthesiser` and `adversary` to model ID, max tokens and temperature**, **when a job runs**, **then each dispatch uses its configured role settings.**.
- **Given an unknown or malformed model ID**, **when the service starts**, **then it fails loudly with the offending role named.**.
- **Given I edit config.toml and send SIGHUP**, **when the next generation starts**, **then the new settings apply seamlessly without daemon restart.**.
