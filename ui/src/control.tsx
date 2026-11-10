// The Control tab's dialogs (0.2.3 on): Loop analysis of the whole model between its analysis
// points. The engine linearises and computes every number; this file only draws them.
import { For, Show, createSignal, JSX } from "solid-js";
import uPlot from "uplot";
import { num, invoke } from "./api";
import { openDialog, closeDialog, act, selWires, selBlocks, dview, toast, results, out } from "./store";
import type { OpPoint } from "./api";
import { Dialog, Tabs } from "./ui";
import { OKABE } from "./scope";

/** Tabs of the Loop analysis dialog, in the order they arrived. */
const ALL_TABS: [string, string][] = [["bode", "Bode"], ["nyquist", "Nyquist"], ["rlocus", "Root locus"], ["sens", "Sensitivity"], ["step", "Step and impulse"], ["poles", "Poles"]];
export const LOOP_TABS = ALL_TABS;

/** Put an analysis point on the selected wire's signal. */
export const setPoint = (point: string) => {
  const w = selWires()[0];
  if (!w) return toast("Select a wire first: the analysis point goes on its signal.");
  act({ SetPoint: { id: w.id, point } });
};

const ink = () => getComputedStyle(document.documentElement).getPropertyValue("--text2").trim() || "#3B4859";
const axis = () => ({ stroke: ink(), grid: { stroke: "#E6E8E580", width: 1 }, font: "11px 'IBM Plex Sans', sans-serif" });

/** A line plot over log frequency or linear time. */
const line = (host: HTMLElement, x: number[], ys: [string, (number | null)[]][], o: { log?: boolean; xl: string; yl: string; h?: number }) => {
  const w = Math.max(420, host.clientWidth);
  new uPlot({ width: w, height: o.h ?? 200, scales: { x: { time: false, distr: o.log ? 3 : 1 } }, legend: { show: ys.length > 1 },
    axes: [{ ...axis(), label: o.xl, labelSize: 18 }, { ...axis(), label: o.yl, labelSize: 18, size: 60 }],
    series: [{}, ...ys.map(([label], i) => ({ label, stroke: OKABE[i % OKABE.length], width: 1.6 }))] }, [x, ...ys.map(y => y[1])] as any, host);
};

/** An x–y plot in SVG (Nyquist, root locus, poles): curves and marks, equal-ish scales. */
const xy = (host: HTMLElement, curves: [number, number][][], marks: { p: [number, number]; k: "x" | "o" | "+"; c?: string }[], lim = 0) => {
  const pts = [...curves.flat(), ...marks.map(m => m.p)].filter(p => isFinite(p[0]) && isFinite(p[1]));
  let [x0, x1, y0, y1] = [Math.min(-1, ...pts.map(p => p[0])), Math.max(0.2, ...pts.map(p => p[0])), Math.min(-0.2, ...pts.map(p => p[1])), Math.max(0.2, ...pts.map(p => p[1]))];
  if (lim > 0) { x0 = Math.max(x0, -lim); x1 = Math.min(x1, lim); y0 = Math.max(y0, -lim); y1 = Math.min(y1, lim); }
  const W = 560, H = 340, pad = 30, dx = (x1 - x0) * 0.08 || 1, dy = (y1 - y0) * 0.08 || 1;
  [x0, x1, y0, y1] = [x0 - dx, x1 + dx, y0 - dy, y1 + dy];
  const sx = (v: number) => pad + (v - x0) / (x1 - x0) * (W - 2 * pad), sy = (v: number) => H - pad - (v - y0) / (y1 - y0) * (H - 2 * pad);
  const clip = (p: [number, number]) => isFinite(p[0]) && isFinite(p[1]) && p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1;
  const path = (c: [number, number][]) => { let d = "", pen = false; for (const p of c) { if (!clip(p)) { pen = false; continue; } d += `${pen ? "L" : "M"}${sx(p[0]).toFixed(1)} ${sy(p[1]).toFixed(1)}`; pen = true; } return d; };
  const mk = (m: { p: [number, number]; k: string; c?: string }) => !clip(m.p) ? "" : m.k === "x" ? `<path d="M${sx(m.p[0]) - 4} ${sy(m.p[1]) - 4}l8 8m0 -8l-8 8" stroke="${m.c ?? OKABE[3]}" stroke-width="2"/>` : m.k === "o" ? `<circle cx="${sx(m.p[0])}" cy="${sy(m.p[1])}" r="4" fill="none" stroke="${m.c ?? OKABE[0]}" stroke-width="2"/>` : `<path d="M${sx(m.p[0]) - 6} ${sy(m.p[1])}h12m-6 -6v12" stroke="${m.c ?? "#DC2626"}" stroke-width="2"/>`;
  host.innerHTML = `<svg viewBox="0 0 ${W} ${H}" class="pzplot xyplot"><line x1="${pad}" x2="${W - pad}" y1="${sy(0)}" y2="${sy(0)}" class="pzaxis"/><line x1="${sx(0)}" x2="${sx(0)}" y1="${pad}" y2="${H - pad}" class="pzaxis"/>` +
    curves.map((c, i) => `<path d="${path(c)}" fill="none" stroke="${OKABE[i % OKABE.length]}" stroke-width="1.6"/>`).join("") + marks.map(mk).join("") +
    `<text x="${W - pad}" y="${sy(0) - 4}" text-anchor="end" class="xyt">Re</text><text x="${sx(0) + 4}" y="${pad - 6}" class="xyt">Im</text>` +
    `<text x="${pad}" y="${H - 8}" class="xyt">${num(x0, 3)}</text><text x="${W - pad}" y="${H - 8}" text-anchor="end" class="xyt">${num(x1, 3)}</text><text x="4" y="${pad}" class="xyt">${num(y1, 3)}</text><text x="4" y="${H - pad}" class="xyt">${num(y0, 3)}</text></svg>`;
};

