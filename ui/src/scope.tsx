// Scopes: the signals into a Scope block, plotted against time with uPlot in the family chart
// style (white plot, thin axes, Okabe-Ito colours, crosshair, PNG button). Live while running;
// after a run the engine sends each zoom level reduced with its peaks kept.
import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount, on } from "solid-js";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { invoke, num, Series } from "./api";
import { chans, results, live, liveTick, running, themeTick, dview, scopes, setScopes, call, toast, defOf } from "./store";
import { C } from "./cmds";
import { Icon } from "./ui";

export const OKABE = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000"];
const css = (v: string) => getComputedStyle(document.documentElement).getPropertyValue(v).trim();
const blockName = (id: number) => dview()?.blocks.find(b => b.id === id)?.name ?? `Scope ${id}`;

/** Every scope-like sink in the last run: [block id, kind, name]. */
export const sinks = createMemo(() => {
  const seen = new Map<number, string>();
  for (const c of chans()) if (!seen.has(c.sink)) seen.set(c.sink, c.name);
  return [...seen.keys()].map(id => {
    const b = dview()?.blocks.find(x => x.id === id);
    return { id, kind: b?.kind ?? "Scope", name: b?.name ?? `Sink ${id}` };
  });
});

const stride = (n: number, max: number) => Math.max(1, Math.ceil(n / max));

