// Calls into the Rust shell and the shapes it returns.
declare global {
  interface Window { __TAURI__?: { core: { invoke: (cmd: string, args?: object) => Promise<any> }; event: { listen: (e: string, f: (x: { payload: any }) => void) => Promise<() => void> } } }
}

export const invoke = <T = any>(cmd: string, args?: object): Promise<T> =>
  window.__TAURI__ ? window.__TAURI__.core.invoke(cmd, args) : Promise.reject("Datum Sim must run inside its desktop window.");
export const listen = (e: string, f: (p: any) => void) => window.__TAURI__?.event.listen(e, x => f(x.payload));

export type Pt = [number, number];
export interface Var { name: string; expr: string; unit: string; note: string }
export interface SimSettings { start: number; stop: number; solver: string; step: number; max_step: number; rtol: number; atol: number }
export interface Out {
  name: string; notes: string; vars: Var[]; sim: SimSettings; path: string | null; dirty: boolean; undo: boolean; redo: boolean;
  clip: boolean; running: boolean; rev: number; created: number[];
}
export interface BlockView {
  id: number; kind: string; name: string; x: number; y: number; w: number; h: number; flip: boolean; label: string;
  ins: Pt[]; outs: Pt[]; in_names: string[]; out_names: string[]; params: Record<string, string>; sub: boolean;
}
export interface WireView { id: number; from: [number, number]; to: [number, number]; pts: Pt[]; path: Pt[] }
export interface DView { parent: number; path: [number, string][]; blocks: BlockView[]; wires: WireView[] }
export interface ParamDef { key: string; label: string; def: string; unit: string; hint: string; kind: "num" | "text" | "choice"; choices: string[] }
export interface Preset { name: string; values: [string, string][] }
export interface Def { kind: string; name: string; cat: string; what: string; how: string; params: ParamDef[]; w: number; h: number; planned: string; presets: Preset[] }
export interface Library { cats: string[]; blocks: Def[] }
export interface Issue { id: number; msg: string; warn: boolean }
export interface Chan { sink: number; port: number; elem: number; name: string }
export interface RunStats { steps: number; rejected: number; fevals: number; events: number; stopped: boolean; ms: number }
export interface ResultsInfo { chans: Chan[]; stats: RunStats; n: number; t0: number; t1: number; last: number[]; sweep: [string, number[]] | null }
export interface Series { t: number[]; y: number[] }
export interface Example { id: string; name: string; what: string }
export interface Options {
  autosave: number; beta: boolean; theme: string; openLast: boolean; solver: string; stop: number; snap: boolean; grid: boolean;
  paper: string; landscape: boolean; company: string;
}
export interface Started { gen: number; chans: Chan[]; warnings: Issue[]; start: number; stop: number }
export interface Live { gen: number; t: number[]; y: number[][]; progress: number }
export interface Done { gen: number; ok: boolean; error: string | null; issues: Issue[]; stats: RunStats | null; truncated: boolean; files: string[] }

/** Numbers for people: up to 4 significant digits, no trailing zeros. */
export const num = (v: number, sig = 4) => {
  if (!isFinite(v)) return v > 0 ? "∞" : v < 0 ? "−∞" : "—";
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a >= 1e5 || a < 1e-3) return v.toExponential(Math.max(0, sig - 1)).replace(/\.?0+e/, "e");
  return String(+v.toPrecision(sig));
};
