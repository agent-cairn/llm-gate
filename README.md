# llm-gate

[![CI](https://github.com/agent-cairn/llm-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/agent-cairn/llm-gate/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Transparent HTTP proxy for LLM APIs with budget enforcement, per-agent spending limits, and NDJSON audit trails. Built for AI agent systems that need cost control without modifying agent code.

## Why

AI agents make LLM calls autonomously. Without guardrails:
- A runaway loop can spend hundreds of dollars before anyone notices
- There's no audit trail when something goes wrong
- Per-agent cost attribution is impossible

`llm-gate` sits between your agent and the LLM API. It's transparent — agents need no code changes — and adds:
- Hard spending limits (block when exceeded) or soft limits (warn and log)
- Per-label cost tracking with persistent state
- NDJSON audit log: timestamp, model, tokens, cost, status

## Architecture

```
  Agent / Client
       │
       ▼
  ┌─────────────┐
  │  llm-gate   │  ← budget check (pre-flight)
  │   proxy     │
  │  :7777      │
  └──────┬──────┘
         │ forward request
         ▼
  ┌─────────────┐
  │  LLM API    │  (Anthropic / OpenAI / Gemini)
  └──────┬──────┘
         │ response + usage
         ▼
  ┌─────────────┐
  │  llm-gate   │  ← extract tokens, compute cost
  │             │  ← record spend, update budget
  │             │  ← write audit event (NDJSON)
  └──────┬──────┘
         │
         ▼
  Agent / Client
```

## Quick Start

```bash
# Install
cargo install --path llm-gate

# Run proxy (blocks at $5.00, logs to audit.ndjson)
llm-gate proxy \
  --listen 127.0.0.1:7777 \
  --target https://api.anthropic.com \
  --label my-agent \
  --budget 5.00 \
  --audit audit.ndjson

# Point your agent at the proxy instead of the real API:
# ANTHROPIC_BASE_URL=http://127.0.0.1:7777

# Check budget status
llm-gate budget --config budgets.json status

# Add a budget manually
llm-gate budget --config budgets.json add my-agent 10.00 --action warn

# Reset a budget counter
llm-gate budget --config budgets.json reset my-agent

# Tail the audit log
llm-gate audit audit.ndjson --tail 50
```

## Configuration

### Proxy flags

| Flag | Default | Description |
|------|---------|-------------|
| `--listen` | `127.0.0.1:7777` | Address to bind |
| `--target` | `https://api.anthropic.com` | Upstream LLM API base URL |
| `--label` | `default` | Budget/audit label for this proxy |
| `--budget` | `0.0` | Spending limit in USD (0 = no limit) |
| `--audit` | stdout | Path to NDJSON audit log file |
| `--config` | none | Path to persistent budgets JSON file |

### Budget actions

- `block` — return HTTP 429 when limit is exceeded (default)
- `warn` — log a warning but allow the request through

### Supported models (auto-detected from request body)

| Provider | Models |
|----------|--------|
| Anthropic | claude-3-5-sonnet, claude-3-5-haiku, claude-3-opus, claude-3-sonnet, claude-3-haiku, claude-opus-4 |
| OpenAI | gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4, gpt-3.5, o1, o1-mini |
| Google | gemini-1.5-flash, gemini-1.5-pro, gemini-2.0-flash |

Unknown models pass through with `cost_usd: 0.0`.

## Audit Log Format

Each line is a JSON object:

```json
{"timestamp":"2025-01-15T10:23:45.123Z","label":"my-agent","model":"claude-3-5-sonnet-20241022","provider":"anthropic","input_tokens":1200,"output_tokens":450,"cost_usd":0.01035,"status":"ok"}
```

## Workspace Structure

```
llm-gate/
├── gate/           # Core library: pricing, budget, audit
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── pricing.rs
│       ├── budget.rs
│       └── audit.rs
└── llm-gate/       # CLI binary: proxy + budget commands
    └── src/
        └── main.rs
```

## License

MIT — see [LICENSE](LICENSE)
