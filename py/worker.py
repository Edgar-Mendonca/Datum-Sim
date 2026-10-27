"""Datum Sim's Python worker (the pattern of Datum Plot's): the app starts it on the first
analysis and sends one JSON request per line on stdin, each with an "id"; each answer is one JSON
line on stdout with the same id ({"ok": true, ...} or {"ok": false, "error": ...}). Long jobs may
send {"id": .., "progress": 0..1, "msg": ..} lines before their answer."""
import json
import os
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def main():
    out = sys.stdout
    sys.stdout = sys.stderr  # stray prints from libraries must not break the protocol

    def send(obj):
        out.write(json.dumps(obj, allow_nan=False, default=str) + "\n")
        out.flush()

    def handle(q):
        cmd = q.get("cmd")
        if cmd == "ping":
            import numpy
            import scipy
            import control
            return {"python": sys.version.split()[0], "numpy": numpy.__version__, "scipy": scipy.__version__, "control": control.__version__}
        if cmd == "linear":
            from datum_sim import linear
            return linear.analyse(q)
        if cmd == "stepinfo":
            from datum_sim import linear
            return linear.step_info(q)
        raise ValueError(f"Unknown request {cmd!r}")

    send({"ok": True, "ready": True})
    for line in sys.stdin:
        if not line.strip():
            continue
        q = {}
        try:
            q = json.loads(line)
            r = {"ok": True, "id": q.get("id"), **handle(q)}
        except Exception as e:  # report every failure to the app instead of dying
            r = {"ok": False, "id": q.get("id"), "error": f"{type(e).__name__}: {e}", "trace": traceback.format_exc(limit=4)}
        try:
            send(r)
        except ValueError as e:  # a number that JSON cannot carry
            send({"ok": False, "id": q.get("id"), "error": f"The result could not be sent: {e}"})


if __name__ == "__main__":
    main()
