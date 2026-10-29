// Scopes: the signals into a Scope block, plotted against time with uPlot in the family chart
// style (white plot, thin axes, Okabe-Ito colours, crosshair, PNG button). Live while running;
// after a run the engine sends each zoom level reduced with its peaks kept.
import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount, on } from "solid-js";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { invoke, num, Series } from "./api";
import { chans, results, live, liveTick, running, themeTick, dview, scopes, setScopes, call, toast, defOf, act, openMenu, openDialog, closeDialog, Item, setProbeT } from "./store";
import { C } from "./cmds";
import { Icon } from "./ui";

export const OKABE = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000"];
const css = (v: string) => getComputedStyle(document.documentElement).getPropertyValue(v).trim();
const blockName = (id: number) => dview()?.blocks.find(b => b.id === id)?.name ?? `Scope ${id}`;

/** Every scope-like sink in the last run: [block id, kind, name]. */
export const sinks = createMemo(() => {
  const seen = new Map<number, string>();
  for (const c of chans()) if (!c.probe && !seen.has(c.sink)) seen.set(c.sink, c.name);
  return [...seen.keys()].map(id => {
    const b = dview()?.blocks.find(x => x.id === id);
    return { id, kind: b?.kind ?? "Scope", name: b?.name ?? `Sink ${id}` };
  });
});

const stride = (n: number, max: number) => Math.max(1, Math.ceil(n / max));

/** The settings a Scope block keeps in its parameters (so they are saved and undoable). */
const scopeSet = (id: number) => {
  const b = dview()?.blocks.find(x => x.id === id);
  const num0 = (v: string | undefined) => (v == null || v.trim() === "" || v.trim() === "auto" ? null : +v);
  const lo = num0(b?.params.ymin), hi = num0(b?.params.ymax);
  return { stacked: b?.params.layout === "stacked", y: lo != null && hi != null && isFinite(lo) && isFinite(hi) && hi > lo ? [lo, hi] as [number, number] : null, here: !!b };
};

interface Marks { y0: number; yf: number; t10: number; t90: number; rise: number; peak: number; tpeak: number; overshoot: number; settle: number }

/** Nice tick values between a and b (for the SVG picture). */
const ticks = (a: number, b: number, n = 6) => {
  const span = b - a || 1, step0 = span / n, mag = Math.pow(10, Math.floor(Math.log10(step0))), f = step0 / mag;
  const step = (f < 1.5 ? 1 : f < 3 ? 2 : f < 7 ? 5 : 10) * mag, out: number[] = [];
  for (let v = Math.ceil(a / step) * step; v <= b + step * 1e-9; v += step) out.push(+v.toPrecision(12));
  return out;
};

