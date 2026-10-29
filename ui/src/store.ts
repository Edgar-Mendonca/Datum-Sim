// Window state: the open model, the diagram being edited, the selection, runs and results.
import { createSignal, batch, JSX } from "solid-js";
import { invoke, Out, DView, Library, Def, Options, Issue, ResultsInfo, Chan } from "./api";

export const [out, setOutRaw] = createSignal<Out | null>(null);
export const [tick, setTick] = createSignal(0);
export const setOut = (o: Out | null | undefined) => { if (o) batch(() => { setOutRaw(o); setTick(t => t + 1); }); };

/** auto, light or dark; auto follows Windows. */
export const [theme, setThemeRaw] = createSignal("auto");
const darkMq = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
export const applyTheme = () => { document.documentElement.dataset.theme = theme() === "auto" ? (darkMq?.matches ? "dark" : "light") : theme(); setThemeTick(t => t + 1); };
export const [themeTick, setThemeTick] = createSignal(0);
darkMq?.addEventListener("change", () => theme() === "auto" && applyTheme());
export const setTheme = (t: string) => { setThemeRaw(t); applyTheme(); const o = { ...opts(), theme: t }; setOpts(o); invoke("set_options", { opts: o }).catch(() => {}); };
export const [opts, setOpts] = createSignal<Options>({ autosave: 5, beta: false, theme: "auto", openLast: false, solver: "rk45", stop: 10, snap: true, grid: true, paper: "A4", landscape: true, company: "" });

export type View = "model" | "vars" | "scopes" | "results" | "analysis" | "issues";
export const [view, setView] = createSignal<View>("model");
export const [backstage, setBackstage] = createSignal<string | null>("home");
export const [tab, setTab] = createSignal("model");

// ---------- the block library ----------
export const [lib, setLib] = createSignal<Library>({ cats: [], blocks: [] });
export const defOf = (kind: string): Def | undefined => lib().blocks.find(d => d.kind === kind && !d.planned);

// ---------- the diagram ----------
/** The diagram being edited: 0 = top level, else a subsystem's block id. */
export const [parent, setParent] = createSignal(0);
export const [dview, setDview] = createSignal<DView | null>(null);
export const [sel, setSel] = createSignal<number[]>([]);
export const selBlocks = () => { const s = new Set(sel()); return dview()?.blocks.filter(b => s.has(b.id)) ?? []; };
export const selWires = () => { const s = new Set(sel()); return dview()?.wires.filter(w => s.has(w.id)) ?? []; };
// ---------- sample-time colours ----------
export interface RateView { id: number; rate: "constant" | "continuous" | "discrete" | "mixed"; ts: number }
export const [showRates, setShowRatesRaw] = createSignal(false);
export const [rates, setRates] = createSignal<RateView[]>([]);
export const refreshRates = async () => { if (showRates()) setRates(await invoke<RateView[]>("rates").catch(() => [] as RateView[])); };
export const setShowRates = (on: boolean) => { setShowRatesRaw(on); if (on) refreshRates(); else setRates([]); };
const RATE_COLOURS = ["#DC2626", "#16A34A", "#2563EB", "#D97706", "#0891B2", "#9333EA", "#DB2777"];
/** Colours: each discrete sample time (fastest first) gets its own; constants magenta, mixed amber. */
export const rateColours = () => {
  const ts = [...new Set(rates().filter(r => r.rate === "discrete").map(r => r.ts))].sort((a, b) => a - b);
  const col = (r: RateView) => r.rate === "discrete" ? RATE_COLOURS[ts.indexOf(r.ts) % RATE_COLOURS.length] : r.rate === "constant" ? "#C026D3" : r.rate === "mixed" ? "#CA8A04" : "var(--text)";
  const by = new Map(rates().map(r => [r.id, col(r)]));
  const legend: [string, string][] = [];
  if (rates().some(r => r.rate === "continuous")) legend.push(["var(--text)", "Continuous"]);
  ts.forEach((t, i) => legend.push([RATE_COLOURS[i % RATE_COLOURS.length], `Ts = ${+t.toPrecision(6)} s`]));
  if (rates().some(r => r.rate === "constant")) legend.push(["#C026D3", "Constant"]);
  if (rates().some(r => r.rate === "mixed")) legend.push(["#CA8A04", "Several rates"]);
  return { by, legend };
};

