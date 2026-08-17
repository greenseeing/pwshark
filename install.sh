#!/usr/bin/env bash
# pwshark installer — safe to re-run; upgrades in place when a newer release exists.
#
#   curl -fsSL https://github.com/greenseeing/pwshark/releases/latest/download/install.sh | bash
#
# Downloads a prebuilt static binary from the latest GitHub release (no Rust
# toolchain required). Falls back to building from source if no prebuilt binary
# matches this machine, or if the download fails.
set -euo pipefail

REPO="greenseeing/pwshark"
BIN="pwshark"

# --- output helpers -------------------------------------------------------
if [ -t 1 ]; then
  C_STEP=$'\033[36m'; C_OK=$'\033[32m'; C_WARN=$'\033[33m'; C_ERR=$'\033[31m'; C_OFF=$'\033[0m'
else
  C_STEP=""; C_OK=""; C_WARN=""; C_ERR=""; C_OFF=""
fi
step() { printf '%s==>%s %s\n' "$C_STEP" "$C_OFF" "$*"; }
ok()   { printf '%s ok%s  %s\n' "$C_OK" "$C_OFF" "$*"; }
warn() { printf '%swarning:%s %s\n' "$C_WARN" "$C_OFF" "$*" >&2; }
die()  { printf '%serror:%s %s\n' "$C_ERR" "$C_OFF" "$*" >&2; exit 1; }

# Run a privileged command via sudo when not already root.
as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    die "this step needs root; install sudo or re-run as root"
  fi
}

# --- detection ------------------------------------------------------------
# Echo the release-asset arch suffix, or nothing if unsupported (-> source build).
detect_arch() {
  case "$(uname -m)" in
    x86_64 | amd64) echo "amd64" ;;
    aarch64 | arm64) echo "arm64" ;;
    *) echo "" ;;
  esac
}

# --- versions -------------------------------------------------------------
latest_version() {
  # An explicit pin skips the API entirely (handy when offline or behind a proxy).
  if [ -n "${PWSHARK_VERSION:-}" ]; then
    printf '%s' "${PWSHARK_VERSION#v}"
    return 0
  fi
  # Resolve the newest release tag (e.g. v0.1.1 -> 0.1.1) from the redirect of
  # /releases/latest. Unlike api.github.com (shared 60/hr per-IP budget) and
  # raw.githubusercontent.com, this endpoint is not rate-limited when
  # unauthenticated.
  local tag_url
  tag_url="$(curl -fsS -o /dev/null -w '%{redirect_url}' "https://github.com/$REPO/releases/latest" 2>/dev/null || true)"
  case "$tag_url" in
    */releases/tag/v*) printf '%s' "${tag_url##*/tag/v}" ;;
    *) die "could not resolve the latest release from https://github.com/$REPO/releases/latest. Try again later, or pin a version:
  PWSHARK_VERSION=0.1.1 curl -fsSL https://github.com/$REPO/releases/latest/download/install.sh | bash" ;;
  esac
}

installed_version() {
  if command -v "$BIN" >/dev/null 2>&1; then
    "$BIN" --version 2>/dev/null | awk '{print $2}'
  fi
}

# --- install --------------------------------------------------------------
# Reuse an existing install's directory so re-runs replace in place instead of
# shadowing it; otherwise default to ~/.local/bin (no sudo for the common case).
choose_bindir() {
  local existing
  existing="$(command -v "$BIN" 2>/dev/null || true)"
  if [ -n "$existing" ]; then
    dirname "$(readlink -f "$existing")"
  else
    echo "$HOME/.local/bin"
  fi
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

# Fail closed: a missing checksum or missing sha256 tool aborts the install rather
# than silently accepting an unverified binary. Set PWSHARK_SKIP_VERIFY=1 to opt out.
verify_checksum() {
  local file="$1" url="$2"
  local sums expected actual
  sums="$(curl -fsSL "${url}.sha256" 2>/dev/null || true)"
  if [ -z "$sums" ]; then
    if [ "${PWSHARK_SKIP_VERIFY:-}" = "1" ]; then
      warn "no published checksum — proceeding because PWSHARK_SKIP_VERIFY=1"
      return 0
    fi
    die "no published checksum for this release — refusing to install an unverified binary.
  To override (not recommended), re-run with PWSHARK_SKIP_VERIFY=1."
  fi
  expected="$(printf '%s' "$sums" | awk '{print $1}')"
  actual="$(sha256_of "$file")"
  if [ -z "$actual" ]; then
    if [ "${PWSHARK_SKIP_VERIFY:-}" = "1" ]; then
      warn "no sha256 tool found — proceeding because PWSHARK_SKIP_VERIFY=1"
      return 0
    fi
    die "no sha256 tool (sha256sum/shasum) found — cannot verify the download.
  Install coreutils, or re-run with PWSHARK_SKIP_VERIFY=1 to skip the check."
  fi
  [ "$expected" = "$actual" ] || die "checksum mismatch — refusing to install (expected $expected, got $actual)"
  ok "checksum verified"
}

# Download + atomically replace the binary. Returns non-zero on download failure
# so the caller can fall back to a source build.
install_binary() {
  local version="$1" arch="$2" bindir="$3"
  local url="https://github.com/$REPO/releases/download/v${version}/${BIN}-linux-${arch}"
  step "Downloading $BIN v$version ($arch)"

  mkdir -p "$bindir" 2>/dev/null || as_root mkdir -p "$bindir"

  # Stage the temp file on the SAME filesystem as the target so the final move is
  # an atomic rename — this sidesteps ETXTBSY ("Text file busy") when pwshark is
  # updating its own running binary.
  local tmp writable=0
  [ -w "$bindir" ] && writable=1
  if [ "$writable" -eq 1 ]; then
    tmp="$bindir/.$BIN.new.$$"
  else
    tmp="$(mktemp)"
  fi

  if ! curl -fSL --progress-bar -o "$tmp" "$url"; then
    rm -f "$tmp" 2>/dev/null || true
    warn "could not download prebuilt binary ($url)"
    return 1
  fi
  verify_checksum "$tmp" "$url"
  chmod +x "$tmp"

  if [ "$writable" -eq 1 ]; then
    mv -f "$tmp" "$bindir/$BIN"
  else
    # Stage inside $bindir (same filesystem) so the final replace is an atomic
    # rename. A cross-filesystem mv from /tmp is not atomic and can hit ETXTBSY
    # when replacing a running binary.
    local stage="$bindir/.$BIN.new.$$"
    as_root cp "$tmp" "$stage"
    as_root chmod +x "$stage"
    as_root mv -f "$stage" "$bindir/$BIN"
    rm -f "$tmp" 2>/dev/null || true
  fi
  ok "installed to $bindir/$BIN"

  case ":$PATH:" in
    *":$bindir:"*) ;;
    *) warn "$bindir is not on your PATH — add it with:  export PATH=\"$bindir:\$PATH\"" ;;
  esac
}

