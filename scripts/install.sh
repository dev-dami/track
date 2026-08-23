#!/usr/bin/env bash
set -e
# Track Language & Yard — interactive ANSI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.sh | bash
#        bash scripts/install.sh [--yes] [--dir ~/.track/bin] [--no-path] [--help]

# ── ANSI ─────────────────────────────────────────────────────────
if [[ -t 1 ]] && [[ "${TERM:-}" != "dumb" ]] && command -v tput >/dev/null 2>&1 && [[ $(tput colors 2>/dev/null || echo 0) -ge 8 ]]; then
  RESET=$'\033[0m'; BOLD=$'\033[1m'; DIM=$'\033[2m'
  RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
  BLUE=$'\033[34m'; MAGENTA=$'\033[35m'; CYAN=$'\033[36m'
  GRAY=$'\033[90m'; WHITE=$'\033[97m'
else
  RESET=""; BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; BLUE=""; MAGENTA=""; CYAN=""; GRAY=""; WHITE=""
fi
CHECK="${GREEN}✔${RESET}"; CROSS="${RED}✘${RESET}"; ARROW="${CYAN}›${RESET}"; DOT="${GRAY}·${RESET}"

info()  { printf "  %b %s\n" "$ARROW" "$*"; }
ok()    { printf "  %b %s\n" "$CHECK" "$*"; }
warn()  { printf "  ${YELLOW}▲${RESET} %s\n" "$*"; }
err()   { printf "  ${RED}✘${RESET} %s\n" "$*" >&2; }
step()  { printf "\n${BOLD}${CYAN}▸ %s${RESET}\n" "$*"; }
hr()    { printf "${GRAY}────────────────────────────────────────${RESET}\n"; }

ask() {
  local prompt="$1" def="$2" ans
  if [[ ! -t 0 ]] || [[ "${ASSUME_YES:-0}" == "1" ]]; then
    printf "  ${CYAN}?${RESET} %s ${DIM}[%s]${RESET} → ${GRAY}auto:%s${RESET}\n" "$prompt" "$def" "$def"
    REPLY="$def"
    return 0
  fi
  printf "  ${CYAN}?${RESET} %s ${DIM}[%s]${RESET} " "$prompt" "$def"
  read -r ans || ans=""
  REPLY="${ans:-$def}"
}

banner() {
  printf "${BOLD}${WHITE}"
  cat <<'BANNER'
   ████████╗██████╗  █████╗  ██████╗██╗  ██╗
   ╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝
      ██║   ██████╔╝███████║██║     █████╔╝
      ██║   ██╔══██╗██╔══██║██║     ██╔═██╗
      ██║   ██║  ██║██║  ██║╚██████╗██║  ██╗
      ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝
BANNER
  printf "${RESET}"
  printf "${DIM}  Track — linear ownership, scoped lenses, Cranelift${RESET}\n"
  hr
}

# ── args ──────────────────────────────────────────────────────────
INSTALL_DIR="${TRACK_INSTALL_DIR:-$HOME/.track/bin}"
ASSUME_YES=0
NO_MODIFY_PATH=0
REPO_URL="https://github.com/dev-dami/track.git"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -y|--yes) ASSUME_YES=1; shift ;;
    --dir) INSTALL_DIR="$2"; shift 2 ;;
    --no-path|--no-modify-path) NO_MODIFY_PATH=1; shift ;;
    -h|--help)
      printf "${BOLD}Track installer${RESET}\n"
      printf "Usage: install.sh [options]\n"
      printf "  -y, --yes          non-interactive, assume yes\n"
      printf "      --dir <path>   install dir (default: ~/.track/bin)\n"
      printf "      --no-path      don't modify shell profile\n"
      printf "  -h, --help         this help\n"
      printf "Env:\n"
      printf "  TRACK_INSTALL_DIR        override install dir\n"
      printf "  TRACK_NO_MODIFY_PATH=1   skip PATH setup\n"
      exit 0 ;;
    *) warn "unknown arg: $1"; shift ;;
  esac
done

[[ "${TRACK_NO_MODIFY_PATH:-0}" == "1" ]] && NO_MODIFY_PATH=1

banner
printf "${DIM}  repo:${RESET} %s  ${DIM}install:${RESET} %s\n" "$REPO_URL" "$INSTALL_DIR"
if [[ $ASSUME_YES == 1 ]]; then
  printf "${DIM}  mode:${RESET} non-interactive (--yes)\n"
else
  printf "${DIM}  mode:${RESET} interactive (use --yes for CI)\n"
fi

# ── checks ────────────────────────────────────────────────────────
step "Checking prerequisites"

