# Datum Sim: notes for Claude

Read `docs/MEMORY.md` first. It is the project's memory: architecture, decisions, roadmap, and a log of every session. The family design guide is `FAMILY.md` in `Edgar-Mendonca/Datum` (read-only from here; `docs/FAMILY.md` links to it).
At the end of every session that changes anything, update `docs/MEMORY.md`:
- tick the roadmap
- add a session-log entry
- record any new decisions
- refresh the architecture section if files moved

User preferences:
- Rust and Tauri 2, small and efficient code, Windows first. The same look, ribbon, help, printing and updater as every Datum app (FAMILY §2–9); copy from Datum Plan rather than reinventing.
- Ship in small numbered releases (0.1.0, 0.1.1, …). Every release appears in the in-app updater.
- Engine logic belongs in `crates/core` (pure Rust, unit-tested), not in the UI. The window never calculates.
- The UI is SolidJS + Vite in `ui/src/*.tsx` (built to `ui/dist`). State lives in `ui/src/store.ts`.
- Big asks get a `docs/PLAN-x.y.md` and questions first; build after the answers.
- Public words (README, CHANGELOG, site.json, help): never name, compare with, or say the app copies or replaces a commercial product (FAMILY §13).
- Licences: check every dependency (MIT/BSD/Apache/MPL only in the app; no GPL, LGPL or AGPL linked in). Record each one in `NOTICE`. Note `evalexpr` ≥ 12 is AGPL: do not add it.

How we build (see PLAN-0.1 §2):
- Layers point inward: `crates/core` ← `src-tauri` ← `ui`. One owner per meaning (block registry, units, sizes). Expected failures are typed errors, never panics.
- Every block and solver has a test against a known answer: solver conformance in `crates/core/src/solve.rs`, blocks and models in `crates/core/src/verify.rs`, and a dated record in `docs/verification/`.
- Report only checks that actually ran; say what was not verified.

Checks before pushing:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (includes the `hardening` and verification tests)
- `python -m unittest discover -s py/tests` (needs `pip install -r py/requirements.txt`)
- `npm run check:ui`, then `npm run test:ui` (dumps the engine fixture, builds the window, runs the Playwright checks)
- `cargo deny check` (licences, advisories, sources; CI runs it)
- A change you can see: check it in the real app (`cargo build -p datum-sim && tests_ui/realapp.sh steps.sh shots/`, see the script) and add a UI check for every bug

Releasing:
1. Bump the version in `src-tauri/Cargo.toml` and `crates/core/Cargo.toml`.
2. Add a CHANGELOG section (`## x.y.z — date`).
3. Push the tag `vX.Y.Z`, or run `release.yml` with workflow_dispatch when tag pushes are blocked. The workflow creates the tag and publishes `DatumSim-x.y.z-setup.exe` and `SHA256SUMS.txt`.
4. After the first release: tell the user to add Datum Sim to the hub's `apps.json` and its icon to the hub's `brand/` (in the hub chat).
5. A change to `py/requirements.txt` needs `py/runtime.txt` bumped, so installed apps unpack the new runtime.
