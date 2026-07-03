# AI Terminal Agent Landscape — 2026 Complete Reference

A comprehensive map of every AI coding agent that runs in the terminal. Updated July 2026.

---

## The Big Four (Frontier Labs)

These are the terminal agents built by the companies that train the frontier models. They define the category.

### 1. Claude Code (Anthropic)

| Attribute | Value |
|-----------|-------|
| **Stars** | 79,500+ (proprietary, source-available) |
| **Language** | TypeScript |
| **Model** | Claude Opus 4.6–4.8, Sonnet 4.6 (locked to Anthropic) |
| **Context** | 1M tokens (Opus) |
| **Pricing** | $20/mo Pro, $100/mo Max, $200/mo Max 20x |
| **Install** | `npm i -g @anthropic-ai/claude-code` |
| **SWE-bench** | 80.8% (highest among CLI tools) |
| **Terminal-Bench** | 78.9% (Opus 4.8) |

**Key Differentiators:**
- Created MCP (Model Context Protocol) — now an industry standard
- Plan Mode: plans before executing, human-in-the-loop review
- Agent Teams: spawns sub-agents for parallel work on independent tasks
- SKILL.md system: teach Claude specific behaviors with markdown skill files
- Deepest permission system: read-only by default, explicit opt-in for destructive ops
- Jupyter notebook editing, background agents, hooks system
- Used by 115,000+ developers, 195M lines of code written per week

**Best For:** Deep codebase work, autonomous multi-step tasks, developers already paying for Claude.

**Weakness:** Locked to Anthropic models. No local model support. No free tier.

---

### 2. Codex CLI (OpenAI)

