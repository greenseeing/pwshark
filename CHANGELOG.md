# Changelog

All notable changes to pwshark are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2026-08-18

### Fixed
- `pwshark update` no longer reports success when the installer download fails:
  the curl-to-bash pipeline lacked `pipefail`, so a failed download (e.g. an
  HTTP 429 from GitHub's rate limiting) fed bash empty input and exited 0
  without updating anything.

### Changed
- `pwshark update` and the README one-liner fetch `install.sh` from the latest
  release's assets instead of `raw.githubusercontent.com`, which GitHub
  rate-limits per IP for unauthenticated fetches (HTTP 429).
- The installer resolves the latest version from the `/releases/latest`
  redirect instead of the GitHub API, whose unauthenticated 60-per-hour budget
  is shared by everything on the same IP.

## [0.3.2] - 2026-07-24

No functional changes to pwshark itself — this release moves where it lives.

### Changed
- pwshark is now hosted at [github.com/greenseeing/pwshark](https://github.com/greenseeing/pwshark);
  Codeberg's July 2026 Terms of Use disallow projects mostly written with
  generative-AI tools, so the repo, releases, and CI moved. Releases are now
  built by GitHub Actions from the same pinned `cargo-zigbuild` image, with the
  same asset names and per-file SHA-256 sums.
- `install.sh` and `pwshark update` resolve releases from GitHub. Existing
  installs upgrade normally while the Codeberg repo still answers: its final
  commit carries this installer, which already points at GitHub. An install
  that misses that window just re-runs the one-liner from the README.

## [0.3.1] - 2026-07-10

### Changed
- Reworked TUI keybindings for clarity: `Tab` is now the sole mode switcher
  (arrow keys no longer switch modes from the mode selector). `Space` toggles
  boolean options and cycles the separator. Typing digits on the Length or
  Words row enters a direct number-edit mode (Enter to apply, Esc to cancel).
  The mode selector is no longer a focus stop — focus starts on the first
  option and wraps around with Up/Down.

## [0.3.0] - 2026-07-08

### Added
- `--notice` flag prints the embedded wordlist's third-party attribution
  (Orchard Street Long List, CC BY-SA 4.0) so the license travels with the
  binary, not just the source tree.

### Changed
- Replaced the embedded wordlist with the **Orchard Street Long List** (17,576
  curated words, CC BY-SA 4.0) in place of the EFF large list (7,776). Under the
  default 5-char truncation the effective pool grows from 6,448 to 10,110
  distinct forms, raising word-selection entropy from 12.66 to 13.30 bits per
  word — for the default 4-word config the reported entropy rises from ~57.1 to
  ~59.7 bits (the word-selection component alone: 50.6 → 53.2 bits). See NOTICE
  for attribution.

## [0.2.0] - 2026-06-15

### Added
- Memorable mode now reports realistic diceware entropy (word_count × log2 of the
  effective word pool, accounting for truncation collisions) instead of the
  charset heuristic — the strength meter no longer reads "Strong" for every
  passphrase.
- `--exclude-ambiguous` flag and a Random-mode toggle to drop visually confusable
  characters (`0 O 1 l I`).
- `--count N` and `--json` for `--stdout`, for scripting/batch generation.

### Changed
- `arboard` is built with `default-features = false`, dropping the unused
  image-codec dependency tree (`image`/`png`/`flate2`/…) — smaller binary and
  supply-chain surface.
- `pwshark update` fetches the installer from the latest release asset
  (`releases/latest/download/install.sh`) instead of the mutable `main` branch.

### Security
- `install.sh` now **fails closed**: a missing checksum or missing sha256 tool
  aborts the install (override with `PWSHARK_SKIP_VERIFY=1`).
- Source-build fallback checks out the exact release tag and no longer uses
  `git fetch --force`; privileged installs stage the binary on the target
  filesystem for an atomic replace.
- CI pins all third-party actions to commit SHAs and gates releases on
  `cargo audit`.

## [0.1.1] - 2026-06-04

### Added
- Prebuilt static binaries (`amd64`, `arm64`) published to the releases page on
  every `v*` tag, so installing no longer requires a Rust toolchain.
- `pwshark update` is now version-aware: re-running the installer is a fast no-op
  when you already have the latest release.

### Changed
- `install.sh` downloads the prebuilt binary and verifies its SHA256, falling
  back to a source build only on an unsupported arch or a failed download.

### Fixed
- Installer no longer re-runs rustup on every update. It now sources
  `~/.cargo/env` before probing for `cargo`, so an existing Rust install is
  detected even when the user's shell (fish/zsh) doesn't put `~/.cargo/bin` on
  PATH.

## [0.1.0] - 2026-06-04

### Added
- Initial release: NIST-compliant random and memorable password generator with a
  ratatui TUI, `--stdout` mode, clipboard copy, entropy meter, and embedded
  wordlist.

[0.3.3]: https://github.com/greenseeing/pwshark/releases/tag/v0.3.3
[0.3.2]: https://github.com/greenseeing/pwshark/releases/tag/v0.3.2
[0.3.1]: https://github.com/greenseeing/pwshark/releases/tag/v0.3.1
[0.3.0]: https://github.com/greenseeing/pwshark/releases/tag/v0.3.0
[0.2.0]: https://github.com/greenseeing/pwshark/releases/tag/v0.2.0
[0.1.1]: https://github.com/greenseeing/pwshark/releases/tag/v0.1.1
[0.1.0]: https://github.com/greenseeing/pwshark/releases/tag/v0.1.0
