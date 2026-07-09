# Releasing pwshark

Releases are built and published by Woodpecker CI on Codeberg. Pushing a `v*`
tag triggers `.woodpecker.yml`, which cross-compiles static musl binaries
(amd64 + arm64) and publishes them — with SHA-256 sums and `install.sh` — to a
Codeberg release. `install.sh` / `pwshark update` then resolve the latest
release and download the binary for the host architecture.

## One-time CI setup

1. Enable the repo in Woodpecker: <https://ci.codeberg.org/repos/add> → select
   `greenseer/pwshark`.
2. Generate a Codeberg access token: **Settings → Applications → Manage Access
   Tokens → Generate Token**, scope **`write:repository`** (copy it now — it is
   shown once).
3. Add it as a Woodpecker repo secret named **`codeberg_token`** (Repository →
   **Settings → Secrets**). Repo secrets are automatically available to `tag`
   pipelines — no per-event configuration needed.

Codeberg's shared runners are amd64-only; the arm64 binary is cross-compiled
with `cargo zigbuild`. Per Codeberg's shared-runner request, the build caps
cargo at `-j 4`.

## Cutting a release

1. Update `CHANGELOG.md`: rename `## [Unreleased]` to `## [x.y.z] - YYYY-MM-DD`
   and add the matching link reference at the bottom.
2. Bump `version` in `Cargo.toml`, then sync `Cargo.lock`. The CI build passes
   `--locked`, so the lockfile must already match the manifest:

   ```bash
   cargo check            # rewrites Cargo.lock's pwshark entry
   cargo check --locked   # must pass — this is what CI does
   ```

   Run the sync with a **plain** `cargo check`. Passing `--locked` to the first
   command cannot work: it refuses to update a stale lockfile and exits 101.

3. Verify green:

   ```bash
   cargo test
   cargo clippy --all-targets -- -D warnings
   ```

4. Commit, tag, and push. **Set up CI first** (above) — the tag build's publish
   step needs the `codeberg_token` secret:

   ```bash
   git commit -am "Release x.y.z"
   git tag -a vx.y.z -m "pwshark x.y.z"
   git push origin main        # push event: no pipeline (when: only matches tags)
   git push origin vx.y.z       # tag event: builds and publishes the release
   ```

   The tag pipeline builds the binaries and **creates the release with its
   assets** — do not create the release or upload files by hand.

## Manual release (CI unavailable)

Reproduce the CI build locally, then create the release and upload the assets
yourself.

```bash
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
cargo install cargo-zigbuild --locked   # needs zig on PATH — https://ziglang.org/download

cargo zigbuild --release --locked -j 4 \
  --target x86_64-unknown-linux-musl \
  --target aarch64-unknown-linux-musl --bin pwshark

mkdir -p dist
cp target/x86_64-unknown-linux-musl/release/pwshark dist/pwshark-linux-amd64
cp target/aarch64-unknown-linux-musl/release/pwshark dist/pwshark-linux-arm64
cp install.sh dist/install.sh
( cd dist && for f in pwshark-linux-amd64 pwshark-linux-arm64; do sha256sum "$f" > "$f.sha256"; done )
```

Then create a release for the tag on Codeberg (**Releases → New release →**
select `vx.y.z`) and upload every file in `dist/`: the two binaries, their
`.sha256` files, and `install.sh`. Asset names must stay exactly
`pwshark-linux-<arch>` and `pwshark-linux-<arch>.sha256` — `install.sh` resolves
them by that name.

## Verify

On a target device:

```bash
pwshark update
pwshark --version   # prints the new version
```