const deg = (v: number | null | undefined) => v == null ? "∞" : `${num(v, 4)}°`;
const rads = (v: number | null | undefined) => v == null ? "" : ` at ${num(v, 4)} rad/s`;

/** Linearise the model at an operating point and look at one input–output pair. */
export const loopAnalysis = (first = "bode", op?: number) => openDialog(() => {
  const [at, setAt] = createSignal(op != null ? `op${op}` : "initial"), [t, setT] = createSignal(1);
  const [inp, setInp] = createSignal(0), [outp, setOutp] = createSignal(0);
  const [tab, setTab] = createSignal(first), [busy, setBusy] = createSignal(false);
  const [r, setR] = createSignal<any>(null);
  const go = async () => {
    setBusy(true);
    try { setR(await invoke<any>("lin_analysis", { at: at() === "time" ? { kind: "time", t: t() } : at().startsWith("op") ? { kind: "op", i: +at().slice(2) } : { kind: "initial" }, inp: inp(), outp: outp() })); }
    catch (e) { setR({ error: String(e) }); }
    finally { setBusy(false); }
  };
  go();
  let host!: HTMLDivElement;
  const draw = () => {
    const x = r();
    if (!host || !x || x.error) return;
    host.innerHTML = "";
    const k = tab();
    if (k === "bode") { line(host, x.w, [["Magnitude", x.mag_db]], { log: true, xl: "Frequency (rad/s)", yl: "dB" }); line(host, x.w, [["Phase", x.phase]], { log: true, xl: "Frequency (rad/s)", yl: "degrees" }); }
    else if (k === "nyquist" && x.nyquist) {
      const pos: [number, number][] = x.nyquist.re.map((v: number, i: number) => [v, x.nyquist.im[i]]);
      xy(host, [pos, pos.map(p => [p[0], -p[1]] as [number, number])], [{ p: [-1, 0], k: "+" }], 6);
    } else if (k === "rlocus" && x.rlocus) {
      const marks = x.rlocus.poles.flatMap((ps: number[][], i: number) => i % 2 ? [] : ps.map(p => ({ p: [p[0], p[1]] as [number, number], k: "o" as const, c: "#94A3B8" })));
      xy(host, [], [...marks, ...x.rlocus.open.map((p: number[]) => ({ p: [p[0], p[1]] as [number, number], k: "x" as const })), ...(x.closed_poles ?? []).map((p: number[]) => ({ p: [p[0], p[1]] as [number, number], k: "+" as const }))]);
    } else if (k === "sens" && x.sens) line(host, x.w, [["S (sensitivity)", x.sens.s_db], ["T (complementary)", x.sens.t_db]], { log: true, xl: "Frequency (rad/s)", yl: "dB", h: 260 });
    else if (k === "step") { const s = x.closed ?? x.open; line(host, s.t, [["Step", s.step]], { xl: "Time (s)", yl: "", h: 200 }); line(host, s.t, [["Impulse", s.impulse]], { xl: "Time (s)", yl: "", h: 160 }); }
    else if (k === "poles") xy(host, [], [...x.poles.map((p: number[]) => ({ p: [p[0], p[1]] as [number, number], k: "x" as const })), ...(x.closed_poles ?? []).map((p: number[]) => ({ p: [p[0], p[1]] as [number, number], k: "o" as const }))]);
  };
  const facts = (): [string, string][] => {
    const x = r();
    if (!x || x.error) return [];
    const f: [string, string][] = [["Linear model", `${x.nx} states, at t = ${num(x.t, 4)} s`]];
    if (x.loop && x.margins) {
      f.push(["Gain margin", x.margins.gm == null ? "∞ (the phase never reaches −180°)" : `${num(x.margins.gm_db, 4)} dB (×${num(x.margins.gm, 4)})${rads(x.margins.wpc)}`]);
      f.push(["Phase margin", x.margins.pm == null ? "∞ (the gain never crosses 0 dB)" : `${deg(x.margins.pm)}${rads(x.margins.wgc)}`]);
      f.push(["Closed loop", x.nyquist.stable ? (x.nyquist.marginal ? "stable (with a pole at the origin or on the imaginary axis: an integrator or angle outside the loop)" : "stable") : `unstable: ${x.nyquist.z} pole(s) in the right half-plane`]);
      f.push(["Nyquist", `${x.nyquist.p} open-loop pole(s) in the right half-plane, ${x.nyquist.n} clockwise encirclement(s) of −1`]);
      f.push(["Peak sensitivity Ms", `${num(x.sens.ms, 4)}${rads(x.sens.w_ms)}`]);
    } else f.push(["Steady-state gain", num(x.dc, 5)]);
    const s = x.closed ?? x.open;
    if (s?.marks) f.push([x.loop ? "Closed-loop step" : "Step", `rise ${num(s.marks.rise, 3)} s, overshoot ${num(s.marks.overshoot, 3)} %, settles in ${num(s.marks.settle, 3)} s`]);
    else if (s?.unstable) f.push(["Step", "grows without bound (unstable)"]);
    return f;
  };
  const copy = () => { const x = r(); if (!x || x.error) return; navigator.clipboard?.writeText(JSON.stringify({ A: x.a, B: x.b, C: x.c, D: x.d, inputs: x.inputs, outputs: x.outputs }, null, 1)).then(() => toast("A, B, C and D copied as JSON.", true)); };
  return <Dialog title="Loop analysis" wide help="loop" extra={<button class="btn" onClick={copy} disabled={!r() || r().error}>Copy A, B, C, D</button>}>
    <div class="lahead form">
      <label>Operating point</label>
      <div class="larow"><select class="laat" value={at()} onChange={e => { setAt(e.currentTarget.value); go(); }}><option value="initial">Initial state</option><option value="time" disabled={!results()}>Time in the last run</option><For each={out()?.ops ?? []}>{(o, i) => <option value={`op${i()}`}>Operating point: {o.name}</option>}</For></select>
        <Show when={at() === "time"}><input class="mono latime" value={t()} onChange={e => { const v = +e.currentTarget.value; if (isFinite(v)) { setT(v); go(); } }} /> s</Show></div>
      <Show when={r() && !r().error}>
        <label>From</label><div><select class="lain" value={inp()} onChange={e => { setInp(+e.currentTarget.value); go(); }}><For each={r().inputs}>{(n: string, i) => <option value={i()}>{n}</option>}</For></select></div>
        <label>To</label><div><select class="laout" value={outp()} onChange={e => { setOutp(+e.currentTarget.value); go(); }}><For each={r().outputs}>{(n: string, i) => <option value={i()}>{n}</option>}</For></select></div>
      </Show>
    </div>
    <Show when={!busy() || r()} fallback={<p class="muted"><i class="spin" /> Linearising…</p>}>
      <Show when={r() && !r().error} fallback={<p class="issue">{r()?.error}</p>}>
        <Tabs tabs={LOOP_TABS.filter(([k]) => r().loop || !["nyquist", "rlocus", "sens"].includes(k))} on={tab()} set={k => { setTab(k); queueMicrotask(draw); }} />
        <div class="anplot" ref={el => { host = el; queueMicrotask(draw); }} />
        <table class="tbl facts"><tbody><For each={facts()}>{f => <tr><td>{f[0]}</td><td class="r mono">{f[1]}</td></tr>}</For></tbody></table>
        <For each={r().notes}>{(n: string) => <p class="muted small">{n}</p>}</For>
        <Show when={!r().loop}><p class="muted small">Margins, Nyquist, sensitivity and the root locus are for a loop: choose a loop opening as both From and To.</p></Show>
      </Show>
    </Show>
  </Dialog>;
});

