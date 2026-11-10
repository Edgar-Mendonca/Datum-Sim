// The Control tab's dialogs (0.2.3 on): Loop analysis of the whole model between its analysis
// points. The engine linearises and computes every number; this file only draws them.
import { For, Show, createSignal, JSX } from "solid-js";
import uPlot from "uplot";
import { num, invoke } from "./api";
import { openDialog, act, selWires, toast, results } from "./store";
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
export const loopAnalysis = (first = "bode") => openDialog(() => {
  const [at, setAt] = createSignal("initial"), [t, setT] = createSignal(1);
  const [inp, setInp] = createSignal(0), [outp, setOutp] = createSignal(0);
  const [tab, setTab] = createSignal(first), [busy, setBusy] = createSignal(false);
  const [r, setR] = createSignal<any>(null);
  const go = async () => {
    setBusy(true);
    try { setR(await invoke<any>("lin_analysis", { at: at() === "time" ? { kind: "time", t: t() } : { kind: "initial" }, inp: inp(), outp: outp() })); }
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
      f.push(["Closed loop", x.nyquist.stable ? "stable" : `unstable: ${x.nyquist.z} pole(s) in the right half-plane`]);
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
      <div class="larow"><select class="laat" value={at()} onChange={e => { setAt(e.currentTarget.value); go(); }}><option value="initial">Initial state</option><option value="time" disabled={!results()}>Time in the last run</option></select>
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

/** The analysis point glyph on a wire: arrow in, a measuring circle, or a cut. */
export const PointGlyph = (p: { kind: string; at: [number, number] }): JSX.Element => {
  const [x, y] = p.at;
  return <g class={`apt apt-${p.kind}`} transform={`translate(${x} ${y})`}>
    <title>{p.kind === "in" ? "Analysis input: a test signal is added here" : p.kind === "out" ? "Analysis output: measured here" : "Loop opening: the loop is cut here for analysis"}</title>
    <circle r="7" />
    {p.kind === "in" ? <path d="M-3 -4L3 0L-3 4z" /> : p.kind === "out" ? <circle r="2.6" class="dot" /> : <path d="M-3.5 -3.5L3.5 3.5M3.5 -3.5L-3.5 3.5" />}
  </g>;
};