# --- source-build fallback ------------------------------------------------
build_from_source() {
  local bindir="$1" version="${2:-}"
  local repo_url="https://github.com/$REPO.git"
  local src_dir="${HOME}/.local/share/pwshark"

  warn "falling back to building from source (needs a Rust toolchain)"

  # Pick up an existing rustup install that isn't on this shell's PATH
  # (e.g. fish/zsh users whose interactive PATH lacks ~/.cargo/bin).
  [ -f "${HOME}/.cargo/env" ] && source "${HOME}/.cargo/env"

  if ! command -v cargo >/dev/null 2>&1; then
    step "Rust not found — installing via rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "${HOME}/.cargo/env"
  fi

  # Clipboard backend (x11rb is pure-Rust, but the X11/xkb dev headers are needed
  # to *build* arboard from source on Debian/Ubuntu).
  if command -v apt-get >/dev/null 2>&1; then
    step "Installing clipboard build dependencies"
    as_root apt-get install -y libxcb1-dev libx11-dev libxkbcommon-dev >/dev/null 2>&1 || \
      warn "could not install build deps — install libxcb1-dev libx11-dev libxkbcommon-dev manually if the build fails"
  fi

  # Build from the exact release tag when we know it, so a force-pushed branch or
  # tag cannot silently change what gets compiled. Fall back to the default branch
  # only when no release version is known (e.g. before the first release).
  local tag=""
  [ -n "$version" ] && tag="v$version"
  if [ -d "$src_dir/.git" ]; then
    step "Updating source checkout"
    git -C "$src_dir" fetch --tags origin >/dev/null 2>&1 || true
    if [ -n "$tag" ]; then
      git -C "$src_dir" checkout --quiet "$tag"
    else
      git -C "$src_dir" pull --ff-only
    fi
  else
    step "Cloning $REPO"
    rm -rf "$src_dir"
    if [ -n "$tag" ]; then
      git clone --quiet --branch "$tag" --depth 1 "$repo_url" "$src_dir"
    else
      git clone --quiet "$repo_url" "$src_dir"
    fi
  fi

  step "Building release binary"
  ( cd "$src_dir" && cargo build --release )

  mkdir -p "$bindir"
  local tmp="$bindir/.$BIN.new.$$"
  cp "$src_dir/target/release/$BIN" "$tmp"
  chmod +x "$tmp"
  mv -f "$tmp" "$bindir/$BIN"
  ok "installed to $bindir/$BIN"
}

main() {
  command -v curl >/dev/null 2>&1 || die "curl is required"
  echo "pwshark installer"

  local arch latest current bindir
  arch="$(detect_arch)"
  latest="$(latest_version)"
  current="$(installed_version)"
  bindir="$(choose_bindir)"

  if [ -n "$latest" ] && [ "$current" = "$latest" ]; then
    ok "$BIN is already up to date (v$current)"
    return 0
  fi

  [ -n "$current" ] && [ -n "$latest" ] && step "Updating $BIN: v$current -> v$latest"

  # Prefer the prebuilt binary; fall back to source on unsupported arch, no
  # published release yet, or a failed download.
  if [ -n "$arch" ] && [ -n "$latest" ] && install_binary "$latest" "$arch" "$bindir"; then
    :
  else
    build_from_source "$bindir" "$latest"
  fi

  printf '\n'
  ok "Done. Run '$BIN' to start."
}

main "$@"