let pending = 0;
export const refreshView = async () => {
  if (!out()) return;
  const n = ++pending;
  try {
    const v = await invoke<DView>("diagram", { parent: parent() });
    if (n === pending) { setDview(v); refreshRates(); }
  } catch {
    // The subsystem was deleted or undone away: go back to the top.
    if (parent() !== 0) { setParent(0); refreshView(); }
  }
};
export const enter = (id: number) => batch(() => { setParent(id); setSel([]); refreshView(); });

// ---------- issues from the last check or run ----------
export const [issues, setIssues] = createSignal<Issue[]>([]);

// ---------- running and results ----------
export const [running, setRunning] = createSignal(false);
export const [paused, setPaused] = createSignal(false);
export const [progress, setProgress] = createSignal(0);
export const [results, setResults] = createSignal<ResultsInfo | null>(null);
/** The channels of the run in progress (or the last one) and their live samples. */
export const [chans, setChans] = createSignal<Chan[]>([]);
export const live = { gen: 0, t: [] as number[], y: [] as number[][] };
export const [liveTick, setLiveTick] = createSignal(0);
/** Scopes shown in the dock, by sink block id. */
export const [scopes, setScopes] = createSignal<number[]>([]);
export const [dockH, setDockH] = createSignal(300);
export const [lastStatus, setLastStatus] = createSignal("Ready");

// ---------- editing ----------
export const act = async (op: object, quiet = false): Promise<Out | null> => {
  try { const o = await invoke<Out>("apply", { op }); setOut(o); refreshView(); return o; }
  catch (e) { if (!quiet) toast(String(e)); return null; }
};
export const call = async <T = any>(cmd: string, args?: object): Promise<T | undefined> => {
  try { return await invoke<T>(cmd, args); } catch (e) { toast(String(e)); return undefined; }
};
export const loadOut = async (cmd: string, args?: object) => {
  const o = await call<Out | null>(cmd, args);
  if (o) batch(() => { setOut(o); setBackstage(null); setParent(0); setSel([]); setResults(null); setChans([]); setIssues([]); setScopes([]); setView("model"); refreshView(); });
  return o;
};

// ---------- messages, menus, dialogs ----------
export const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
let timer = 0;
export const toast = (text: string, ok = false) => { setMsg({ text, ok }); clearTimeout(timer); timer = window.setTimeout(() => setMsg(null), ok ? 3000 : 7000); };
export type Item = { label: string; run?: () => void; keys?: string; off?: boolean; on?: boolean; sub?: Item[]; icon?: string } | "-";
export const [menu, setMenu] = createSignal<{ x: number; y: number; items: Item[] } | null>(null);
export const openMenu = (e: MouseEvent | { x: number; y: number }, items: Item[]) => {
  if ("preventDefault" in e) e.preventDefault();
  const r = (e as MouseEvent).currentTarget instanceof HTMLElement && (e as MouseEvent).type === "click" ? ((e as MouseEvent).currentTarget as HTMLElement).getBoundingClientRect() : null;
  setMenu({ x: r ? r.left : (e as MouseEvent).clientX ?? (e as any).x, y: r ? r.bottom + 2 : (e as MouseEvent).clientY ?? (e as any).y, items });
};
export const [dialogs, setDialogs] = createSignal<(() => JSX.Element)[]>([]);
export const openDialog = (d: () => JSX.Element) => setDialogs(v => [...v, d]);
export const closeDialog = () => setDialogs(v => v.slice(0, -1));