| Attribute | Value |
|-----------|-------|
| **Stars** | 62,000+ (Apache 2.0) |
| **Language** | Rust (rewritten from TS mid-2025) |
| **Model** | GPT-5.5, GPT-5.4, codex-mini (locked to OpenAI) |
| **Context** | 2M tokens (GPT-5.5) |
| **Pricing** | Free OSS + API costs; bundled with ChatGPT Plus/Pro |
| **Install** | `npm i -g @openai/codex` |
| **SWE-bench** | — |
| **Terminal-Bench** | **83.4% (#1 overall)** |

**Key Differentiators:**
- Fastest startup (~50ms, Rust binary)
- Sandboxed execution with approval policies (Suggest, Auto Edit, Full Auto)
- Parallel multi-agent: multiple agents work in isolated git worktrees simultaneously
- Cloud sandbox: per-task isolated containers with repo snapshots
- AGENTS.md support for per-project agent configuration
- MCP integration, web search, local sandboxing

**Best For:** Developers in the OpenAI ecosystem, Rust performance enthusiasts, multi-agent parallel workflows.

**Weakness:** Primarily optimized for OpenAI models. Cloud sandbox requires network.

---

### 3. Gemini CLI (Google)

| Attribute | Value |
|-----------|-------|
| **Stars** | 105,641 (Apache 2.0 → closed-source June 2026) |
| **Language** | TypeScript |
| **Model** | Gemini 2.5 Pro, Gemini 3.1 Pro (locked to Google) |
| **Context** | 1M tokens (native, best-in-class) |
| **Pricing** | **Free** (1000 req/day with Google account) |
| **Install** | `npm i -g @google/gemini-cli` |
| **Terminal-Bench** | 70.7% (Gemini 3.1 Pro) |

**Key Differentiators:**
- Most generous free tier: 1000 requests/day with just a Google account
- 1M token context window (native, no compression needed)
- Search grounding: can search the web in real-time during tasks
- Multimodal: can generate apps from PDFs or sketches
- Extensible via MCP

**⚠️ June 18, 2026: Free individual access ended.** Google moved Gemini CLI to closed-source under "Antigravity CLI." Existing open-source repo remains but is frozen.

**Best For:** Developers who want a free capable agent, Google ecosystem users, large-context tasks.

**Weakness:** No local model support. Transition to closed-source creates uncertainty.

---

### 4. OpenCode (Anomaly / SST)

| Attribute | Value |
|-----------|-------|
| **Stars** | **180,000+** (MIT) — most-starred AI coding agent |
| **Language** | Go (TUI via Bubble Tea) |
| **Model** | **75+ providers** via Models.dev (Claude, GPT, Gemini, Ollama, local) |
| **Context** | Provider-dependent |
| **Pricing** | **Free + BYOK** (pay only model API costs) |
| **Install** | `curl -fsSL https://opencode.ai/install \| bash` |
| **Users** | 5M+ monthly, 650K+ monthly active |

**Key Differentiators:**
- Provider-agnostic: Claude, GPT, Gemini, Ollama, DeepSeek, Groq, 75+ total
- Multi-session TUI: run multiple parallel agent sessions
- LSP-native: deep Language Server Protocol integration for type-safe edits
- Plan mode + Build mode: separate planning from execution
- Custom agents: define your own agent behaviors
- Desktop app + VS Code extension + CLI (3 surfaces)
- 10 parallel agents with Oh-My-OpenAgent extension
- 120M+ npm downloads monthly

**Best For:** Model-flexible developers, open-source enthusiasts, terminal-first workflows, privacy-conscious users.

**Weakness:** Requires API keys for most providers. Less opinionated = more configuration.

---

## Tier 2: Major Open-Source Agents

### 5. Aider (Paul Gauthier)

| Attribute | Value |
|-----------|-------|
| **Stars** | 45,945 (Apache 2.0) |
| **Language** | Python |
| **Model** | Any (Claude, GPT, DeepSeek, Ollama, local) |
| **Pricing** | Free + BYOK |
| **Install** | `pip install aider-chat` or `curl -LsSf https://aider.chat/install.sh \| sh` |

**Key Differentiators:**
- Git-native: every change auto-commits with descriptive messages
- Tree-sitter repomap: understands code structure across 100+ languages
- Multi-file coordinated edits with architectural understanding
- Original CLI coding agent (predates Claude Code)
- Model flexibility: works with Claude, GPT, DeepSeek, Ollama, local models
- Easy undo: `git reset` to any checkpoint

**Best For:** Git-centric workflows, developers who want full control, pair-programming style.

**Weakness:** Development cadence slowed (no tagged release since Aug 2025). Single maintainer. No TUI.

---

### 6. Cline (Cline Bot Inc.)

| Attribute | Value |
|-----------|-------|
| **Stars** | 63,501 (Apache 2.0) |
| **Language** | TypeScript |
| **Surface** | VS Code, JetBrains, **CLI 2.0+**, Cursor, Windsurf, Neovim |
| **Installs** | 5M+ (most-installed AI coding extension) |
| **Pricing** | Free + BYOK |
| **Install** | `npm i -g cline` |

**Key Differentiators:**
- Plan/Act modes: Tab toggles planning vs execution
- MCP Marketplace: browse and install extensions
- Built-in browser automation (Puppeteer) — can interact with web pages
- Checkpoints: shadow Git for instant rollback
- 30+ LLM providers including local (Ollama, LM Studio)
- CLI 2.0: headless mode, YOLO mode, stdin/stdout piping for CI/CD
- Rules system + SKILL.md support
- Parallel agents with state isolation

**Best For:** VS Code users wanting agent capabilities, developers needing browser automation, MCP ecosystem.

**Weakness:** VS Code heritage means CLI is secondary (but improving fast). Approval prompts can be noisy.

---

### 7. Goose (Block / Linux Foundation AAIF)

| Attribute | Value |
|-----------|-------|
| **Stars** | 48,542 (Apache 2.0) |
| **Language** | Rust (58%) + TypeScript (34%) |
| **Surface** | CLI + Desktop app + API |
| **Pricing** | Free + BYOK |
| **Install** | `curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh \| bash` |

**Key Differentiators:**
- **Linux Foundation governance** (donated Dec 2025 to AAIF) — vendor-neutral
- General-purpose: not just code, also research, writing, automation, data analysis
- 70+ MCP extensions built-in
- ACP (Agent Client Protocol) support: can reuse Claude/ChatGPT/Gemini subscriptions
- 25+ LLM providers
- Recipe system: YAML-based workflow macros
- macOS sandbox integration, multi-agent orchestration (Goosetown)

**Best For:** Non-code automation, multi-provider workflows, enterprise wanting foundation governance.

**Weakness:** SWE-bench ~45% (27pt gap behind Claude Code). Code features lag behind code-first tools.

---

### 8. Kilo Code

| Attribute | Value |
|-----------|-------|
| **Stars** | 19,968 (MIT) |
| **Surface** | VS Code, JetBrains |
| **Pricing** | Free + BYOK; Kilo Pass from $19/mo |
| **Install** | VS Code marketplace |

**Key Differentiators:**
- Fork of Cline with added Orchestrator mode, Memory Bank, voice commands
- No-markup BYOK: pay exact provider rates through Kilo Gateway
- JetBrains support alongside VS Code
- Cloud agents, voice commands, marketplace

**Best For:** Cline users wanting more features, JetBrains users, cost-transparent BYOK.

**Weakness:** Smaller community. Fork means upstream merge challenges.

---

## Tier 3: Startup & Niche Agents

### 9. Mistral Vibe (Mistral AI)

| Attribute | Value |
|-----------|-------|
| **Stars** | ~15K (Apache 2.0) |
| **Surface** | CLI + VS Code |
| **Model** | Mistral models + BYOK |
| **Pricing** | Free + BYOK |
| **Install** | `npm i -g @mistralai/vibe` |

French lab's entry. Clean TUI, focuses on Mistral's code models. Good for European developers wanting GDPR-compliant AI.

---

### 10. Qwen Code (Alibaba)

| Attribute | Value |
|-----------|-------|
| **Stars** | 25K (Apache 2.0) |
| **Language** | TypeScript (fork of Gemini CLI) |
| **Model** | Qwen3-Coder + BYOK |
| **Pricing** | Free + BYOK |
| **Install** | `npm i -g @qwen-code/qwen-code` |

Fork of Google Gemini CLI optimized for Qwen-Coder models. Ships with strong open-weight models. Good for developers wanting open-weight models with a polished CLI.

---

### 11. Kimi CLI (Moonshot AI)

| Attribute | Value |
|-----------|-------|
| **Stars** | 5,900 (Apache 2.0) |
| **Language** | Python |
| **Pricing** | Free + BYOK |

First Chinese lab with a dedicated CLI agent. Focuses on Moonshot's Kimi models.

---

### 12. MiMo-Code (Xiaomi)

| Attribute | Value |
|-----------|-------|
| **Stars** | New (2026) |
| **Language** | — |
| **Model** | MiMo-V2.5-Pro |
| **Pricing** | Free + BYOK (via OpenRouter) |

Xiaomi's open-source entry. Very low token pricing ($0.435/M input). Newcomer but well-funded.

---

## Tier 4: CI/CD & Platform Agents

### 13. GitHub Copilot CLI

| Attribute | Value |
|-----------|-------|
| **Stars** | 8K (proprietary) |
| **Language** | Shell/Go |
| **Pricing** | $10/mo Pro, usage-based billing since June 2026 |
| **Install** | `npm i -g @github/copilot` |

GitHub's terminal agent. Tight GitHub integration, MCP support, `/model` command. Switched to usage-based billing June 2026 (credits). Good for teams deep in GitHub ecosystem.

---

### 14. Amp (Sourcegraph)

| Attribute | Value |
|-----------|-------|
| **Stars** | Proprietary |
| **Pricing** | Free beta |
| **Model** | Claude Opus 4.7, Sonnet 4.6, GPT-5 |

Sourcegraph's autonomous CLI agent. Currently free beta. Routes through Sourcegraph's Cody infrastructure. Different from Cody (in-editor assistant); Amp is autonomous CLI.

---

### 15. Continue (`cn`)

| Attribute | Value |
|-----------|-------|
| **Stars** | 33K (Apache 2.0) |
| **Surface** | IDE + CLI (`cn`) |
| **Pricing** | $3/M tokens Starter, $20/seat Team |

Pivoted to "Continuous AI" — running agents on every PR as CI status checks. The CLI (`cn`) still exists but the focus is now CI checks defined as markdown in `.continue/checks/`.

---

### 16. Crush (Charmbracelet)

| Attribute | Value |
|-----------|-------|
| **Stars** | 25K (FSL) |
| **Language** | Go |
| **Pricing** | Free + BYOK |

Beautiful Go TUI from the team behind Bubble Tea (same TUI framework OpenCode uses). Multi-model with mid-session switching, LSP and MCP support. The pick if terminal aesthetics matter.

---

### 17. Trae Agent (ByteDance)

| Attribute | Value |
|-----------|-------|
| **Stars** | 10,700 (MIT) |
| **Language** | Python |
| **Benchmark** | SOTA on SWE-bench Verified |

ByteDance's entry. Claims state-of-the-art on SWE-bench Verified. Python-based.

---

### 9. Grok Build (xAI)

| Attribute | Value |
|-----------|-------|
| **Stars** | New (May 2026) |
| **Language** | Go |
| **Model** | grok-build-0.1 (256K context), custom model routing via OpenRouter |
| **Pricing** | SuperGrok Heavy only (~$300/mo, $99/mo intro) |
| **Install** | `curl -fsSL https://x.ai/cli/install.sh \| bash` |
| **Binary** | `grok` |

**Key Differentiators:**
- Parallel subagents in isolated git worktrees (8-way parallel)
- Plan mode with review/approve/comment/rewrite before execution
- AGENTS.md, plugins, hooks, skills, MCP servers out of the box
- Headless `-p` mode for scripts/bots/automation
- ACP (Agent Client Protocol) support for custom bots
- Plugin Marketplace for sharing capabilities
- Built-in `/feedback` command
- `/skillify` — capture any session as a reusable skill

**Best For:** xAI ecosystem users, developers wanting parallel subagent workflows.

**Weakness:** Very expensive ($300/mo). Early beta with rough edges. Terminal-only, no IDE/desktop surface. 17pt behind on SWE-bench vs Opus 4.8.

---

### 10. Pi (earendil-works)

| Attribute | Value |
|-----------|-------|
| **Stars** | 54,000+ (rapid growth) |
| **Language** | TypeScript (monorepo of npm packages) |
| **Model** | 15+ providers (Anthropic, OpenAI, Google, xAI, Ollama, Groq, etc.) |
| **Pricing** | Free + BYOK; supports subscription login (Claude Pro, ChatGPT Plus, GitHub Copilot) |
| **Install** | `npm i -g @earendil-works/pi-coding-agent` or `curl -fsSL https://pi.dev/install.sh \| sh` |
| **Binary** | `pi` |

**Key Differentiators:**
- Radical minimalism: 4 tools (Read, Write, Edit, Bash), ~1,000 token system prompt
- Self-extending: ask Pi to build its own extensions/skills by writing code
- Tree-structured session history with branching and sharing
- Switch models mid-session with `/model` or `Ctrl+L`
- Extensions, Skills, Prompt Templates, Themes — all written by the agent itself
- 4 modes: interactive, print/JSON, RPC, SDK
- No MCP dependency — agent writes its own tools via CDP, etc.

**Best For:** Developers who want minimal, hackable agents. Python/Rust/Go power users (created by Flask's Armin Ronacher).

**Weakness:** No plan mode, no sub-agents built-in. Requires TypeScript/Node ecosystem. Philosophy of "agent writes its own tools" is powerful but not for everyone.

---

### 11. Plandex

| Attribute | Value |
|-----------|-------|
| **Stars** | 11,000 (Apache 2.0) |
| **Language** | Go |
| **Model** | Multi-model: Anthropic, OpenAI, Google, open-source hybrid pipelines |
| **Pricing** | Free + BYOK (self-hosted); Cloud plan available |
| **Context** | Up to 2M tokens, 20M+ token indexing |
| **Install** | `curl -fsSL https://plandex.ai/install.sh \| sh` |

**Key Differentiators:**
- Built for large codebases: 2M token context, tree-sitter project mapping
- Diff review sandbox: review every change before applying
- Auto mode (full autonomy) or step-by-step mode (manual oversight)
- Version control for AI changes with branching
- Multi-model pipelines: combine models for speed/cost/quality

**Best For:** Large codebases, complex multi-file refactors, developers wanting diff-based control.

**Weakness:** Slower development cadence. Smaller community. No TUI. Cloud service winding down.

---

### 12. Amazon Q Developer CLI (AWS)

| Attribute | Value |
|-----------|-------|
| **Stars** | 9K (proprietary) |
| **Language** | Go/Shell |
| **Model** | Claude 3.7 Sonnet via Bedrock (agentic), multiple models for chat |
| **Pricing** | Free tier (50 agentic req/mo), Pro $19/user/mo (1,000 req/mo) |
| **Install** | `brew install amazon-q` or `curl -fsSL https://amazonq.dev/install \| bash` |
| **Binary** | `q` |

**Key Differentiators:**
- AWS-native: deep knowledge of AWS services, IAM, CLI, infrastructure
- IAM Identity Center auth — inherits existing AWS permissions
- `q chat` for agentic coding, `q translate` for natural-language → bash
- MCP support, image support, conversation persistence
- Code transformation (Java/.NET upgrades), vulnerability scanning
- Ghost-text autocompletion in shell
- Agentic: reads/writes files, runs commands, generates code diffs

**Best For:** AWS-centric teams and developers. Platform engineers managing cloud infrastructure.

**Weakness:** Free tier very limited (50 req/mo). Model choices limited to Bedrock. AWS lock-in.

---

### 13. Google Antigravity CLI

| Attribute | Value |
|-----------|-------|
| **Stars** | Successor to Gemini CLI (105K stars, but repo frozen) |
| **Language** | Go (rewritten from TypeScript) |
| **License** | Closed-source |
| **Model** | Gemini 3.5 Flash (default), Gemini 3.1 Pro |
| **Pricing** | Free tier (weekly quotas, less generous than Gemini CLI); AI Pro $19.99/mo; Ultra $99.99/mo |
| **Install** | `curl -fsSL https://antigravity.google/cli/install.sh \| bash` |
| **Binary** | `agy` |
| **Terminal-Bench** | 76.2% (Gemini 3.5 Flash) |

**Key Differentiators:**
- Multi-agent orchestration (not single-agent like Gemini CLI)
- Async workflows: background agents for large refactors
- Plugin import from Gemini CLI, MCP, Skills, Hooks preserved
- Unified harness with Antigravity 2.0 (desktop), Antigravity IDE, Antigravity SDK
- Faster Go binary, terminal sandboxing, credential masking
- Subagents for parallel tasks

**⚠️ Replaced Gemini CLI on June 18, 2026.** Free individual access ended. Enterprise licenses retain old Gemini CLI.

**Best For:** Google ecosystem users willing to subscribe. Multi-agent workflows.

**Weakness:** Closed-source. Free tier is less generous than Gemini CLI was. Migration pain.

---

### 14. Devin CLI (Cognition)

| Attribute | Value |
|-----------|-------|
| **Stars** | N/A (proprietary) |
| **Language** | Rust (custom terminal rendering) |
| **Model** | Opus 4.7, GPT-5.5, SWE-1.6 (Cognition proprietary) |
| **Pricing** | Devin Cloud: from $500/mo per seat; Devin CLI bundled |
| **Install** | `curl -fsSL https://cli.devin.ai/install.sh \| bash` |
| **Binary** | `devin` |

**Key Differentiators:**
- Local-to-cloud handoff: start in terminal, hand off to Devin Cloud when work outgrows laptop
- Cloud agents keep working after you close your laptop
- SWE-1.6 proprietary model purpose-built for coding
- Skills system: custom slash commands via Markdown
- Part of Devin Suite: Cloud + Desktop (ex-Windsurf) + CLI + Review
- Agent Command Center for orchestrating multiple agents

**Best For:** Teams already using Devin ecosystem. Long-running autonomous tasks.

**Weakness:** Very expensive ($500/mo+). Proprietary. Cloud features not fully available in CLI yet.

---

### 15. Kiro CLI (AWS)

| Attribute | Value |
|-----------|-------|
| **Stars** | N/A (proprietary) |
| **Language** | Go |
| **Model** | Claude Sonnet 4.5 + open-weight models |
| **Surface** | IDE + CLI + Web |
| **Pricing** | Free (50 credits/mo), Pro $20/mo (1,000 credits), Pro+ $40/mo, Power $200/mo |
| **Install** | `curl -fsSL https://cli.kiro.dev/install \| bash` |
| **Binary** | `kiro-cli` |

**Key Differentiators:**
- Spec-driven development: prompts → requirements → design → executable tasks
- Custom agents: JSON-configurable specialists (Rust expert, AWS ops, code review)
- Built on Q Developer CLI technology with added Haiku 4.5 and social login
- Hooks system, MCP servers, steering files
- Shared config with Kiro IDE (`.kiro/` folder)
- AST-based code intelligence

**Best For:** AWS ecosystem developers wanting spec-driven rigor. Structured development workflows.

**Weakness:** Small community. AWS product (risk of deprecation). Credit-based pricing.

---

### 16. Warp (AI Terminal + Agent)

| Attribute | Value |
|-----------|-------|
| **Stars** | 20K (proprietary) |
| **Language** | Rust |
| **Surface** | Native terminal emulator (macOS, Linux, Windows) |
| **Model** | Multiple (Claude, GPT, Gemini) |
| **Pricing** | Free tier; Warp AI Pro $12/mo; Warp Teams $25/user/mo |

**Key Differentiators:**
- AI-native terminal: not just an agent, but a reimagined terminal with AI built-in
- Warp AI: natural language → shell commands directly in the terminal
- Agent mode: multi-step coding tasks with file editing
- Smart autocomplete, workflow saving, Notebook-style command history
- IDE-like features: command palette, themes, splits

**Best For:** Developers who want a modern terminal with AI baked in, not just an agent CLI.

**Weakness:** Proprietary. Not a standalone agent — it's a terminal replacement. Requires running their terminal emulator.

## Comparison Matrix

### By Benchmark Performance

| Rank | Agent + Model | Terminal-Bench 2.1 |
|------|--------------|-------------------|
| 1 | Codex CLI + GPT-5.5 | **83.4%** |
| 2 | Claude Code + Opus 4.8 | 78.9% |
| 3 | Terminus 2 + GPT-5.5 | 78.2% |
| 4 | Terminus 2 + Opus 4.8 | 74.6% |
| 5 | Gemini CLI + Gemini 3.1 Pro | 70.7% |
| 6 | Claude Code + Opus 4.7 | 69.7% |

### By Pricing

| Agent | Free Tier | Starting Price | Model Costs |
|-------|-----------|---------------|-------------|
| Antigravity CLI | ✅ Weekly quotas | $19.99/mo AI Pro | Included |
| Pi | ✅ BYOK / sub login | Free | $0-20/mo typical |
| OpenCode | ✅ BYOK | Free | $3-20/mo typical |
| Aider | ✅ BYOK | Free | $3-20/mo typical |
| Cline | ✅ BYOK | Free | $3-20/mo typical |
| Goose | ✅ BYOK | Free | $3-20/mo typical |
| Plandex | ✅ BYOK | Free | $3-20/mo typical |
| Amazon Q Developer | ✅ 50 req/mo | $19/user/mo Pro | Included |
| Kiro CLI | ✅ 50 credits/mo | $20/mo Pro | Credits |
| Gemini CLI | ✅ 1000 req/day (ended June 2026) | Free (was) | Included |
| Warp | ✅ Limited | $12/mo Pro | Included |
| Codex CLI | ❌ | Included with ChatGPT ($20/mo) | Variable |
| Claude Code | ❌ | $20/mo Pro | Included up to limits |
| GitHub Copilot | ✅ Limited | $10/mo Pro | Usage-based billing |
| Grok Build | ❌ | ~$300/mo (SuperGrok Heavy) | Included |
| Devin CLI | ❌ | $500/mo per seat | Included |

### By Open-Source Status

| Agent | License | Truly Open? | Forkable? |
|-------|---------|-------------|-----------|
| OpenCode | **MIT** | ✅ | ✅ |
| Pi | Apache 2.0 | ✅ | ✅ |
| Aider | Apache 2.0 | ✅ | ✅ |
| Cline | Apache 2.0 | ✅ | ✅ |
| Goose | Apache 2.0 | ✅ (AAIF) | ✅ |
| Codex CLI | Apache 2.0 | ✅ | ✅ |
| Plandex | Apache 2.0 | ✅ | ✅ |
| Gemini CLI | Apache 2.0 (frozen) | ⚠️ Now unsupported | ❌ |
| Claude Code | Proprietary | ❌ | ❌ |
| Kilo Code | MIT | ✅ | ✅ |
| Grok Build | Proprietary | ❌ | ❌ |
| Antigravity CLI | Closed-source | ❌ | ❌ |
| GitHub Copilot | Proprietary | ❌ | ❌ |
| Amazon Q Developer | Proprietary | ❌ | ❌ |
| Devin CLI | Proprietary | ❌ | ❌ |
| Kiro CLI | Proprietary | ❌ | ❌ |
| Warp | Proprietary | ❌ | ❌ |

### By Model Flexibility

| Agent | Providers | Local Models | BYOK |
|-------|-----------|-------------|------|
| **OpenCode** | 75+ | ✅ Ollama, LM Studio, llama.cpp | ✅ |
| **Pi** | 15+ | ✅ Ollama | ✅ |
| **Cline** | 30+ | ✅ Ollama, LM Studio | ✅ |
| **Goose** | 25+ | ✅ Ollama | ✅ |
| **Aider** | 15+ | ✅ Ollama | ✅ |
| **Plandex** | 10+ | ✅ Ollama | ✅ |
| **Grok Build** | 5+ (via OpenRouter) | ❌ | ⚠️ Sub only |
| **Codex CLI** | 1 (OpenAI) | ⚠️ Limited | ✅ (OpenAI key) |
| **Claude Code** | 1 (Anthropic) | ❌ | ❌ (sub only) |
| **Gemini CLI** | 1 (Google) | ❌ | ✅ (Google key) |
| **Antigravity CLI** | 1 (Google) | ❌ | ❌ |
| **Amazon Q Developer** | 1 (Bedrock/Claude) | ❌ | ❌ |
| **Kiro CLI** | 5+ | ❌ | ❌ (credits) |
| **Devin CLI** | 3 (Opus, GPT, SWE-1.6) | ❌ | ❌ |
| **Warp** | 5+ | ❌ | ❌ |
| **Kilo Code** | 10+ | ✅ | ✅ |

### By Surface

| Agent | Terminal TUI | CLI | VS Code | JetBrains | Desktop | API/SDK |
|-------|-------------|-----|---------|-----------|---------|---------|
| OpenCode | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| Pi | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Claude Code | ❌ | ✅ | ❌ | ❌ | ✅ | ❌ |
| Codex CLI | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Cline | ✅ (v2.0+) | ✅ | ✅ | ✅ | ❌ | ✅ |
| Grok Build | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ (ACP) |
| Aider | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Goose | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Antigravity CLI | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ |
| Devin CLI | ❌ | ✅ | ❌ | ❌ | ✅ (Desktop) | ✅ |
| Amazon Q Developer | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Kiro CLI | ❌ | ✅ | ✅ | ❌ | ✅ (IDE) | ❌ |
| Warp | ✅ (native terminal) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Gemini CLI | ❌ | ✅ | ✅ (Code Assist) | ❌ | ❌ | ❌ |

## Installation Quick Reference

```bash
# OpenCode (most popular OSS)
curl -fsSL https://opencode.ai/install | bash
npm i -g opencode-ai
brew install opencode

# Pi (minimal agent by Armin Ronacher)
curl -fsSL https://pi.dev/install.sh | sh
npm i -g @earendil-works/pi-coding-agent

# Grok Build (xAI, beta, SuperGrok Heavy only)
curl -fsSL https://x.ai/cli/install.sh | bash

# Plandex (large codebases)
curl -fsSL https://plandex.ai/install.sh | sh

# Claude Code
npm i -g @anthropic-ai/claude-code

# Codex CLI
npm i -g @openai/codex

# Gemini CLI (being replaced by Antigravity)
npm i -g @google/gemini-cli

# Google Antigravity CLI (agy, replaces Gemini CLI)
curl -fsSL https://antigravity.google/cli/install.sh | bash

# Amazon Q Developer CLI
brew install amazon-q
curl -fsSL https://amazonq.dev/install | bash

# Devin CLI (Cognition)
curl -fsSL https://cli.devin.ai/install.sh | bash

# Kiro CLI (AWS)
curl -fsSL https://cli.kiro.dev/install | bash

# Aider
curl -LsSf https://aider.chat/install.sh | sh
pip install aider-chat

# Cline
npm i -g cline

# Goose
curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash

# Warp (AI-native terminal, not just an agent)
brew install --cask warp

# GitHub Copilot CLI
npm i -g @github/copilot
```

## Decision Flowchart

```
Which terminal AI agent should you use?

Do you want free + open source?
├─ Do you want the largest community?
│   └─ OpenCode (180K stars, 75+ providers)
├─ Do you want minimal, hackable, self-extending?
│   └─ Pi (4 tools, 1K-token system prompt, agent writes its own tools)
├─ Do you want git-native workflow?
│   └─ Aider (auto-commit, tree-sitter)
├─ Do you want a VS Code agent in terminal?
│   └─ Cline (Plan/Act, MCP marketplace)
├─ Do you want large codebase focus?
│   └─ Plandex (2M token context, diff sandbox)
└─ Do you want foundation governance?
    └─ Goose (Linux Foundation AAIF)

Do you want a specific model's best experience?
├─ Claude → Claude Code ($20/mo)
├─ GPT → Codex CLI (included with ChatGPT)
├─ Gemini → Antigravity CLI ($19.99/mo, replaced Gemini CLI)
└─ Grok → Grok Build (~$300/mo, SuperGrok Heavy)

Do you want the highest benchmark score?
└─ Codex CLI + GPT-5.5 (83.4% Terminal-Bench)

Do you use AWS heavily?
├─ Amazon Q Developer CLI (50 free/mo, $19 Pro, AWS-native)
├─ Kiro CLI (spec-driven, 50 free credits/mo)
└─ Devin CLI (enterprise, $500/mo+)

Do you want a modern AI-native terminal?
└─ Warp (replaces your terminal emulator, AI baked in)

Do you want to run on a server 24/7?
└─ Any CLI agent. Pair with tmux for persistence.
```

## Key Trends (2026)

1. **Consolidation around MCP** — Model Context Protocol is now the standard for tool integration. Every major agent supports it.

2. **Rust for performance** — Codex CLI rewrote in Rust. Goose is 58% Rust. Startup times dropped from ~800ms to <50ms.

3. **AGENTS.md standardization** — OpenAI's AGENTS.md format is becoming the de facto standard for per-project agent configuration, adopted by Claude Code, Codex CLI, and others.

4. **Parallel agents** — Multi-agent execution (OpenCode 10 agents, Codex CLI worktrees, Claude Code Agent Teams) is the new frontier.

5. **Free tier contraction** — Gemini CLI's free individual access ended June 2026. GitHub Copilot moved to usage-based billing. The free tier window is closing.

6. **Open-source domination** — OpenCode (180K stars) is now the most-starred AI coding tool of any kind, surpassing even many GUI tools.

## Sources

- [Every CLI coding agent, compared](https://michaellivs.com/blog/cli-coding-agents-compared) (Feb 2026)
- [The 2026 Guide to Coding CLI Tools: 15 AI Agents Compared](https://www.tembo.io/blog/coding-cli-tools-comparison) (Feb 2026)
- [Best AI Coding Agent 2026: Ranked by Terminal-Bench](https://www.morphllm.com/ai-coding-agent) (Jun 2026)
- [AI Terminal Coding Tools 2026: Claude Code vs Codex CLI vs Gemini CLI vs OpenCode](https://baeseokjae.github.io/posts/ai-terminal-coding-tools-comparison-2026/) (May 2026)
- [OpenCode vs Claude Code vs Cursor](https://computingforgeeks.com/opencode-vs-claude-code-vs-cursor) (Apr 2026)
- [The 5 Best AI CLI Tools for Coding in 2026](https://pasqualepillitteri.it/en/news/586/best-ai-cli-tools-coding-2026) (Jun 2026)
- [Best Open Source CLI Coding Agents in 2026](https://pinggy.io/blog/best_open_source_cli_coding_agents) (May 2026)