/** Control › Trim: find where the model rests with chosen outputs held. */
export const trimDialog = () => openDialog(() => {
  const [info, setInfo] = createSignal<[string[], number] | null>(null), [err, setErr] = createSignal("");
  const [hold, setHold] = createSignal<Record<string, string>>({}), [free, setFree] = createSignal<string[]>([]);
  const [res, setRes] = createSignal<any>(null), [busy, setBusy] = createSignal(false), [name, setName] = createSignal("");
  invoke<[string[], number]>("trim_info").then(setInfo).catch(e => setErr(String(e)));
  const find = async () => {
    setBusy(true); setRes(null);
    const h = Object.entries(hold()).filter(([, v]) => v.trim() !== "" && isFinite(+v)).map(([k, v]) => [k, +v]);
    try { setRes(await invoke<any>("trim", { spec: { hold: h, free: free(), t: 0 } })); } catch (e) { setRes({ error: String(e) }); } finally { setBusy(false); }
  };
  const save = () => {
    const r = res(); if (!r || r.error) return;
    const n = name().trim() || `Point ${(out()?.ops?.length ?? 0) + 1}`;
    act({ SetOps: { ops: [...(out()?.ops ?? []), { ...r.op, name: n }] } }).then(() => { toast(`Operating point “${n}” saved.`, true); closeDialog(); });
  };
  return <Dialog title="Trim: find an operating point" wide help="trim">
    <Show when={!err()} fallback={<p class="issue">{err()}</p>}>
      <Show when={info()} fallback={<p class="muted"><i class="spin" /> Reading the model…</p>}>
        <p class="muted small">Trim finds the states where every derivative is zero (the model at rest), with the outputs you choose held at their values. Outputs are the model's analysis-point outputs; free variables are model variables trim may change (a set point, a load, a supply).</p>
        <h4>Hold these outputs</h4>
        <Show when={info()![0].length} fallback={<p class="muted">No outputs: mark a wire as an analysis Output (Control › Output) to hold its value.</p>}>
          <table class="tbl"><tbody><For each={info()![0]}>{n => <tr><td>{n}</td><td><input class="mono trimhold" placeholder="free" value={hold()[n] ?? ""} onInput={e => setHold({ ...hold(), [n]: e.currentTarget.value })} /></td></tr>}</For></tbody></table>
        </Show>
        <h4>Variables trim may change</h4>
        <Show when={out()?.vars.length} fallback={<p class="muted">The model has no variables.</p>}>
          <div class="trimvars"><For each={out()!.vars}>{v => <label class="chk"><input type="checkbox" checked={free().includes(v.name)} onChange={e => setFree(e.currentTarget.checked ? [...free(), v.name] : free().filter(x => x !== v.name))} />{v.name} <span class="muted">= {v.expr}</span></label>}</For></div>
        </Show>
        <p><button class="btn pri trimgo" disabled={busy()} onClick={find}>{busy() ? "Searching…" : "Find operating point"}</button></p>
        <Show when={res()}><Show when={!res().error} fallback={<p class="issue">{res().error}</p>}>
          <p class={res().met ? "ok" : "issue"}>{res().met ? `Found in ${res().iters} iterations: the model is at rest.` : "Not found exactly:"}</p>
          <For each={res().unmet}>{(u: string) => <p class="issue small">{u}</p>}</For>
          <table class="tbl facts"><tbody>
            <For each={res().outputs}>{(o: [string, number]) => <tr><td>{o[0]}</td><td class="r mono">{num(o[1], 6)}</td></tr>}</For>
            <For each={res().op.vars}>{(o: [string, number]) => <tr><td>Variable {o[0]}</td><td class="r mono">{num(o[1], 6)}</td></tr>}</For>
            <tr><td>States</td><td class="r mono">{res().op.x.length}</td></tr>
          </tbody></table>
          <div class="larow"><input class="trimname" placeholder="Name, e.g. 2 m level" value={name()} onInput={e => setName(e.currentTarget.value)} /><button class="btn pri trimsave" onClick={save}>Save operating point</button></div>
        </Show></Show>
      </Show>
    </Show>
  </Dialog>;
});

