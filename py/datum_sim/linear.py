"""Linear analysis of one block's state-space model with python-control: Bode plot and margins,
step response and its specifications, poles and zeros with damping."""
import math

import numpy as np
import control


def _f(v, sig=4):
    """A number for people: up to 4 significant digits; infinity and NaN in words."""
    if v is None:
        return "—"
    v = float(v)
    if math.isnan(v):
        return "—"
    if math.isinf(v):
        return "∞" if v > 0 else "−∞"
    if v == 0:
        return "0"
    a = abs(v)
    if a >= 1e5 or a < 1e-3:
        return f"{v:.{sig - 1}e}"
    return f"{v:.{sig}g}"


def _clean(a):
    """A list JSON can carry (non-finite values become None)."""
    return [float(x) if np.isfinite(x) else None for x in np.asarray(a, dtype=float).ravel()]


def system(q):
    A, B, C, D = (np.array(q.get(k) or [], dtype=float) for k in ("A", "B", "C", "D"))
    if A.size == 0:
        d = D.reshape(1, 1) if D.size else np.zeros((1, 1))
        return control.ss([], [], [], d) if hasattr(control, "ss") else control.tf(d, [1])
    n = A.shape[0]
    B = B.reshape(n, -1)
    C = C.reshape(-1, n)
    D = D.reshape(C.shape[0], B.shape[1]) if D.size else np.zeros((C.shape[0], B.shape[1]))
    return control.ss(A, B, C, D)


def analyse(q):
    sys_ = system(q)
    what = q.get("what", "bode")
    if sys_.ninputs != 1 or sys_.noutputs != 1:
        sys_ = sys_[0, 0]
    note = "The analysis uses the block's first input and first output." if (q.get("B") and len(q["B"][0]) > 1) else ""
    poles = control.poles(sys_)
    re = np.real(poles)
    tol = 1e-9 * max(1.0, float(np.max(np.abs(poles)))) if len(poles) else 0
    stable = bool(np.all(re < -tol)) if len(poles) else True
    verdict = "yes" if stable else "no: a pole in the right half-plane" if np.any(re > tol) else "marginally: a pole on the imaginary axis (an integrator or an undamped mode)"
    facts = [["Order", str(len(poles))], ["Stable", verdict]]
    dc = control.dcgain(sys_)
    facts.append(["Steady-state gain", _f(float(np.real(dc)))])
    out = {"what": what, "control": control.__version__, "note": note}
    if what == "bode":
        w = _freqs(sys_)
        resp = control.frequency_response(sys_, w)
        mag = np.abs(resp.complex).ravel()
        phase = np.unwrap(np.angle(resp.complex).ravel()) * 180 / math.pi
        with np.errstate(divide="ignore"):
            out["mag_db"] = _clean(20 * np.log10(mag))
        out["phase_deg"] = _clean(phase)
        out["w"] = _clean(w)
        try:
            gm, pm, wcg, wcp = control.margin(sys_)
            gm_t = f"{_f(20 * math.log10(gm))} dB at {_f(wcg)} rad/s" if gm and np.isfinite(gm) and gm > 0 and np.isfinite(wcg) else "∞ (the phase never reaches −180°)"
            pm_t = f"{_f(pm)}° at {_f(wcp)} rad/s" if np.isfinite(pm) and np.isfinite(wcp) else "∞ (the gain never crosses 0 dB)"
            facts += [["Gain margin", gm_t], ["Phase margin", pm_t]]
        except Exception:  # margins are undefined for some systems (e.g. a pure gain)
            facts.append(["Margins", "not defined for this system"])
        bw = control.bandwidth(sys_) if stable and np.isfinite(dc) and dc != 0 else float("nan")
        facts.append(["Bandwidth (−3 dB)", f"{_f(bw)} rad/s" if np.isfinite(bw) else "not defined (needs a stable system with a finite, non-zero gain)"])
    elif what == "step":
        if not stable:
            raise ValueError("The step response of an unstable system grows without bound; look at its poles instead.")
        t, y = control.step_response(sys_)
        out["t"], out["y"] = _clean(t), _clean(np.ravel(y))
        facts += _info(control.step_info(sys_))
    else:
        zeros = control.zeros(sys_)
        out["poles"] = [[float(np.real(p)), float(np.imag(p))] for p in poles]
        out["zeros"] = [[float(np.real(z)), float(np.imag(z))] for z in zeros]
        wn, zeta, _ = control.damp(sys_, doprint=False)
        for p, a, b in zip(poles, wn, zeta):
            facts.append([f"Pole {_f(np.real(p))}{'+' if np.imag(p) >= 0 else '−'}{_f(abs(np.imag(p)))}j", f"ωn {_f(a)} rad/s, ζ {_f(b)}"])
        for z in zeros:
            facts.append([f"Zero {_f(np.real(z))}{'+' if np.imag(z) >= 0 else '−'}{_f(abs(np.imag(z)))}j", ""])
    out["facts"] = facts
    return out


def _freqs(sys_):
    """Frequencies two decades either side of the poles and zeros."""
    pts = [abs(x) for x in list(control.poles(sys_)) + list(control.zeros(sys_)) if abs(x) > 1e-12]
    lo = min(pts) / 100 if pts else 1e-2
    hi = max(pts) * 100 if pts else 1e2
    return np.logspace(math.log10(lo), math.log10(hi), 600)


def _info(i):
    return [["Rise time (10–90 %)", f"{_f(i['RiseTime'])} s"], ["Settling time (2 %)", f"{_f(i['SettlingTime'])} s"],
            ["Overshoot", f"{_f(i['Overshoot'])} %"], ["Undershoot", f"{_f(i['Undershoot'])} %"], ["Peak", _f(i['Peak'])],
            ["Peak time", f"{_f(i['PeakTime'])} s"], ["Final value", _f(i['SteadyStateValue'])]]


def step_info(q):
    t = np.asarray(q["t"], dtype=float)
    y = np.asarray(q["y"], dtype=float)
    if len(t) < 3:
        raise ValueError("The signal has too few points.")
    i = control.step_info(y, T=t)
    return {"facts": _info(i), "control": control.__version__}