export const Scope = (p: { id: number; close?: boolean; height?: number }) => {
  let host!: HTMLDivElement;
  let u: uPlot | null = null;
  const idx = createMemo(() => chans().map((c, i) => [c, i] as const).filter(([c]) => c.sink === p.id).map(([, i]) => i));
  const isXY = () => dview()?.blocks.find(b => b.id === p.id)?.kind === "XY";
  const [range, setRange] = createSignal<[number, number] | null>(null);
  const [cursor, setCursor] = createSignal<string>("");
  const sweep = () => results()?.sweep ?? null;
  /** The plot's height, leaving room for the legend under it. */
  const plotH = () => Math.max(100, host.clientHeight - 8 - (host.querySelector(".u-legend")?.clientHeight ?? 24));

  const make = (data: uPlot.AlignedData, labels: string[]) => {
    u?.destroy();
    const ink = css("--text2") || "#3B4859", grid = css("--line2") || "#E6E8E5";
    const axis = { stroke: ink, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid, width: 1 }, font: "11px 'IBM Plex Sans', sans-serif" };
    const opts: uPlot.Options = {
      width: Math.max(200, host.clientWidth), height: plotH(),
      scales: { x: { time: false } },
      axes: [{ ...axis, label: "Time (s)", labelFont: "11px 'IBM Plex Sans', sans-serif", labelSize: 18 }, { ...axis, size: 56 }],
      series: [{ label: "t", value: (_, v) => (v == null ? "" : num(v)) }, ...labels.map((l, i) => ({ label: l, stroke: OKABE[i % OKABE.length], width: 1.6, spanGaps: true, value: (_: unknown, v: number | null) => (v == null ? "" : num(v)) }))],
      cursor: { drag: { x: true, y: false, setScale: true }, points: { size: 5 } },
      legend: { live: true },
      hooks: {
        setScale: [(uu: uPlot, key: string) => {
          if (key !== "x" || running()) return;
          const mn = uu.scales.x.min, mx = uu.scales.x.max;
          if (mn != null && mx != null) { const r = results(); if (r && (Math.abs(mn - r.t0) > 1e-12 || Math.abs(mx - r.t1) > 1e-12)) setRange([mn, mx]); }
        }],
        setCursor: [(uu: uPlot) => { const i = uu.cursor.idx; setCursor(i == null ? "" : `t = ${num(uu.data[0][i] as number)}`); }],
      },
    };
    u = new uPlot(opts, data, host);
    host.ondblclick = () => setRange(null);
  };

  /** Live samples (or the full result) for this scope. */
  const draw = async () => {
    const ix = idx(); if (!host || !ix.length || isXY()) return;
    const labels = ix.map(i => chans()[i].name);
    const sw = sweep();
    if (!running() && sw) {
      // One channel per scope input, every run overlaid.
      const runs = await Promise.all(ix.map(i => invoke<Series[]>("sweep_series", { chan: i, buckets: Math.max(200, host.clientWidth) })));
      const tables: uPlot.AlignedData[] = runs.flatMap(rs => rs.map(r => [r.t, r.y] as uPlot.AlignedData));
      const joined = uPlot.join(tables);
      const names = ix.flatMap(i => sw[1].map(v => `${chans()[i].name} · ${sw[0]} = ${num(v)}`));
      make(joined, names);
      return;
    }
    if (!running() && results()) {
      const r = results()!, [t0, t1] = range() ?? [r.t0, r.t1];
      const s = await invoke<{ t: number[]; y: number[][] }>("series", { chans: ix, t0, t1, buckets: Math.max(200, host.clientWidth) });
      if (!u || u.series.length !== labels.length + 1 || sw) make([s.t, ...s.y], labels);
      else u.setData([s.t, ...s.y], !range());
      if (range()) u!.setScale("x", { min: t0, max: t1 });
      return;
    }
    // Live: every k-th sample so a long run stays smooth.
    const n = live.t.length, k = stride(n, 6000);
    const t: number[] = [], ys: number[][] = ix.map(() => []);
    for (let j = 0; j < n; j += k) { t.push(live.t[j]); ix.forEach((i, m) => ys[m].push(live.y[i]?.[j] ?? NaN)); }
    if (!u || u.series.length !== labels.length + 1) make([t, ...ys], labels);
    else u.setData([t, ...ys]);
  };

  createEffect(on([liveTick, idx, range, themeTick, () => results()?.sweep], () => { if (!isXY()) draw(); }));
  onMount(() => {
    const ro = new ResizeObserver(() => { if (u) u.setSize({ width: Math.max(200, host.clientWidth), height: plotH() }); });
    ro.observe(host);
    onCleanup(() => { ro.disconnect(); u?.destroy(); });
  });

  const png = async () => {
    const cv = host.querySelector("canvas"); if (!cv) return toast("Run the model first.");
    const c2 = document.createElement("canvas"); c2.width = cv.width; c2.height = cv.height;
    const g = c2.getContext("2d")!; g.fillStyle = "#fff"; g.fillRect(0, 0, c2.width, c2.height); g.drawImage(cv, 0, 0);
    const path = await call<string | null>("save_file", { name: `${blockName(p.id)}.png`, data: c2.toDataURL("image/png") });
    if (path) toast(`Saved ${path}`, true);
  };

  return (
    <div class="scope" data-scope={p.id} style={p.height ? { height: `${p.height}px` } : undefined}>
      <header>
        <Icon n="scope" /><b>{blockName(p.id)}</b><span class="muted">{cursor()}</span><span class="grow" />
        <Show when={range()}><button class="btn sm" onClick={() => setRange(null)}>Whole run</button></Show>
        <button class="btn sm" title="Save a picture of this scope" onClick={png}>PNG</button>
        <button class="btn sm" title="Save this scope's signals as CSV" onClick={() => C.exportCsv(p.id)} disabled={!results()}>CSV</button>
        <Show when={p.close}><button class="x" title="Close" onClick={() => setScopes(scopes().filter(x => x !== p.id))}>×</button></Show>
      </header>
      <Show when={isXY()} fallback={<div class="plot" ref={host}><Show when={!idx().length}><p class="muted pempty">Run the model to see this scope.</p></Show></div>}>
        <XYPlot id={p.id} />
      </Show>
    </div>
  );
};

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
          <tbody><For each={r()!.chans}>{(c, i) => <ResultRow name={c.name} sink={c.sink} k={i()} last={r()!.last[i()]} />}</For></tbody></table>
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