/** Control › Operating Points: the saved points; linearise at one or delete it. */
export const opsDialog = () => openDialog(() => {
  const ops = () => out()?.ops ?? [];
  const del = (i: number) => act({ SetOps: { ops: ops().filter((_, k) => k !== i) } });
  return <Dialog title="Operating points" help="trim">
    <Show when={ops().length} fallback={<p class="muted">No operating points yet. Control › Trim finds one and saves it here.</p>}>
      <table class="tbl"><thead><tr><th>Name</th><th>Holds</th><th>Variables</th><th /></tr></thead><tbody><For each={ops()}>{(o: OpPoint, i) => <tr>
        <td>{o.name}</td><td class="muted">{o.note}</td><td class="mono small">{o.vars.map(v => `${v[0]} = ${num(v[1], 4)}`).join(", ")}</td>
        <td><button class="btn sm" onClick={() => { closeDialog(); setTimeout(() => loopAnalysis("bode", i())); }}>Loop analysis here</button> <button class="btn sm" onClick={() => del(i())}>Delete</button></td></tr>}</For></tbody></table>
    </Show>
  </Dialog>;
});

/** The edits that give a PID block these gains: a gain that is a model variable changes the
 *  variable (in `vars`, returns true), any other is set on the block. */
const gainOps = (b: { id: number; params?: Record<string, string> }, pid: Record<string, number>, vars: any[], ops: any[]) => {
  let vc = false;
  for (const k of ["P", "I", "D", "N"]) {
    const v = pid[k.toLowerCase()];
    if (!isFinite(v)) continue;
    const text = (b.params?.[k] ?? "").trim(), vi = vars.findIndex(x => x.name === text);
    const val = String(+v.toPrecision(6));
    if (vi >= 0) { vars[vi] = { ...vars[vi], expr: val }; vc = true; }
    else ops.push({ SetParam: { id: b.id, key: k, value: val } });
  }
  return vc;
};