need_ok=1
if command -v git >/dev/null 2>&1; then ok "git $(git --version | head -n1 | awk '{print $3}')"; else err "git not found — install git"; need_ok=0; fi
if command -v cargo >/dev/null 2>&1; then
  ok "cargo $(cargo --version | awk '{print $2}')"
else
  err "cargo not found — install Rust: https://rustup.rs"
  printf "    ${DIM}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${RESET}\n"
  need_ok=0
fi
if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then
  ccv=$(cc --version 2>/dev/null | head -n1 || gcc --version | head -n1)
  ok "cc ${ccv:0:48}"
else
  warn "cc/gcc not found — linking may fail"
fi
if [[ $need_ok == 0 ]]; then err "fix prerequisites and re-run"; exit 1; fi

# ── confirm ───────────────────────────────────────────────────────
step "Configuration"
ask "Install directory" "$INSTALL_DIR"
INSTALL_DIR="$REPLY"
# expand ~
INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"

if [[ -d "$INSTALL_DIR" ]] && ls "$INSTALL_DIR"/track >/dev/null 2>&1; then
  warn "existing Track found at $INSTALL_DIR"
  ask "Overwrite existing binaries? (y/N)" "y"
  case "$REPLY" in y|Y|yes|YES) :;; *) info "aborted by user"; exit 0;; esac
fi

if [[ $NO_MODIFY_PATH == 0 ]]; then
  ask "Add $INSTALL_DIR to PATH via shell profile? (Y/n)" "Y"
  case "$REPLY" in y|Y|yes|YES) MODIFY_PATH=1;; *) MODIFY_PATH=0;; esac
else
  MODIFY_PATH=0
fi

ask "Install editor grammars (VS Code + Neovim) if found? (Y/n)" "Y"
case "$REPLY" in y|Y|yes|YES) INSTALL_EDITORS=1;; *) INSTALL_EDITORS=0;; esac

hr
printf "${BOLD}  Summary:${RESET} dir=${CYAN}%s${RESET}  PATH=%s  editors=%s\n" "$INSTALL_DIR" "$([ $MODIFY_PATH == 1 ] && echo "yes" || echo "no")" "$([ $INSTALL_EDITORS == 1 ] && echo "yes" || echo "no")"
if [[ -t 0 ]] && [[ $ASSUME_YES == 0 ]]; then
  ask "Proceed with install? (Y/n)" "Y"
  case "$REPLY" in y|Y|yes|YES) :;; *) info "cancelled"; exit 0;; esac
fi

# ── fetch ─────────────────────────────────────────────────────────
step "Fetching Track"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
printf "  ${DIM}→ cloning --depth 1 $REPO_URL${RESET}\n"
# pretty git clone with no noisy output unless fail
if ! git clone --depth 1 "$REPO_URL" "$TMP_DIR/track" 2>&1 | sed 's/^/  '"$DOT"' /'; then
  err "git clone failed"
  exit 1
fi
ok "cloned to $TMP_DIR/track"
cd "$TMP_DIR/track"

# ── build ─────────────────────────────────────────────────────────
step "Building release binaries"
printf "  ${DIM}cargo build --release --bins  (this may take a minute)${RESET}\n"
# show cargo output with dim prefix; keep colors if terminal
if ! cargo build --release --bins 2>&1 | sed 's/^/  '"$GRAY"'│ '"$RESET"'/'; then
  err "cargo build failed — check Rust toolchain"
  exit 1
fi
ok "built track, yard, track-lsp"

# ── install ───────────────────────────────────────────────────────
step "Installing"
mkdir -p "$INSTALL_DIR"
BINARIES=("track" "yard" "track-lsp")
for bin in "${BINARIES[@]}"; do
  src="target/release/$bin"
  if [[ ! -f "$src" ]]; then err "missing $src (build failed)"; exit 1; fi
  cp "$src" "$INSTALL_DIR/$bin"
  chmod +x "$INSTALL_DIR/$bin"
  ok "$bin → $INSTALL_DIR/$bin"
done