export const Scope = (p: { id: number; close?: boolean; height?: number }) => {
  let host!: HTMLDivElement;
  let plots: uPlot[] = [];
  const idx = createMemo(() => chans().map((c, i) => [c, i] as const).filter(([c]) => c.sink === p.id && !c.probe).map(([, i]) => i));
  const isXY = () => dview()?.blocks.find(b => b.id === p.id)?.kind === "XY";
  const set = () => scopeSet(p.id);
  const [range, setRange] = createSignal<[number, number] | null>(null);
  const [cursor, setCursor] = createSignal<string>("");
  // Two measuring cursors (times), step marks, and earlier runs to overlay.
  const [curs, setCurs] = createSignal<[number, number] | null>(null);
  const [curVals, setCurVals] = createSignal<{ a: number[]; b: number[] } | null>(null);
  const [marks, setMarks] = createSignal<Marks | null>(null);
  const [cmp, setCmp] = createSignal<{ id: number; label: string }[]>([]);
  const sweep = () => results()?.sweep ?? null;
  const plotH = (n: number) => Math.max(80, (host.clientHeight - 8) / n - (host.querySelector(".u-legend")?.clientHeight ?? 24));
  /** The current data table: [t, current series…, compared series…] and which columns each plot shows. */
  let table: { data: uPlot.AlignedData; labels: string[]; dashed: boolean[]; groups: number[][]; ncur: number } | null = null;
  /** A signal keeps its colour in every layout; an earlier run takes its signal's colour, dashed. */
  const colour = (c: number) => OKABE[(c % Math.max(1, table?.ncur ?? 1)) % OKABE.length];

  const cssv = (v: string, d: string) => css(v) || d;
  const drawOver = (u: uPlot) => {
    const ctx = u.ctx, { left, top, width, height } = u.bbox, pr = uPlot.pxRatio;
    const xs = (t: number) => u.valToPos(t, "x", true);
    ctx.save();
    // uPlot leaves its own text settings on the canvas.
    ctx.textAlign = "left"; ctx.textBaseline = "alphabetic";
    ctx.beginPath(); ctx.rect(left, top, width, height); ctx.clip();
    const m = marks();
    if (m && u === plots[0]) {
      const ys = (v: number) => u.valToPos(v, "y", true), band = 0.02 * Math.abs(m.yf - m.y0);
      ctx.setLineDash([4 * pr, 3 * pr]); ctx.lineWidth = pr; ctx.strokeStyle = cssv("--muted", "#5B6878");
      for (const v of [m.yf - band, m.yf + band]) { ctx.beginPath(); ctx.moveTo(left, ys(v)); ctx.lineTo(left + width, ys(v)); ctx.stroke(); }
      ctx.strokeStyle = cssv("--blue", "#C2410C");
      for (const t of [m.t10, m.t90, m.settle]) if (isFinite(t)) { ctx.beginPath(); ctx.moveTo(xs(t), top); ctx.lineTo(xs(t), top + height); ctx.stroke(); }
      ctx.setLineDash([]); ctx.fillStyle = cssv("--blue", "#C2410C");
      ctx.beginPath(); ctx.arc(xs(m.tpeak), ys(m.peak), 3.5 * pr, 0, 7); ctx.fill();
      // The figures in one box at the top right, so they never sit on each other.
      const lines = [`Rise (10–90 %) ${num(m.rise)} s`, `Overshoot ${num(m.overshoot)} %`, `Peak ${num(m.peak)} at ${num(m.tpeak)} s`, `Settling (2 %) ${num(m.settle)} s`];
      ctx.font = `${11 * pr}px 'IBM Plex Sans', sans-serif`;
      const lw = Math.max(...lines.map(l => ctx.measureText(l).width)) + 12 * pr, lh = 14 * pr, bx = left + width - lw - 6 * pr, by = top + 6 * pr;
      ctx.fillStyle = cssv("--bg2", "#FFFFFF"); ctx.globalAlpha = 0.9; ctx.fillRect(bx, by, lw, lh * lines.length + 8 * pr); ctx.globalAlpha = 1;
      ctx.strokeStyle = cssv("--line", "#D3D7D6"); ctx.lineWidth = pr; ctx.strokeRect(bx, by, lw, lh * lines.length + 8 * pr);
      ctx.fillStyle = cssv("--text2", "#3B4859");
      lines.forEach((l, k) => ctx.fillText(l, bx + 6 * pr, by + 4 * pr + lh * (k + 0.8)));
    }
    const c = curs();
    if (c) {
      ctx.lineWidth = 1.2 * pr; ctx.font = `bold ${11 * pr}px 'IBM Plex Sans', sans-serif`;
      c.forEach((t, k) => {
        ctx.strokeStyle = ctx.fillStyle = k ? "#009E73" : "#D55E00";
        ctx.beginPath(); ctx.moveTo(xs(t), top); ctx.lineTo(xs(t), top + height); ctx.stroke();
        ctx.fillText(k ? "B" : "A", xs(t) + 3 * pr, top + height - 4 * pr);
      });
    }
    ctx.restore();
  };

  /** Drag a cursor line: grab within 6 px of it. */
  const hookCursors = (u: uPlot) => {
    let grab = -1;
    u.over.addEventListener("mousedown", e => {
      const c = curs(); if (!c) return;
      const x = e.offsetX, near = c.map(t => Math.abs(u.valToPos(t, "x") - x));
      const k = near[0] <= near[1] ? 0 : 1;
      if (near[k] > 6) return;
      grab = k; e.stopPropagation(); e.preventDefault();
      const mv = (m: MouseEvent) => {
        const r = u.over.getBoundingClientRect(), t = u.posToVal(Math.min(r.width, Math.max(0, m.clientX - r.left)), "x");
        const n = [...curs()!] as [number, number]; n[grab] = t; setCurs(n);
      };
      const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); grab = -1; };
      addEventListener("mousemove", mv); addEventListener("mouseup", up);
    }, true);
  };

  const make = () => {
    plots.forEach(u => u.destroy());
    plots = [];
    host.querySelectorAll(".uplot").forEach(e => e.remove());
    if (!table) return;
    const ink = css("--text2") || "#3B4859", grid = css("--line2") || "#E6E8E5";
    const axis = { stroke: ink, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid, width: 1 }, font: "11px 'IBM Plex Sans', sans-serif" };
    const yr = set().y;
    const n = table.groups.length, stacked = n > 1;
    table.groups.forEach((cols, g) => {
      // Stacked: no legend under each plot (it would take their height); the y axis names the signal.
      const yname = stacked ? [...new Set(cols.filter(c => !table!.dashed[c]).map(c => table!.labels[c]))].join(", ") : undefined;
      const opts: uPlot.Options = {
        width: Math.max(200, host.clientWidth), height: stacked ? Math.max(70, (host.clientHeight - 30) / n) : plotH(1),
        scales: yr ? { x: { time: false }, y: { auto: false, range: () => yr } } : { x: { time: false } },
        axes: [{ ...axis, label: g === n - 1 ? "Time (s)" : undefined, labelFont: "11px 'IBM Plex Sans', sans-serif", labelSize: g === n - 1 ? 18 : 0, ...(g === n - 1 ? {} : { size: 10, values: (_: uPlot, sp: number[]) => sp.map(() => "") }) },
          { ...axis, size: 56, label: yname, labelFont: "10px 'IBM Plex Sans', sans-serif", labelSize: yname ? 14 : 0 }],
        series: [{ label: "t", value: (_, v) => (v == null ? "" : num(v)) }, ...cols.map(c => ({
          label: table!.labels[c], stroke: colour(c),
          width: 1.6, dash: table!.dashed[c] ? [5, 4] : undefined, spanGaps: true, value: (_: unknown, v: number | null) => (v == null ? "" : num(v)),
        }))],
        cursor: { drag: { x: true, y: true, uni: 40, setScale: true }, points: { size: 5 }, sync: { key: `scope-${p.id}` } },
        legend: { live: true, show: !stacked },
        hooks: {
          draw: [drawOver],
          setScale: [(uu: uPlot, key: string) => {
            if (key !== "x" || running()) return;
            const mn = uu.scales.x.min, mx = uu.scales.x.max;
            if (mn != null && mx != null) { const r = results(); if (r && (Math.abs(mn - r.t0) > 1e-12 || Math.abs(mx - r.t1) > 1e-12)) setRange([mn, mx]); }
          }],
          setCursor: [(uu: uPlot) => { const i = uu.cursor.idx; const t = i == null ? null : (uu.data[0][i] as number); setCursor(t == null ? "" : `t = ${num(t)}`); if (t != null && !curs()) setProbeT(t); }],
        },
      };
      const data: uPlot.AlignedData = [table!.data[0], ...cols.map(c => table!.data[c + 1])];
      const u = new uPlot(opts, data, host);
      hookCursors(u);
      plots.push(u);
    });
    // Stacked plots share one time range.
    if (plots.length > 1) plots.forEach(u => (u.hooks.setScale ??= []).push((uu: uPlot, key: string) => {
      if (key !== "x") return;
      plots.forEach(o => { if (o !== uu && (o.scales.x.min !== uu.scales.x.min || o.scales.x.max !== uu.scales.x.max)) o.setScale("x", { min: uu.scales.x.min!, max: uu.scales.x.max! }); });
    }));
    host.ondblclick = () => { setRange(null); make(); };
  };

  /** Group the current columns per plot: one plot, or one per scope input when stacked. */
  const groupsFor = (ix: number[], extra: number) => {
    const ports = ix.map(i => chans()[i].port);
    const stacked = set().stacked && !running();
    const all = [...ix.keys()];
    const per = (k: number) => [...all.filter(c => ports[c] === k), ...Array.from({ length: extra }, (_, r) => all.filter(c => ports[c] === k).map(c => ix.length * (r + 1) + c)).flat()];
    if (!stacked) return [[...all, ...Array.from({ length: extra * ix.length }, (_, k) => ix.length + k)]];
    return [...new Set(ports)].map(per);
  };

  /** Live samples (or the full result, with any compared runs) for this scope. */
  const draw = async () => {
    const ix = idx(); if (!host || !ix.length || isXY()) return;
    const labels = ix.map(i => chans()[i].name);
    const sw = sweep();
    if (!running() && sw) {
      const runs = await Promise.all(ix.map(i => invoke<Series[]>("sweep_series", { chan: i, buckets: Math.max(200, host.clientWidth) })));
      const tables: uPlot.AlignedData[] = runs.flatMap(rs => rs.map(r => [r.t, r.y] as uPlot.AlignedData));
      const joined = uPlot.join(tables);
      const names = ix.flatMap(i => sw[1].map(v => `${chans()[i].name} · ${sw[0]} = ${num(v)}`));
      table = { data: joined, labels: names, dashed: names.map(() => false), groups: [names.map((_, k) => k)], ncur: names.length };
      make();
      return;
    }
    if (!running() && results()) {
      const r = results()!, [t0, t1] = range() ?? [r.t0, r.t1], buckets = Math.max(200, host.clientWidth);
      const s = await invoke<{ t: number[]; y: number[][] }>("series", { chans: ix, t0, t1, buckets });
      const others = await Promise.all(cmp().map(c => invoke<{ t: number[]; y: number[][] }>("run_series", { run: c.id, names: labels, t0, t1, buckets })));
      const data = others.length ? uPlot.join([[s.t, ...s.y] as uPlot.AlignedData, ...others.map(o => [o.t, ...o.y] as uPlot.AlignedData)]) : [s.t, ...s.y] as uPlot.AlignedData;
      const all = [...labels, ...cmp().flatMap(c => labels.map(l => `${l} · ${c.label}`))];
      const same = table && plots.length && table.labels.join("|") === all.join("|") && table.groups.length === groupsFor(ix, others.length).length;
      table = { data, labels: all, dashed: all.map((_, k) => k >= labels.length), groups: groupsFor(ix, others.length), ncur: labels.length };
      if (!same) make();
      else table.groups.forEach((cols, g) => plots[g]?.setData([data[0], ...cols.map(c => data[c + 1])], !range()));
      if (range()) plots.forEach(u => u.setScale("x", { min: t0, max: t1 }));
      return;
    }
    // Live: every k-th sample so a long run stays smooth.
    const n = live.t.length, k = stride(n, 6000);
    const t: number[] = [], ys: number[][] = ix.map(() => []);
    for (let j = 0; j < n; j += k) { t.push(live.t[j]); ix.forEach((i, m) => ys[m].push(live.y[i]?.[j] ?? NaN)); }
    const data = [t, ...ys] as uPlot.AlignedData;
    if (!table || plots.length !== 1 || table.labels.join("|") !== labels.join("|")) { table = { data, labels, dashed: labels.map(() => false), groups: [labels.map((_, k) => k)], ncur: labels.length }; make(); }
    else { table.data = data; plots[0].setData(data); }
  };

  createEffect(on([liveTick, idx, range, () => results()?.sweep, cmp, () => set().stacked, () => JSON.stringify(set().y)], () => { if (!isXY()) draw(); }));
  // A new run clears the marks; cursors and compared runs stay.
  createEffect(on(() => results(), () => { setMarks(null); setCmp(c => c.filter(x => x.id !== -1)); }, { defer: true }));
  // Cursor readouts come from the engine (interpolated at the exact times).
  // Redraw the overlays on the next frame: a plot made in this tick has not measured its axes yet.
  const redraw = () => requestAnimationFrame(() => plots.forEach(u => u.redraw(false)));
  createEffect(on([curs, () => results()], async () => {
    const c = curs(); if (!c || !results()) { setCurVals(null); redraw(); return; }
    const [a, b] = await Promise.all(c.map(t => invoke<number[]>("value_at", { chans: idx(), t })));
    setCurVals({ a, b });
    setProbeT(c[0]);
    redraw();
  }));
  createEffect(on(marks, redraw, { defer: true }));
  // A new theme needs new axis and grid colours: build the plot again.
  createEffect(on(themeTick, () => { if (!isXY()) make(); }, { defer: true }));
  onMount(() => {
    const ro = new ResizeObserver(() => plots.forEach(u => u.setSize({ width: Math.max(200, host.clientWidth), height: plots.length > 1 ? Math.max(70, (host.clientHeight - 30) / plots.length) : plotH(1) })));
    ro.observe(host);
    onCleanup(() => { ro.disconnect(); plots.forEach(u => u.destroy()); });
  });

  const toggleCursors = () => {
    if (curs()) return setCurs(null);
    const u = plots[0]; const mn = u?.scales.x.min, mx = u?.scales.x.max;
    if (mn == null || mx == null) return toast("Run the model first.");
    setCurs([mn + 0.25 * (mx - mn), mn + 0.75 * (mx - mn)]);
  };
  const toggleMarks = async () => {
    if (marks()) return setMarks(null);
    const i = idx()[0]; if (i == null || !results()) return toast("Run the model first.");
    try { setMarks(await invoke<Marks>("step_marks", { chan: i })); } catch (e) { toast(String(e)); }
  };
  const compareMenu = async (e: MouseEvent) => {
    const all = await invoke<{ id: number; label: string }[]>("runs").catch(() => []);
    const earlier = all.slice(0, -1).reverse();
    if (!earlier.length) return toast("Run the model again to compare runs: the last five are kept.");
    openMenu(e, earlier.map(r => ({ label: `${cmp().some(c => c.id === r.id) ? "✓ " : ""}${r.label}`, run: () => setCmp(c => c.some(x => x.id === r.id) ? c.filter(x => x.id !== r.id) : [...c, r]) })));
  };
  const setParam = (key: string, value: string) => act({ SetParam: { id: p.id, key, value } });
  const moreMenu = (e: MouseEvent) => {
    const s = set();
    if (isXY()) return openMenu(e, [{ label: "Save as PNG", run: () => png(1) }, { label: "Save signals as CSV", run: () => C.exportCsv(p.id) }]);
    openMenu(e, [
      { label: "Save as PNG, as shown", run: () => png(1) },
      { label: "Save as PNG, high resolution (3×)", run: () => png(3) },
      { label: "Save as SVG (vector, for papers)", run: svg },
      { label: "Save signals as CSV", run: () => C.exportCsv(p.id) },
      "-" as unknown as Item,
      { label: `${!s.stacked ? "✓ " : ""}All inputs on one plot`, run: () => setParam("layout", "overlaid") },
      { label: `${s.stacked ? "✓ " : ""}One plot per input (stacked)`, run: () => setParam("layout", "stacked") },
      "-" as unknown as Item,
      { label: `${!s.y ? "✓ " : ""}Auto-scale the y axis`, run: () => act({ Batch: [{ SetParam: { id: p.id, key: "ymin", value: "auto" } }, { SetParam: { id: p.id, key: "ymax", value: "auto" } }] }) },
      { label: `${s.y ? "✓ " : ""}Fixed y range…`, run: () => yRangeDialog(p.id, s.y ?? (plots[0] ? [plots[0].scales.y.min ?? 0, plots[0].scales.y.max ?? 1] : [0, 1])) },
    ].filter((_, k) => s.here || k < 4));
  };

  /** The plots drawn again off screen at a size and scale, for a picture. */
  const render = async (w: number, h: number, k: number): Promise<HTMLCanvasElement | null> => {
    if (!table) return null;
    const off = document.createElement("div");
    off.style.cssText = `position:fixed;left:-99999px;top:0;width:${w * k}px;`;
    document.body.appendChild(off);
    const n = table.groups.length, ph = (h * k) / n;
    const made: uPlot[] = [];
    const yr = set().y;
    table.groups.forEach((cols, g) => {
      const axis = { stroke: "#3B4859", grid: { stroke: "#E6E8E5", width: k }, ticks: { stroke: "#E6E8E5", width: k }, font: `${11 * k}px 'IBM Plex Sans', sans-serif`, size: 56 * k, gap: 5 * k };
      const range0 = plots[g]?.scales.x;
      const u = new uPlot({
        width: w * k, height: ph, legend: { show: false }, cursor: { show: false },
        scales: { x: { time: false, ...(range0?.min != null ? { min: range0.min, max: range0.max } : {}) }, ...(yr ? { y: { auto: false, range: () => yr } } : {}) },
        axes: [{ ...axis, size: 40 * k, label: g === n - 1 ? "Time (s)" : undefined, labelFont: `${11 * k}px 'IBM Plex Sans', sans-serif`, labelSize: 18 * k }, axis],
        series: [{}, ...cols.map(c => ({ stroke: colour(c), width: 1.6 * k, dash: table!.dashed[c] ? [5 * k, 4 * k] : undefined, spanGaps: true }))],
      }, [table!.data[0], ...cols.map(c => table!.data[c + 1])], off);
      made.push(u);
    });
    // uPlot draws on the next frame.
    await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
    const canv = made.map(u => u.ctx.canvas);
    const out = document.createElement("canvas");
    out.width = canv[0]?.width ?? w * k; out.height = canv.reduce((a, c) => a + c.height, 0);
    const g2 = out.getContext("2d")!; g2.fillStyle = "#fff"; g2.fillRect(0, 0, out.width, out.height);
    let y = 0; canv.forEach(c => { g2.drawImage(c, 0, y); y += c.height; });
    made.forEach(u => u.destroy());
    off.remove();
    return out;
  };
  const png = async (k = 1) => {
    const w = host.clientWidth, h = host.clientHeight;
    const cv = k === 1 ? null : await render(w, h, k);
    let data: string;
    if (cv) data = cv.toDataURL("image/png");
    else {
      const cs = [...host.querySelectorAll("canvas")] as HTMLCanvasElement[]; if (!cs.length) return toast("Run the model first.");
      const c2 = document.createElement("canvas"); c2.width = cs[0].width; c2.height = cs.reduce((a, c) => a + c.height, 0);
      const g = c2.getContext("2d")!; g.fillStyle = "#fff"; g.fillRect(0, 0, c2.width, c2.height);
      let y = 0; cs.forEach(c => { g.drawImage(c, 0, y); y += c.height; });
      data = c2.toDataURL("image/png");
    }
    const path = await call<string | null>("save_file", { name: `${blockName(p.id)}${k > 1 ? ` ${k}x` : ""}.png`, data });
    if (path) toast(`Saved ${path}`, true);
  };
  /** A vector picture: axes, ticks, grid and the lines, for papers and reports. */
  const svg = async () => {
    if (!table) return toast("Run the model first.");
    const W = 720, n = table.groups.length, PH = 260, M = { l: 64, r: 16, t: 14, b: 40 }, H = n * PH;
    const t = table.data[0] as number[];
    const x0 = plots[0]?.scales.x.min ?? t[0], x1 = plots[0]?.scales.x.max ?? t[t.length - 1];
    const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
    let body = "";
    table.groups.forEach((cols, g) => {
      const top = g * PH, ph = PH - M.t - M.b, pw = W - M.l - M.r;
      const vals = cols.flatMap(c => (table!.data[c + 1] as (number | null)[]).filter((v, i): v is number => v != null && isFinite(v) && t[i] >= x0 && t[i] <= x1));
      let [y0, y1] = set().y ?? [Math.min(...vals), Math.max(...vals)];
      if (!(y1 > y0)) { y0 -= 1; y1 += 1; }
      const sx = (v: number) => M.l + (v - x0) / (x1 - x0) * pw, sy = (v: number) => top + M.t + ph - (v - y0) / (y1 - y0) * ph;
      for (const v of ticks(y0, y1)) body += `<line x1="${M.l}" x2="${M.l + pw}" y1="${sy(v).toFixed(1)}" y2="${sy(v).toFixed(1)}" stroke="#E6E8E5"/><text x="${M.l - 6}" y="${sy(v).toFixed(1)}" text-anchor="end" dominant-baseline="central">${num(v)}</text>`;
      for (const v of ticks(x0, x1, 8)) body += `<line x1="${sx(v).toFixed(1)}" x2="${sx(v).toFixed(1)}" y1="${top + M.t}" y2="${top + M.t + ph}" stroke="#E6E8E5"/><text x="${sx(v).toFixed(1)}" y="${top + M.t + ph + 14}" text-anchor="middle">${num(v)}</text>`;
      body += `<rect x="${M.l}" y="${top + M.t}" width="${pw}" height="${ph}" fill="none" stroke="#B9BFC2"/>`;
      cols.forEach((c, j) => {
        const ys = table!.data[c + 1] as (number | null)[];
        let d = "", pen = false;
        ys.forEach((v, i) => { if (v == null || !isFinite(v) || t[i] < x0 || t[i] > x1) { pen = false; return; } d += `${pen ? "L" : "M"}${sx(t[i]).toFixed(2)} ${sy(v).toFixed(2)}`; pen = true; });
        body += `<path d="${d}" fill="none" stroke="${colour(c)}" stroke-width="1.6"${table!.dashed[c] ? ` stroke-dasharray="5 4"` : ""}/>`;
        body += `<text x="${M.l + 8}" y="${top + M.t + 14 + 14 * j}" fill="${colour(c)}">${esc(table!.labels[c])}</text>`;
      });
      if (g === n - 1) body += `<text x="${M.l + pw / 2}" y="${top + PH - 8}" text-anchor="middle">Time (s)</text>`;
    });
    const doc = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="IBM Plex Sans, Arial, sans-serif" font-size="11" fill="#3B4859"><rect width="${W}" height="${H}" fill="#fff"/>${body}</svg>`;
    const path = await call<string | null>("save_file", { name: `${blockName(p.id)}.svg`, data: doc });
    if (path) toast(`Saved ${path}`, true);
  };

  return (
    <div class="scope" data-scope={p.id} style={p.height ? { height: `${p.height}px` } : undefined}>
      <header>
        <Icon n="scope" /><b class="stitle" title={blockName(p.id)}>{blockName(p.id)}</b><span class="muted stime">{cursor()}</span><span class="grow" />
        <Show when={range()}><button class="btn sm" onClick={() => setRange(null)}>Whole run</button></Show>
        <Show when={!isXY()}>
          <button class="btn sm cursbtn" classList={{ on: !!curs() }} title="Two measuring cursors: drag the A and B lines" onClick={toggleCursors} disabled={!results() || running()}>Cursors</button>
          <button class="btn sm marksbtn" classList={{ on: !!marks() }} title="Mark rise time, overshoot and settling time of the first signal" onClick={toggleMarks} disabled={!results() || running()}>Marks</button>
          <button class="btn sm cmpbtn" classList={{ on: cmp().length > 0 }} title="Overlay earlier runs (the last five are kept)" onClick={compareMenu} disabled={!results() || running()}>Compare ▾</button>
        </Show>
        <button class="btn sm morebtn" title="Pictures (PNG, SVG), CSV, layout and y range" onClick={moreMenu}>More ▾</button>
        <Show when={p.close}><button class="x" title="Close" onClick={() => setScopes(scopes().filter(x => x !== p.id))}>×</button></Show>
      </header>
      <Show when={curs() && curVals()}>
        <div class="cursread"><table><thead><tr><th /><th>A</th><th>B</th><th>B − A</th></tr></thead><tbody>
          <tr><td>t</td><td>{num(curs()![0])}</td><td>{num(curs()![1])}</td><td>{num(curs()![1] - curs()![0])}{curs()![1] !== curs()![0] ? ` (1/Δt ${num(1 / Math.abs(curs()![1] - curs()![0]))} Hz)` : ""}</td></tr>
          <For each={idx()}>{(i, k) => <tr><td style={{ color: OKABE[k() % OKABE.length] }}>{chans()[i].name}</td><td>{num(curVals()!.a[k()])}</td><td>{num(curVals()!.b[k()])}</td><td>{num(curVals()!.b[k()] - curVals()!.a[k()])}</td></tr>}</For>
        </tbody></table></div>
      </Show>
      <Show when={isXY()} fallback={<div class="plot" ref={host}><Show when={!idx().length}><p class="muted pempty">Run the model to see this scope.</p></Show></div>}>
        <XYPlot id={p.id} />
      </Show>
    </div>
  );
};

/** Fixed y range for a scope: saved in the Scope block, so it is undoable and kept in the file. */
const yRangeDialog = (id: number, cur: [number, number]) => openDialog(() => {
  const [lo, setLo] = createSignal(num(cur[0])), [hi, setHi] = createSignal(num(cur[1]));
  const ok = () => { const a = +lo(), b = +hi(); if (!(isFinite(a) && isFinite(b) && b > a)) return toast("The upper limit must be above the lower one."); closeDialog(); act({ Batch: [{ SetParam: { id, key: "ymin", value: String(a) } }, { SetParam: { id, key: "ymax", value: String(b) } }] }); };
  return <div class="ov" onMouseDown={e => e.target === e.currentTarget && closeDialog()}><div class="dlg sm"><h2>Y range</h2>
    <div class="iprow"><label>Lower</label><input class="mono" value={lo()} onInput={e => setLo(e.currentTarget.value)} /></div>
    <div class="iprow"><label>Upper</label><input class="mono" value={hi()} onInput={e => setHi(e.currentTarget.value)} onKeyDown={e => e.key === "Enter" && ok()} /></div>
    <p class="btns"><button class="btn" onClick={closeDialog}>Cancel</button><button class="btn pri" onClick={ok}>Set</button></p></div></div>;
});

/** XY graph: input 2 against input 1. */
const XYPlot = (p: { id: number }) => {
  const [pts, setPts] = createSignal<[number[], number[]]>([[], []]);
  const ix = createMemo(() => chans().map((c, i) => [c, i] as const).filter(([c]) => c.sink === p.id).map(([, i]) => i));
  createEffect(on([liveTick, ix], async () => {
    const i = ix(); if (i.length < 2) return;
    if (!running() && results()) { const r = results()!; const s = await invoke<{ t: number[]; y: number[][] }>("series", { chans: i.slice(0, 2), t0: r.t0, t1: r.t1, buckets: 2000 }); setPts([s.y[0], s.y[1]]); }
    else { const n = live.t.length, k = stride(n, 4000), x: number[] = [], y: number[] = []; for (let j = 0; j < n; j += k) { x.push(live.y[i[0]]?.[j]); y.push(live.y[i[1]]?.[j]); } setPts([x, y]); }
  }));
  const box = createMemo(() => {
    const [x, y] = pts(); if (!x.length) return null;
    const fx = x.filter(isFinite), fy = y.filter(isFinite);
    const x0 = Math.min(...fx), x1 = Math.max(...fx), y0 = Math.min(...fy), y1 = Math.max(...fy);
    return { x0, x1: x1 > x0 ? x1 : x0 + 1, y0, y1: y1 > y0 ? y1 : y0 + 1 };
  });
  const W = 400, H = 260, M = 36;
  return <div class="plot xy"><Show when={box()} fallback={<p class="muted pempty">Run the model to see this graph.</p>}>{b => {
    const sx = (v: number) => M + (v - b().x0) / (b().x1 - b().x0) * (W - M - 10), sy = (v: number) => H - M + 10 - (v - b().y0) / (b().y1 - b().y0) * (H - M);
    return <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMidYMid meet">
      <rect x={M} y={10} width={W - M - 10} height={H - M} class="xyframe" />
      <path d={pts()[0].map((x, i) => `${i ? "L" : "M"}${sx(x).toFixed(1)} ${sy(pts()[1][i]).toFixed(1)}`).join("")} fill="none" stroke={OKABE[0]} stroke-width="1.6" />
      <text x={M} y={H - 6} class="xyt">{num(b().x0)}</text><text x={W - 10} y={H - 6} class="xyt" text-anchor="end">{num(b().x1)}</text>
      <text x={M - 4} y={H - M + 10} class="xyt" text-anchor="end">{num(b().y0)}</text><text x={M - 4} y={16} class="xyt" text-anchor="end">{num(b().y1)}</text>
    </svg>;
  }}</Show></div>;
};

/** The dock under the diagram: the open scopes side by side. */
export const Dock = () => (
  <div class="dock">
    <For each={scopes()} fallback={<p class="muted pempty">Double-click a Scope block to open it here. Scopes open by themselves on the first run.</p>}>{id => <Scope id={id} close />}</For>
  </div>
);

/** View › Scopes: every scope of the last run, large. */
export const ScopesPage = () => (
  <div class="page scopespage">
    <h1>Scopes</h1>
    <Show when={sinks().length} fallback={<p class="muted">Run the model (Simulate › Run, or F5) to see its scopes. Add Scope blocks to plot signals.</p>}>
      <div class="scopegrid"><For each={sinks().filter(s => s.kind === "Scope" || s.kind === "XY")}>{s => <Scope id={s.id} height={320} />}</For></div>
    </Show>
  </div>
);

/** View › Results: the latest value of every logged signal, and the run's figures. */
export const ResultsPage = () => {
  const r = results;
  return (
    <div class="page">
      <h1>Results</h1>
      <Show when={r()} fallback={<p class="muted">Run the model to see its results. Every signal going into a Scope, Display, To file or To Datum Plot block is logged.</p>}>
        <p class="muted">{r()!.n.toLocaleString()} time points from {num(r()!.t0)} s to {num(r()!.t1)} s · {r()!.stats.steps.toLocaleString()} steps ({r()!.stats.rejected.toLocaleString()} rejected) · {r()!.stats.fevals.toLocaleString()} evaluations{r()!.stats.events ? ` · ${r()!.stats.events} switching events located` : ""} · {Math.round(r()!.stats.ms)} ms</p>
        <p><button class="btn" onClick={() => C.exportCsv()}><Icon n="csv" /> Save all as CSV</button> <button class="btn" onClick={C.toPlot}><Icon n="plot" /> Send to Datum Plot</button> <button class="btn" onClick={C.stepInfo}><Icon n="specs" /> Step-response specifications</button></p>
        <table class="tbl"><thead><tr><th>Signal</th><th>Logged by</th><th class="r">Final value</th><th class="r">Minimum</th><th class="r">Maximum</th></tr></thead>
          <tbody><For each={r()!.chans}>{(c, i) => <Show when={!c.probe}><ResultRow name={c.name} sink={c.sink} k={i()} last={r()!.last[i()]} /></Show>}</For></tbody></table>
      </Show>
    </div>
  );
};

const ResultRow = (p: { name: string; sink: number; k: number; last: number }) => {
  const [mm, setMm] = createSignal<[number, number] | null>(null);
  onMount(async () => {
    const r = results(); if (!r) return;
    const s = await invoke<{ t: number[]; y: number[][] }>("series", { chans: [p.k], t0: r.t0, t1: r.t1, buckets: 2000 });
    const y = s.y[0] ?? []; if (y.length) setMm([Math.min(...y), Math.max(...y)]);
  });
  const sinkName = () => { const b = dview()?.blocks.find(b => b.id === p.sink); return b ? `${b.name} (${defOf(b.kind)?.name ?? b.kind})` : "inside a subsystem"; };
  return <tr><td>{p.name}</td><td class="muted">{sinkName()}</td><td class="r">{num(p.last)}</td><td class="r">{mm() ? num(mm()![0]) : ""}</td><td class="r">{mm() ? num(mm()![1]) : ""}</td></tr>;
};