const MOTORS = ["DCMotor", "BLDC", "PMSM", "PMSM3"];

/** Control › Drive Tuning: the current and speed PIs of a drive from the motor's data. */
export const driveTuner = () => {
  const all = dview()?.blocks ?? [];
  const motors = all.filter(b => MOTORS.includes(b.kind));
  const mo = selBlocks().find(b => MOTORS.includes(b.kind)) ?? (motors.length === 1 ? motors[0] : undefined);
  if (!mo) return toast(motors.length ? "Select the motor block of the drive." : "This diagram has no DC, BLDC or PMSM motor block: open the diagram that holds it.");
  const pids = all.filter(b => b.kind === "PID" || b.kind === "DiscretePID");
  const guess = (re: RegExp) => pids.find(b => re.test(b.name))?.id ?? -1;
  openDialog(() => {
    const [wi, setWi] = createSignal(2000), [ws, setWs] = createSignal(200), [tq, setTq] = createSignal(false);
    const [sp, setSp] = createSignal(guess(/speed|ω|omega|velocity/i)), [iq, setIq] = createSignal(guess(/^i_?q\b|q.?current|q.?axis|^current/i)), [idd, setId] = createSignal(guess(/^i_?d\b|d.?current|d.?axis/i));
    const [r, setR] = createSignal<any>(null);
    const go = () => invoke<any>("drive_tune", { id: mo.id, wcI: wi(), wcS: ws(), torqueOut: tq() }).then(setR).catch(e => setR({ error: String(e) }));
    go();
    const pick = (v: () => number, set: (n: number) => void, cls: string) => <select class={cls} value={v()} onChange={e => set(+e.currentTarget.value)}><option value={-1}>(not set)</option><For each={pids}>{b => <option value={b.id}>{b.name}</option>}</For></select>;
    const apply = () => {
      const x = r(); if (!x || x.error) return false;
      const vars = [...(out()?.vars ?? [])], ops: any[] = [];
      let vc = false;
      for (const [id, g] of [[sp(), x.tune.speed], [iq(), x.tune.iq], [idd(), x.tune.id]] as [number, any][]) {
        const b = pids.find(p => p.id === id);
        if (b) vc = gainOps(b, { ...g, d: 0 }, vars, ops) || vc;
      }
      if (!ops.length && !vc) { toast("Choose which PID is the speed loop and which the current loop."); return false; }
      if (vc) ops.push({ SetVars: { vars } });
      act({ Batch: ops }).then(() => toast("Drive tuned: one undoable edit.", true));
    };
    const g = (p: any) => p ? `P ${num(p.p, 4)} · I ${num(p.i, 4)}` : "";
    const mm = (m: any) => m ? `crossover ${num(m.wgc, 4)} rad/s, phase margin ${num(m.pm, 3)}°` : "";
    return <Dialog title={`Drive Tuning: ${mo.name}`} wide help="drivetune" ok={apply} okText="Apply">
      <div class="form">
        <label>Current loop</label><div class="larow"><input class="mono dtwi" value={wi()} onChange={e => { setWi(+e.currentTarget.value); go(); }} /> rad/s</div>
        <label>Speed loop</label><div class="larow"><input class="mono dtws" value={ws()} onChange={e => { setWs(+e.currentTarget.value); go(); }} /> rad/s <span class="muted small">(about a tenth of the current loop)</span></div>
        <span /><div><label class="chk"><input type="checkbox" checked={tq()} onChange={e => { setTq(e.currentTarget.checked); go(); }} />The speed PI gives a torque (not a current) reference</label></div>
        <label>Speed PID</label><div>{pick(sp, setSp, "dtsp")}</div>
        <label>q (or armature) current PID</label><div>{pick(iq, setIq, "dtiq")}</div>
        <label>d current PID</label><div>{pick(idd, setId, "dtid")}</div>
      </div>
      <Show when={r() && !r().error} fallback={<p class="issue">{r()?.error}</p>}>
        <table class="tbl facts"><tbody>
          <tr><td>Motor data</td><td class="r mono">R {num(r().data.r, 4)} Ω · Ld {num(r().data.ld, 4)} H · Lq {num(r().data.lq, 4)} H · Kt {num(r().data.kt, 4)} N·m/A · J {num(r().data.j, 4)} kg·m²</td></tr>
          <tr><td>Current PIs</td><td class="r mono">q: {g(r().tune.iq)} · d: {g(r().tune.id)}</td></tr>
          <tr><td>Speed PI</td><td class="r mono">{g(r().tune.speed)}</td></tr>
          <tr><td>Current loop</td><td class="r mono">{mm(r().tune.current_margins)}</td></tr>
          <tr><td>Speed loop</td><td class="r mono">{mm(r().tune.speed_margins)}</td></tr>
        </tbody></table>
        <p class="muted small">Current PIs cancel the winding's pole (P = L·ωc, I = R·ωc), so the current loop is ωc/s. The speed PI sees Kt/(J s + b) through the closed current loop, with its zero at a quarter of the speed bandwidth. Check the result with a run: limits, sampling and the inverter are not in this design.</p>
      </Show>
    </Dialog>;
  });
};

