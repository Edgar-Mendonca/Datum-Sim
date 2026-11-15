# Plan: scripting, the Datum way (stage S, 0.6.1 → 0.7.0)

Written 2026-11-15 at 0.5.1. Status: **agreed in outline** (the user, 2026-11-15). The details below are my defaults; §6 lists what can still change before 0.6.1.

## 1. Why
The app is built to be used with the mouse, and that stays true. Some users want to type a few lines:
- set gains from a calculation;
- run a model ten ways;
- build a model from a table.

Today they can only do that from outside, with the Python API or the command line. Stage S brings it inside the window, using what the app already ships: Python, numpy, SciPy and python-control.

## 2. What the user gets
- **0.6.1 Python console.** A panel beside the scopes with a prompt.
  - The open model is ready as `model`: `model.vars["Kp"] = 3`, `r = model.run()`, `r["Speed"]` as an array.
  - The control commands are loaded: `tf`, `ss`, `step`, `bode`, `feedback`, `lqr` (from python-control).
  - `plot(t, y)` draws in a Datum scope, not in a separate plotting library.
- **0.6.2 Script editor and setup scripts.**
  - Scripts are `.py` files kept with the model.
  - One script can be marked **setup**: it runs before every simulation, so variables can come from a calculation (for example `K = lqr(A, B, Q, R)`).
  - Errors show the line, in plain words.
- **0.6.3 Build models from code.** `model.add("Gain", k=2)` and `model.connect(a, b)`, with undo like any edit. Scripting examples come with it.
- **0.6.4 Python block.** A block whose output is a Python function of its inputs and time, for things the library lacks. It is slower than built-in blocks, and its ? card says so.
- **0.6.5 Import parameter files (.m).** Plain assignments become model variables:
  - numbers and arithmetic, such as `J = 0.01;` and `wn = 2*pi*50;`;
  - `%` comments are kept as notes.
  - The file is **read, never run**: no loops, functions or toolbox calls. Lines that cannot be read are listed.
  - The public words say "parameter files (.m)", never a product name (FAMILY §13).
- **0.7.0 Milestone.** Verification record and docs.

## 3. How Python is bundled (unchanged)
- Datum Plot does the same, and its `runtime.rs` is ours:
  - `release.yml` takes the official embeddable CPython 3.13 from python.org;
  - it installs `py/requirements.txt` (numpy, SciPy, python-control), runs the Python tests on it, and zips it as `runtime.zip` inside the installer.
- The app unpacks it once into the user's data folder, and again only when `py/runtime.txt` changes. Nothing is downloaded at install or run time. The installer is 86 MB at 0.5.1; Plot's is larger because it also carries matplotlib, statsmodels, openpyxl and lmfit.
- **The console uses this same Python, with no new libraries:**
  - plots go to Datum scopes, so matplotlib is not needed;
  - the installer stays under about 90 MB.
- **The console runs in its own Python process**, separate from the analysis worker:
  - a script that hangs or crashes cannot freeze the window;
  - Stop ends it, and Restart gives a fresh one.
- **The `datum_sim` module gets an in-app mode.** Outside the app it talks to `datum-sim-cli serve`. Inside, it talks to the open model through the shell, so changes made from the console appear on the diagram and can be undone.
- **Later, if asked:** users may add their own packages into a separate folder that updates do not touch. This is not in stage S.

## 4. How it is checked
- Engine and shell:
  - the console protocol (ids, errors, Stop);
  - `model` changes appear as undoable edits;
  - a setup script runs before each run and its error stops the run with a message.
- The Python block: its outputs match a Function block computing the same thing.
- The `.m` reader: a test file with every supported form, and the unsupported lines reported.
- UI checks: the console opens, a command's answer appears, and Stop works. A real-app check on the bundled runtime.

## 5. Licences
- Python (PSF), numpy and SciPy (BSD), python-control (BSD): already in NOTICE.
- No new libraries are planned.
- Nothing from any commercial product is used. The `.m` reader is our own, and it reads the user's own text.

## 6. Still open (my defaults in brackets)
1. Console panel place: [bottom, as a tab beside the scopes].
2. Keep the analysis worker and the console as two processes: [yes].
3. The Python block on every solver: [yes; the variable-step solvers treat it as smooth].
