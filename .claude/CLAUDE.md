# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project

Stardrift is a 3D gravitational N-body simulation built with Rust and Bevy —
an experimental learning project, not for scientific use. It is in alpha:
backwards compatibility is a very low priority.

The reference documentation under `docs/` is maintained and accurate — use it
instead of rediscovering things:

- `docs/architecture.md` — plugin structure, physics engine, project layout
- `docs/integration.md` — the staged integration protocol and its guarantees
- `docs/configuration.md` — config file and all defaults
- `docs/usage.md`, `docs/integrators.md`, `docs/color-schemes.md`, `docs/release.md`

`docs/log/` (devlogs) and `docs/plans/` are gitignored and local-only; all
other docs are tracked normally.

## Commands

```bash
cargo run -p stardrift              # dev build (add --release for performance)
cargo run -p stardrift -- --help    # CLI is self-documenting
cargo test                          # correctness suites live in tests/
cargo bench                         # criterion benches (octree, simulation_step)
cargo fmt && cargo clippy           # before committing
trunk serve                         # WASM dev server (trunk build --release to dist/)
```

Deterministic repro for debugging:
`cargo run -p stardrift -- --seed 42 --screenshot-after 60 --screenshot-use-frames --exit-after-screenshots`

## Conventions

- Conventional commit format for all commits.
- Significant changes get a `CHANGELOG.md` entry under `[Unreleased]`.
- Update `README.md` only for user-facing changes; it is kept intentionally
  small. Keep `docs/` in sync with behavior changes; verify stated defaults
  against `src/config.rs` rather than trusting prose.
- Physics stays f64: use the `Scalar` and `Vector` aliases from
  `src/physics/math.rs`.
- Use the `SharedRng` resource for randomness (determinism under `--seed`).
- Plugins are self-contained and communicate via `SimulationCommand` messages
  (`src/messages.rs`), not by reaching into each other.
- New integrators must be registered in
  `src/physics/integrators/registry.rs` AND characterized in
  `tests/integrator_correctness.rs` (the registry integrity test fails until
  the expectation-table entry exists).
- Dependency versions: check `Cargo.toml`, don't assume (Bevy tracks recent
  releases; APIs move).