/** Control › PID Tuner: tune a PID with two sliders (speed, robustness) or a classic rule. */
export const pidTuner = () => {
  const pids = (dview()?.blocks ?? []).filter(b => b.kind === "PID" || b.kind === "DiscretePID");
  const sel = selBlocks().find(b => b.kind === "PID" || b.kind === "DiscretePID") ?? (pids.length === 1 ? pids[0] : undefined);
  if (!sel) return toast(pids.length ? "Select the PID block to tune." : "This diagram has no PID block: open the diagram that holds it.");
  openDialog(() => {
    const [plant, setPlant] = createSignal<any>(null), [err, setErr] = createSignal("");
    const [rule, setRule] = createSignal("shape"), [form, setForm] = createSignal("auto");
    const [lw, setLw] = createSignal(0), [pm, setPm] = createSignal(60), [res, setRes] = createSignal<any>(null);
    invoke<any>("pid_plant", { id: sel.id, at: { kind: "initial" } }).then(p => { setPlant(p); setLw(Math.log10(p.wgc > 0 ? p.wgc : 1)); design(); }).catch(e => setErr(String(e)));
    let seq = 0;
    const design = async () => {
      const p = plant(); if (!p) return;
      const my = ++seq;
      try { const r = await invoke<any>("pid_design", { plant: p.plant, cur: p.cur, rule: rule(), wc: 10 ** lw(), pm: pm(), form: form() }); if (my === seq) { setRes(r); queueMicrotask(draw); } }
      catch (e) { if (my === seq) setRes({ error: String(e) }); }
    };
    let host!: HTMLDivElement;
    const draw = () => {
      const r = res()?.report; if (!host || !r) return;
      host.innerHTML = "";
      const s0 = r.step0, s1 = r.step1;
      if (s1) line(host, s1.t, [["Before", s0 && !s0.unstable ? s0.y : s1.t.map(() => null)], ["After", s1.y]], { xl: "Time (s)", yl: "closed-loop step", h: 200 });
      line(host, r.w, [["Before", r.mag0], ["After", r.mag1]], { log: true, xl: "Frequency (rad/s)", yl: "|L| dB", h: 170 });
    };
    const m = (x: any) => x ? `phase margin ${x.pm == null ? "∞" : num(x.pm, 3) + "°"}, gain margin ${x.gm_db == null ? "∞" : num(x.gm_db, 3) + " dB"}${x.wgc == null ? "" : `, crossover ${num(x.wgc, 3)} rad/s`}` : "";
    const apply = () => {
      const r = res(); if (!r || r.error) return;
      const vars = [...(out()?.vars ?? [])], ops: any[] = [];
      const vc = gainOps(sel, r.pid, vars, ops);
      if (vc) ops.push({ SetVars: { vars } });
      act({ Batch: ops }).then(() => { toast(`${sel.name} tuned: one undoable edit.`, true); closeDialog(); });
    };
    return <Dialog title={`PID Tuner: ${sel.name}`} wide help="pidtune" ok={apply} okText="Apply">
      <Show when={!err()} fallback={<p class="issue">{err()}</p>}>
        <Show when={plant()} fallback={<p class="muted"><i class="spin" /> Finding the plant the PID sees…</p>}>
          <div class="form pidform">
            <label>Rule</label><div><select class="pidrule" value={rule()} onChange={e => { setRule(e.currentTarget.value); design(); }}><option value="shape">Speed and robustness (loop shaping)</option><option value="zn">Ziegler–Nichols (ultimate gain)</option><option value="simc">SIMC (first order plus dead time)</option></select></div>
            <Show when={rule() === "shape"}>
              <label>Speed</label><div class="larow"><input type="range" class="pidspeed" min={lw() - 2} max={lw() + 2} step="0.01" value={lw()} onInput={e => { setLw(+e.currentTarget.value); design(); }} /><span class="mono">{num(10 ** lw(), 3)} rad/s</span></div>
              <label>Robustness</label><div class="larow"><input type="range" class="pidrob" min="30" max="85" step="1" value={pm()} onInput={e => { setPm(+e.currentTarget.value); design(); }} /><span class="mono">{pm()}° phase margin</span></div>
              <label>Controller</label><div><select value={form()} onChange={e => { setForm(e.currentTarget.value); design(); }}><option value="auto">PI or PID, as needed</option><option value="PI">PI</option><option value="PID">PID</option></select></div>
            </Show>
          </div>
          <Show when={res() && !res().error} fallback={<p class="issue">{res()?.error}</p>}>
            <table class="tbl facts pidgains"><thead><tr><th /><th>P</th><th>I</th><th>D</th><th>N</th></tr></thead><tbody>
              <tr><td>Now</td><For each={["p", "i", "d", "n"]}>{k => <td class="r mono">{num(plant().cur[k], 4)}</td>}</For></tr>
              <tr><td><b>Tuned</b></td><For each={["p", "i", "d", "n"]}>{k => <td class="r mono"><b>{num(res().pid[k], 4)}</b></td>}</For></tr></tbody></table>
            <p class="small">Before: {m(res().report.before)}.<br /><b>After: {m(res().report.after)}.</b></p>
            <p class="muted small">{res().rule}. {res().note}</p>
            <div class="anplot" ref={el => { host = el; queueMicrotask(draw); }} />
          </Show>
          <p class="muted small">The plant is linearised with the loop cut at {sel.name}'s output, at the model's initial state. Apply sets P, I, D and N (or the variables they use) in one undoable edit.{sel.kind === "DiscretePID" ? " The gains are designed in continuous time: good when the sample time is well below 1/crossover." : ""}</p>
        </Show>
      </Show>
    </Dialog>;
  });
};