# ── PATH ──────────────────────────────────────────────────────────
if [[ $MODIFY_PATH == 1 ]]; then
  step "Configuring PATH"
  SHELL_NAME=$(basename "${SHELL:-bash}")
  PROFILE_FILE=""
  case "$SHELL_NAME" in
    bash)
      if [[ -f "$HOME/.bashrc" ]]; then PROFILE_FILE="$HOME/.bashrc"
      elif [[ -f "$HOME/.bash_profile" ]]; then PROFILE_FILE="$HOME/.bash_profile"
      else PROFILE_FILE="$HOME/.profile"
      fi ;;
    zsh)  PROFILE_FILE="$HOME/.zshrc" ;;
    fish) PROFILE_FILE="$HOME/.config/fish/config.fish" ;;
    *)    PROFILE_FILE="$HOME/.profile" ;;
  esac

  PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    export PATH="$INSTALL_DIR:$PATH"
    ok "exported PATH for this session"
  fi

  if [[ "$SHELL_NAME" == "fish" ]]; then
    if [[ -f "$PROFILE_FILE" ]] && ! grep -q "$INSTALL_DIR" "$PROFILE_FILE" 2>/dev/null; then
      printf "\n# Track Language\nset -gx PATH %s \$PATH\n" "$INSTALL_DIR" >> "$PROFILE_FILE"
      ok "added to $PROFILE_FILE"
    else
      info "fish PATH already configured or $PROFILE_FILE missing"
    fi
  else
    if [[ -n "$PROFILE_FILE" ]]; then
      if ! grep -qF "$INSTALL_DIR" "$PROFILE_FILE" 2>/dev/null; then
        {
          echo ""
          echo "# Track Language — added by install.sh"
          echo "$PATH_LINE"
        } >> "$PROFILE_FILE"
        ok "added to $PROFILE_FILE"
      else
        info "already in $PROFILE_FILE"
      fi
    else
      warn "could not detect profile — add manually: $PATH_LINE"
    fi
  fi
else
  info "skipped PATH setup (--no-path)"
  export PATH="$INSTALL_DIR:$PATH"
fi

# ── editors ───────────────────────────────────────────────────────
if [[ $INSTALL_EDITORS == 1 ]]; then
  step "Editor integration"
  # VS Code
  if command -v code >/dev/null 2>&1; then
    # link grammar if not via marketplace
    VS_SRC="$TMP_DIR/track/grammars/track.tmLanguage.json"
    if [[ -f "$VS_SRC" ]]; then
      ok "VS Code detected — run 'code --install-extension track-vscode-*.vsix' or copy grammars/track.tmLanguage.json"
      info "VS Code extension lives in editor/vscode/ — open it and press F5 to test"
    fi
  else
    info "VS Code not found — grammar at grammars/track.tmLanguage.json"
  fi
  # Neovim
  if [[ -d "$HOME/.config/nvim" ]] || command -v nvim >/dev/null 2>&1; then
    info "Neovim: add to init.lua →  require('track').setup()  (see editor/nvim/README.md)"
    info "  vim.opt.rtp:append(\"$INSTALL_DIR/../share/track/editor/nvim\")  or symlink editor/nvim"
  fi
fi

# ── verify ────────────────────────────────────────────────────────
step "Verifying"
for bin in "${BINARIES[@]}"; do
  if command -v "$bin" >/dev/null 2>&1; then
    ver=$("$bin" --version 2>/dev/null | head -n1 || echo "$bin")
    ok "$ver"
  else
    # try direct path
    if [[ -x "$INSTALL_DIR/$bin" ]]; then
      ver=$("$INSTALL_DIR/$bin" --version 2>/dev/null | head -n1)
      ok "$ver (at $INSTALL_DIR/$bin — restart shell for PATH)"
    else
      warn "$bin not executable"
    fi
  fi
done

printf "\n"
hr
printf "${BOLD}${GREEN}  Track installed!${RESET}\n"
printf "  ${DIM}bin:${RESET} %s\n" "$INSTALL_DIR"
printf "  ${DIM}track:${RESET} %s\n" "$("$INSTALL_DIR/track" --version 2>/dev/null || echo "track")"
printf "  ${DIM}yard:${RESET}  %s\n" "$("$INSTALL_DIR/yard" --version 2>/dev/null || echo "yard")"
printf "\n"
printf "  ${BOLD}Quick start:${RESET}\n"
printf "    ${CYAN}track --help${RESET}          type-check / build\n"
printf "    ${CYAN}yard init my_app${RESET}      new project\n"
printf "    ${CYAN}yard run${RESET}              build & run\n"
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  printf "\n  ${YELLOW}↻ Restart terminal or:${RESET} ${BOLD}source %s${RESET}\n" "${PROFILE_FILE:-~/.profile}"
fi
printf "\n"
printf "  ${DIM}docs:${RESET} README.md • SPEC.md • grammars/README.md • editor/nvim/README.md\n"
printf "  ${DIM}uninstall:${RESET} rm -rf %s  (+ remove PATH line from profile)\n" "$INSTALL_DIR"
hr
