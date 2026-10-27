# Contributing

Ideas and bug reports are welcome as [issues](https://github.com/Edgar-Mendonca/Datum-Sim/issues/new). File › Roadmap in the app (and `docs/site.json`) shows what is planned.

## How the code is laid out

- `crates/core`: the engine in pure Rust: the model and its edits, the block library, the compiler, the solvers, results, wire routing and sweeps. All calculation lives here, with tests.
- `src-tauri`: the desktop shell (commands, files, the run thread, the Python engine, the updater).
- `ui/src`: the window (SolidJS + TypeScript). It shows what the engine computes and sends edits back as operations.
- `py`: the Python analysis engine (python-control) and its tests.
- `docs/MEMORY.md`: architecture, decisions, roadmap and a log of every session. Read it first.

## Before you send a change

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # engine tests: verification, solver conformance, hardening
python -m unittest discover -s py/tests
npm run check:ui                  # type-check the window
npm run test:ui                   # UI tests (Playwright)
cargo deny check                  # licences, advisories, sources (cargo install cargo-deny)
```

Keep code compact and in the style around it. A new block or solver comes with a test against a known answer; a new window feature with a UI test where it can. Never add GPL, LGPL or AGPL code to the app.

By contributing you agree your work is released under the MIT licence.