// ---------------------------------------------------------------- live tuning (0.2.8)

const [liveOpen, setLiveOpen] = createSignal(false);
export { liveOpen, setLiveOpen };
const [liveVals, setLiveVals] = createSignal<Record<string, number>>({});

/** Mark or unmark a variable as tunable (a slider in the Live panel), with a range around it. */
const setTune = (name: string, on: boolean) => {
  const vars = (out()?.vars ?? []).map(v => {
    if (v.name !== name) return v;
    const x = +v.expr, a = isFinite(x) && x !== 0 ? Math.abs(x) : 1;
    return on ? { ...v, tune: true, lo: (v as any).lo || (x >= 0 ? 0 : -2 * a), hi: (v as any).hi || (x >= 0 ? 2 * a : 0) } : { ...v, tune: false };
  });
  act({ SetVars: { vars } });
};

/** The Live panel: a slider per tunable variable. While the model runs a slider changes the running
 *  model at once; "Keep these values" writes them to the variables (one undoable edit). */
export const LivePanel = () => {
  const tun = () => (out()?.vars ?? []).filter((v: any) => v.tune);
  const plain = () => (out()?.vars ?? []).filter((v: any) => !v.tune && isFinite(+v.expr));
  const val = (v: any) => liveVals()[v.name] ?? +v.expr;
  const move = (v: any, x: number) => {
    setLiveVals({ ...liveVals(), [v.name]: x });
    if (out()?.running) invoke("live_set", { name: v.name, value: x }).catch(e => toast(String(e)));
  };
  const keep = () => {
    const lv = liveVals();
    act({ SetVars: { vars: (out()?.vars ?? []).map(v => v.name in lv ? { ...v, expr: String(+lv[v.name].toPrecision(6)) } : v) } }).then(() => { setLiveVals({}); toast("Values kept in the model's variables.", true); });
  };
  const range = (v: any, k: "lo" | "hi", x: string) => { const n = +x; if (isFinite(n)) act({ SetVars: { vars: (out()?.vars ?? []).map(w => w.name === v.name ? { ...w, [k]: n } : w) } }); };
  return <Show when={liveOpen()}>
    <div class="livepanel">
      <header><b>Live tuning</b><span class="muted small">{out()?.running ? "running: changes apply at once" : "run the model, then move a slider"}</span><button class="x" onClick={() => setLiveOpen(false)}>×</button></header>
      <Show when={tun().length} fallback={<p class="muted small">No tunable variables yet: add one below.</p>}>
        <For each={tun()}>{(v: any) => <div class="lrow">
          <label class="mono">{v.name}</label>
          <input class="mono lnum" value={v.lo} onChange={e => range(v, "lo", e.currentTarget.value)} title="Lowest value" />
          <input type="range" class="lslider" min={v.lo} max={v.hi} step={(v.hi - v.lo) / 200 || 0.01} value={val(v)} onInput={e => move(v, +e.currentTarget.value)} />
          <input class="mono lnum" value={v.hi} onChange={e => range(v, "hi", e.currentTarget.value)} title="Highest value" />
          <span class="mono lval">{num(val(v), 4)}</span>
          <button class="btn sm" title="Stop tuning this variable" onClick={() => setTune(v.name, false)}>×</button>
        </div>}</For>
      </Show>
      <div class="larow">
        <select class="laddvar" onChange={e => { if (e.currentTarget.value) setTune(e.currentTarget.value, true); e.currentTarget.value = ""; }}><option value="">Add a variable…</option><For each={plain()}>{(v: any) => <option value={v.name}>{v.name} = {v.expr}</option>}</For></select>
        <span class="grow" />
        <button class="btn sm pri" disabled={!Object.keys(liveVals()).length} onClick={keep}>Keep these values</button>
      </div>
    </div>
  </Show>;
};

/** The analysis point glyph on a wire: arrow in, a measuring circle, or a cut. */
export const PointGlyph = (p: { kind: string; at: [number, number] }): JSX.Element => {
  const [x, y] = p.at;
  return <g class={`apt apt-${p.kind}`} transform={`translate(${x} ${y})`}>
    <title>{p.kind === "in" ? "Analysis input: a test signal is added here" : p.kind === "out" ? "Analysis output: measured here" : "Loop opening: the loop is cut here for analysis"}</title>
    <circle r="7" />
    {p.kind === "in" ? <path d="M-3 -4L3 0L-3 4z" /> : p.kind === "out" ? <circle r="2.6" class="dot" /> : <path d="M-3.5 -3.5L3.5 3.5M3.5 -3.5L-3.5 3.5" />}
  </g>;
};
