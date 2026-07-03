#!/usr/bin/env bash
# TermWeb Browser — Auto-detect AI agents and install the bcli-web-agent skill
set -euo pipefail

SKILL_DIR_REL=".skills/bcli-web-agent"
SKILL_NAME="bcli-web-agent"
SKILL_FILE="SKILL.md"

# ─── Colors ───────────────────────────────────────────────────────────────────
info()  { printf "\033[1;34m•\033[0m %s\n" "$*"; }
ok()    { printf "\033[1;32m✓\033[0m %s\n" "$*"; }
warn()  { printf "\033[1;33m!\033[0m %s\n" "$*" >&2; }

# ─── Resolve source dir ───────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILL_SRC_DIR="$REPO_DIR/$SKILL_DIR_REL"

if [ ! -f "$SKILL_SRC_DIR/$SKILL_FILE" ]; then
  warn "Skill source not found at $SKILL_SRC_DIR/$SKILL_FILE"
  exit 1
fi

# ─── Install to a given skill directory ────────────────────────────────────────
install_skill() {
  local target_base="$1"
  local target_dir="$target_base/$SKILL_NAME"
  mkdir -p "$target_dir"
  cp "$SKILL_SRC_DIR/$SKILL_FILE" "$target_dir/$SKILL_FILE"

  # Also copy skill.json if present (for backward compat)
  if [ -f "$SKILL_SRC_DIR/skill.json" ]; then
    cp "$SKILL_SRC_DIR/skill.json" "$target_dir/skill.json"
  fi

  ok "Installed to $target_dir"
}

# ─── Detect installed agents ──────────────────────────────────────────────────
detect_agents() {
  local detected=()

  # opencode
  if command -v opencode &>/dev/null; then
    detected+=("opencode")
  fi

  # Claude Code
  if command -v claude &>/dev/null; then
    detected+=("claude-code")
  fi

  # Google Gemini CLI
  if command -v gemini &>/dev/null; then
    detected+=("gemini-cli")
  fi

  # Aider
  if command -v aider &>/dev/null; then
    detected+=("aider")
  fi

  # Cursor (detect via config directory, not CLI command)
  if [ -d "$HOME/.cursor" ]; then
    detected+=("cursor")
  fi

  # Codex CLI (OpenAI)
  if command -v codex &>/dev/null; then
    detected+=("codex")
  fi

  printf '%s\n' "${detected[@]}"
}

# ─── Install per agent ────────────────────────────────────────────────────────
install_for_opencode() {
  local dirs=(
    "$HOME/.config/opencode/skills"
  )
  # Also check project-level .opencode
  if [ -d "$REPO_DIR/.opencode" ]; then
    dirs+=("$REPO_DIR/.opencode/skills")
  fi
  for d in "${dirs[@]}"; do
    [ -n "$d" ] && install_skill "$d"
  done
}

install_for_claude_code() {
  local dirs=(
    "$HOME/.claude/skills"
  )
  if [ -d "$REPO_DIR/.claude" ]; then
    dirs+=("$REPO_DIR/.claude/skills")
  fi
  for d in "${dirs[@]}"; do
    [ -n "$d" ] && install_skill "$d"
  done
}

install_for_gemini_cli() {
  # Gemini CLI uses .gemini/ in home or project
  local dirs=(
    "$HOME/.gemini"
  )
  if [ -d "$REPO_DIR/.gemini" ]; then
    dirs+=("$REPO_DIR/.gemini")
  fi
  for d in "${dirs[@]}"; do
    if [ -d "$d" ]; then
      local target="$d/skills/$SKILL_NAME"
      mkdir -p "$target"
      cp "$SKILL_SRC_DIR/$SKILL_FILE" "$target/$SKILL_FILE"
      ok "Installed to $target"
    fi
  done
}

install_for_aider() {
  # Aider uses .aider.conf.yml and CONVENTIONS.md
  local aider_conf="$HOME/.aider.conf.yml"
  if [ -f "$aider_conf" ] || [ -d "$HOME/.aider" ]; then
    local conv_dir="$HOME/.aider"
    mkdir -p "$conv_dir"
    # Copy as a conventions file that aider reads
    cp "$SKILL_SRC_DIR/$SKILL_FILE" "$conv_dir/bcli-web-agent.md"
    ok "Installed to $conv_dir/bcli-web-agent.md (aider conventions)"
  fi
}

install_for_cursor() {
  # Cursor uses .cursor/rules/
  local cursor_rules="$HOME/.cursor/rules"
  if [ -d "$HOME/.cursor" ]; then
    mkdir -p "$cursor_rules"
    cp "$SKILL_SRC_DIR/$SKILL_FILE" "$cursor_rules/bcli-web-agent.mdc"
    ok "Installed to $cursor_rules/bcli-web-agent.mdc (Cursor rules)"
  fi
}

install_for_codex() {
  # Codex CLI uses ~/.codex/
  local codex_dir="$HOME/.codex"
  mkdir -p "$codex_dir"
  cp "$SKILL_SRC_DIR/$SKILL_FILE" "$codex_dir/bcli-web-agent.md"
  ok "Installed to $codex_dir/bcli-web-agent.md"
}

# ─── Main ──────────────────────────────────────────────────────────────────────
main() {
  echo ""
  printf "\033[1;36m╔══════════════════════════════════════════╗\033[0m\n"
  printf "\033[1;36m║   TermWeb — AI Agent Skill Installer      ║\033[0m\n"
  printf "\033[1;36m╚══════════════════════════════════════════╝\033[0m\n"
  echo ""

  local agents=()
  if [ $# -gt 0 ]; then
    agents=("$@")
    info "Installing skill for specified agents: ${agents[*]}"
  else
    mapfile -t agents < <(detect_agents)
    info "Detected agents: ${agents[*]:-(none found)}"
  fi

  if [ ${#agents[@]} -eq 0 ]; then
    warn "No supported AI agents detected."
    warn "Install one of: opencode, claude, gemini, aider, codex"
    warn ""
    info "You can manually copy the skill:"
    info "  mkdir -p ~/.config/opencode/skills"
    info "  cp -r \"$SKILL_SRC_DIR\" ~/.config/opencode/skills/$SKILL_NAME"
    exit 0
  fi

  for agent in "${agents[@]}"; do
    agent_trimmed="${agent// /}"
    [ -z "$agent_trimmed" ] && continue
    case "$agent" in
      opencode)    install_for_opencode ;;
      claude-code|claude) install_for_claude_code ;;
      gemini-cli|gemini)  install_for_gemini_cli ;;
      aider)       install_for_aider ;;
      cursor)      install_for_cursor ;;
      codex)       install_for_codex ;;
      *)
        warn "Unknown agent: $agent (supported: opencode, claude, gemini-cli, aider, cursor, codex)"
        ;;
    esac
  done

  echo ""
  ok "Skill installation complete."
  echo ""
  info "What was installed:"
  info "  Name:        $SKILL_NAME"
  info "  Description: Browser automation skill for AI agents"
  info "  Content:     REST API reference + bai CLI usage + automation patterns"
  echo ""
  info "Your AI agent can now discover and load the 'bcli-web-agent' skill"
  info "to browse the web, click elements, fill forms, and extract data."
  echo ""
}

main "$@"
