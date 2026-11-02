"""Datum Sim from Python: open, change, run and sweep models and get the results as NumPy arrays.

    import datum_sim as ds
    m = ds.open("motor.dsim")            # or ds.example("dcmotor")
    m.set(Kp=120)                         # model variables
    m.set_param("PID", "D", 5)            # a block parameter, by name or path ("Motor/Gain")
    r = m.run(stop=5)                     # Results
    r.t, r["DC motor/ω"]                  # NumPy arrays
    for value, r in m.sweep("Kp", [50, 100, 150]): ...

It talks to the engine through the command-line program `datum-sim-cli` (its `serve` mode), so
results are exactly the app's. The program is found from DATUM_SIM_CLI, the PATH, the installed
app, or a build of this repository.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np

__all__ = ["open", "example", "examples", "Model", "Results", "DatumSimError", "find_cli"]


class DatumSimError(RuntimeError):
    """The engine refused a request (an unknown block, a model error, a bad value...)."""


def find_cli():
    """The path of datum-sim-cli, or raise with what to do."""
    exe = "datum-sim-cli.exe" if os.name == "nt" else "datum-sim-cli"
    cands = [os.environ.get("DATUM_SIM_CLI"), shutil.which("datum-sim-cli")]
    if os.name == "nt":
        for base in (os.environ.get("LOCALAPPDATA"), os.environ.get("ProgramFiles")):
            if base:
                cands.append(os.path.join(base, "Datum Sim", exe))
    # Next to this package in the installed app (resources/py/datum_sim), or a repository build.
    here = Path(__file__).resolve()
    for up in here.parents[:4]:
        cands += [str(up / exe), str(up / "target" / "release" / exe), str(up / "target" / "debug" / exe)]
    for c in cands:
        if c and os.path.isfile(c):
            return c
    raise DatumSimError(
        "datum-sim-cli was not found. Install Datum Sim, or set DATUM_SIM_CLI to the program "
        "(in the app's folder, e.g. %LOCALAPPDATA%\\Datum Sim\\datum-sim-cli.exe)."
    )


class _Engine:
    """One `datum-sim-cli serve` process: JSON requests in, answers out."""

    def __init__(self, cli=None):
        self.p = subprocess.Popen(
            [cli or find_cli(), "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, encoding="utf-8", bufsize=1,
        )
        self.n = 0

    def ask(self, cmd, **args):
        self.n += 1
        self.p.stdin.write(json.dumps({"id": self.n, "cmd": cmd, **args}) + "\n")
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            raise DatumSimError("The engine stopped unexpectedly.")
        a = json.loads(line)
        if not a.get("ok"):
            raise DatumSimError(a.get("error", "unknown error"))
        return a["result"]

    def close(self):
        if self.p.poll() is None:
            self.p.stdin.close()
            try:
                self.p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.p.kill()
        if self.p.stdout and not self.p.stdout.closed:
            self.p.stdout.close()


class Results:
    """One run: `t` and each signal as NumPy arrays, by the names in the scope legends."""

    def __init__(self, d):
        self.t = np.asarray(d["t"], dtype=float)
        self.names = [s["name"] for s in d["signals"]]
        self._y = {s["name"]: np.asarray(s["y"], dtype=float) for s in d["signals"]}
        self.stats = d.get("stats", {})
        self.truncated = d.get("truncated", False)

    def __getitem__(self, name):
        try:
            return self._y[name]
        except KeyError:
            raise KeyError(f"No signal {name!r}. Signals: {', '.join(self.names)}") from None

    def __iter__(self):
        return iter(self.names)

    def __len__(self):
        return len(self.names)

    def final(self, name):
        """The value at the end of the run."""
        return float(self[name][-1])

    def at(self, name, t):
        """The value at time t (linear between logged points)."""
        return float(np.interp(t, self.t, self[name]))

    def to_dict(self):
        return {"time": self.t, **self._y}

    def to_pandas(self):
        """A DataFrame indexed by time (needs pandas)."""
        import pandas as pd
        return pd.DataFrame(self._y, index=pd.Index(self.t, name="time"))

    def __repr__(self):
        return f"<Results {len(self.t)} points, {len(self.names)} signals: {', '.join(self.names)}>"


class Model:
    """A model held by the engine. Changes stay in memory until `save`."""

    def __init__(self, path=None, example=None, cli=None):
        self._e = _Engine(cli)
        if path is not None:
            self._e.ask("open", path=str(Path(path).resolve()))
        else:
            self._e.ask("new", **({"example": example} if example else {}))

    # ---- reading the model
    def info(self):
        """Name, variables, run settings, every block (path, kind, parameters) and the signals."""
        return self._e.ask("info")

    @property
    def variables(self):
        return {v["name"]: v["expr"] for v in self.info()["vars"]}

    @property
    def blocks(self):
        return {b["path"]: b for b in self.info()["blocks"]}

    @property
    def signals(self):
        """The signals a run logs (into scopes; every signal after log_all=True)."""
        return self.info()["signals"]

    # ---- changing it
    def set(self, **variables):
        """Set model variables: m.set(Kp=120, Ki="Kp/4"). New names are added."""
        for k, v in variables.items():
            self._e.ask("set_var", name=k, value=v)
        return self

    def set_param(self, block, key, value):
        """Set a block parameter: m.set_param("PID", "P", 120) or ("Motor/Gain", "k", "2*R")."""
        self._e.ask("set_param", block=block, key=key, value=value)
        return self

    def settings(self, **sim):
        """Run settings: start, stop, solver ("rk45", "stiff", "rk4", "heun", "euler"), step,
        max_step, rtol, atol, log_all."""
        self._e.ask("set_sim", sim=sim)
        return self

    def check(self):
        """The model's errors and warnings: {"ok": bool, "issues": [...]}."""
        return self._e.ask("check")

    # ---- running it
    def run(self, signals=None, points=0, **sim):
        """Run with the current settings (and any given here: stop=5, solver="stiff", ...).
        `signals` picks signals by name; `points` reduces the results (peaks kept)."""
        if sim:
            self.settings(**sim)
        q = {"points": int(points)}
        if signals:
            q["signals"] = [signals] if isinstance(signals, str) else list(signals)
        return Results(self._e.ask("run", **q))

    def sweep(self, var, values, signals=None, points=0):
        """Run once per value of a model variable (in parallel). Yields (value, Results)."""
        q = {"var": var, "values": [float(v) for v in values], "points": int(points)}
        if signals:
            q["signals"] = [signals] if isinstance(signals, str) else list(signals)
        return [(r["value"], Results(r["results"])) for r in self._e.ask("sweep", **q)["runs"]]

    def save(self, path):
        self._e.ask("save", path=str(Path(path).resolve()))
        return self

    def close(self):
        self._e.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    def __del__(self):
        try:
            self.close()
        except Exception:
            pass


def open(path, cli=None):  # noqa: A001 (the natural name, as in other tools)
    """Open a .dsim model."""
    return Model(path=path, cli=cli)


def example(name, cli=None):
    """A worked example by id (see examples())."""
    return Model(example=name, cli=cli)


def examples(cli=None):
    """The worked examples: [(id, name), ...]."""
    e = _Engine(cli)
    try:
        return [(x["id"], x["name"]) for x in e.ask("examples")]
    finally:
        e.close()


if __name__ == "__main__":
    # python -m datum_sim.api MODEL: print a quick summary (handy to check the setup).
    m = open(sys.argv[1]) if len(sys.argv) > 1 else example("msd")
    r = m.run()
    print(r)
    for n in r:
        print(f"  {n}: final {r.final(n):.6g}")
