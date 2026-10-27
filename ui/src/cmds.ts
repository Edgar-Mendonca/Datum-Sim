// Commands: what the ribbon, menus, keys and Tell Me run.
import { batch } from "solid-js";
import { invoke, listen, Out, Started, Live, Done, ResultsInfo, Issue, Pt } from "./api";
import {
  out, setOut, act, call, sel, setSel, selBlocks, parent, refreshView, setBackstage, loadOut, toast, setView, View, setIssues, setRunning, running, setPaused, paused,
  setProgress, setResults, setChans, live, setLiveTick, scopes, setScopes, dview, setLastStatus, openDialog, enter, issues,
} from "./store";
import { fit, zoomBy, viewCenter, quickInsert } from "./canvas";
import * as D from "./dialogs";

const blockIds = () => selBlocks().map(b => b.id);

export const C = {
  // ---------- file ----------
  save: async (asNew = false) => { const o = await call<Out | null>("save", { asNew }); if (o) { setOut(o); toast("Saved.", true); } },
  open: () => loadOut("open", { path: null }),
  newModel: (example: string | null = null) => loadOut("new_model", { example }).then(o => { if (o) setTimeout(fit, 60); }),
  print: () => setBackstage("print"),
  undo: async () => { const o = await call<Out>("history", { redo: false }); if (o) { setOut(o); refreshView(); } },
  redo: async () => { const o = await call<Out>("history", { redo: true }); if (o) { setOut(o); refreshView(); } },

  // ---------- editing ----------
  addBlock: async (kind: string, at?: Pt) => {
    const p = at ?? viewCenter();
    const o = await act({ Add: { parent: parent(), kind, x: p[0], y: p[1] } });
    if (o?.created.length) setSel(o.created);
  },
  quickAdd: () => quickInsert(),
  del: () => { if (sel().length) act({ Delete: { ids: sel() } }).then(() => setSel([])); },
  copy: async () => { const ids = blockIds(); if (!ids.length) return toast("Select blocks to copy."); if (await call<boolean>("copy", { ids })) { const o = await call<Out>("state"); if (o) setOut(o); toast(`Copied ${ids.length} block${ids.length > 1 ? "s" : ""}.`, true); } },
  cut: async () => { await C.copy(); C.del(); },
  paste: async () => { const o = await call<Out>("paste", { parent: parent(), dx: 30, dy: 30 }); if (o) { setOut(o); setSel(o.created); refreshView(); } },
  flip: () => { const ids = blockIds(); if (ids.length) act({ Flip: { ids } }); },
  rename: () => { const b = selBlocks()[0]; if (b) D.rename(b.id, b.name); else toast("Select a block to rename."); },
  group: async () => { const ids = blockIds(); if (!ids.length) return toast("Select the blocks to put in a subsystem."); const o = await act({ Group: { ids } }); if (o?.created.length) setSel(o.created); },
  ungroup: (id?: number) => { const b = id ?? selBlocks().find(x => x.sub)?.id; if (b) act({ Ungroup: { id: b } }).then(() => setSel([])); else toast("Select a subsystem to ungroup."); },
  align: (how: "left" | "top" | "middle") => {
    const bs = selBlocks(); if (bs.length < 2) return toast("Select two or more blocks to align.");
    const ops = how === "left" ? bs.map(b => ({ Move: { ids: [b.id], dx: bs[0].x - b.x, dy: 0 } }))
      : how === "top" ? bs.map(b => ({ Move: { ids: [b.id], dx: 0, dy: bs[0].y - b.y } }))
        : bs.map(b => ({ Move: { ids: [b.id], dx: 0, dy: Math.round(bs[0].y + bs[0].h / 2 - b.y - b.h / 2) } }));
    act({ Batch: ops });
  },
  setSim: (k: string, v: number | string) => { const s = out()?.sim; if (s) act({ SetSim: { sim: { ...s, [k]: v } } }); },

  // ---------- running ----------
  run: async () => {
    if (running()) return;
    try {
      const st = await invoke<Started>("run");
      batch(() => {
        live.gen = st.gen; live.t = []; live.y = st.chans.map(() => []);
        setChans(st.chans); setResults(null); setRunning(true); setPaused(false); setProgress(0); setIssues(st.warnings);
        setLastStatus(st.warnings.length ? `Running · ${st.warnings.length} warning${st.warnings.length > 1 ? "s" : ""}` : "Running…");
        // Open the model's scopes in the dock on the first run.
        if (!scopes().length) setScopes([...new Set(st.chans.filter(c => ["Scope", "XY"].includes(dview()?.blocks.find(b => b.id === c.sink)?.kind ?? "Scope")).map(c => c.sink))].slice(0, 4));
        setLiveTick(t => t + 1);
      });
      // A short run can finish before this call returns: handle its end now.
      const early = earlyDone.get(st.gen);
      if (early) { earlyDone.delete(st.gen); finish(early); }
    } catch (e) {
      const list = e as Issue[] | string;
      if (Array.isArray(list)) { setIssues(list); const first = list.find(i => !i.warn) ?? list[0]; toast(first?.msg ?? "The model has errors."); setLastStatus(`${list.filter(i => !i.warn).length} error(s): see Issues`); if (first?.id) C.reveal(first.id); }
      else toast(String(e));
    }
  },
  stop: () => invoke("control", { what: "stop" }),
  pause: () => { invoke("control", { what: paused() ? "resume" : "pause" }); setPaused(!paused()); },
  step: () => { if (!paused()) { invoke("control", { what: "pause" }); setPaused(true); } else invoke("control", { what: "step" }); },
  check: async () => {
    const list = await call<Issue[]>("check");
    if (!list) return;
    setIssues(list);
    if (!list.length) toast("The model is ready to run: no errors or warnings.", true);
    else { setView("issues"); toast(`${list.filter(i => !i.warn).length} error(s), ${list.filter(i => i.warn).length} warning(s).`); }
  },
  /** Select a block wherever it is (opens its subsystem). */
  reveal: async (id: number) => {
    const inView = dview()?.blocks.some(b => b.id === id);
    if (!inView) {
      // Search the subsystems for it.
      const find = async (p: number): Promise<number | null> => {
        const v = await invoke<any>("diagram", { parent: p }).catch(() => null); if (!v) return null;
        if (v.blocks.some((b: any) => b.id === id)) return p;
        for (const b of v.blocks.filter((b: any) => b.sub)) { const r = await find(b.id); if (r !== null) return r; }
        return null;
      };
      const p = await find(0); if (p !== null && p !== parent()) enter(p);
    }
    setView("model"); setSel([id]);
  },
  sweep: () => D.sweepDialog(),
  vars: () => setView("vars"),
  view: (v: View) => setView(v),
  zoomIn: () => zoomBy(1.2), zoomOut: () => zoomBy(1 / 1.2), fit,

  // ---------- analysis and results ----------
  analyze: (what: string) => {
    const b = selBlocks()[0];
    if (!b) return toast("Select a transfer function, state-space, PID, gain, integrator or DC motor block first.");
    D.analysis(b.id, b.name, what);
  },
  stepInfo: () => D.stepInfoDialog(),
  exportCsv: async (sink: number | null = null) => { const p = await call<string | null>("export_csv", { sink }); if (p) toast(`Saved ${p}`, true); },
  toPlot: async () => {
    const p = await call<string | null>("export_csv", { sink: null });
    if (p) openDialog(() => D.plotHint(p));
  },
  engine: async (restart = false) => { const v = await call<Record<string, string>>("engine", { restart }); if (v) toast(`Python ${v.python} · numpy ${v.numpy} · SciPy ${v.scipy} · python-control ${v.control}`, true); },
};

/** Live samples from the run thread, and its end. */
export const wireRunEvents = () => {
  listen("sim-live", (l: Live) => {
    if (l.gen !== live.gen) return;
    live.t.push(...l.t);
    l.y.forEach((ys, i) => live.y[i]?.push(...ys));
    setProgress(l.progress);
    setLiveTick(t => t + 1);
  });
  listen("sim-done", (d: Done) => {
    if (d.gen > live.gen) { earlyDone.set(d.gen, d); return; }
    if (d.gen === live.gen) finish(d);
  });
};

/** Ends of runs that arrived before the run's own start call returned. */
const earlyDone = new Map<number, Done>();
const finish = async (d: Done) => {
  batch(() => { setRunning(false); setPaused(false); setProgress(d.ok ? 1 : 0); });
  if (!d.ok) { toast(d.error ?? "The run failed."); setLastStatus("Run failed"); setIssues([...issues(), { id: 0, msg: d.error ?? "The run failed.", warn: false }]); return; }
  const r = await invoke<ResultsInfo | null>("results");
  batch(() => {
    setResults(r); setLiveTick(t => t + 1);
    if (d.issues.length) setIssues([...issues(), ...d.issues]);
    const s = d.stats!;
    setLastStatus(`${s.stopped ? "Stopped" : "Done"} · ${s.steps.toLocaleString()} steps · ${s.events ? `${s.events} events · ` : ""}${s.ms < 1000 ? `${Math.round(s.ms)} ms` : `${(s.ms / 1000).toFixed(1)} s`}${d.truncated ? " · results cut at 2 million points" : ""}`);
  });
  if (d.files.length) toast(`Wrote ${d.files.join(", ")}`, true);
  refreshView();
};

export const ALL_CMDS: [string, () => void, string?][] = [
  ["Run", C.run, "F5"], ["Stop", C.stop], ["Pause / Resume", C.pause], ["Step", C.step], ["Check model", C.check],
  ["Save", () => C.save(), "Ctrl+S"], ["Save As", () => C.save(true)], ["Open", C.open, "Ctrl+O"], ["New model", () => setBackstage("home"), "Ctrl+N"], ["Print / PDF", C.print, "Ctrl+P"],
  ["Undo", C.undo, "Ctrl+Z"], ["Redo", C.redo, "Ctrl+Y"], ["Add block", C.quickAdd, "Double-click"], ["Delete", C.del, "Del"], ["Copy", C.copy, "Ctrl+C"], ["Paste", C.paste, "Ctrl+V"], ["Cut", C.cut, "Ctrl+X"],
  ["Flip block", C.flip, "Ctrl+R"], ["Rename block", C.rename, "F2"], ["Create subsystem", C.group, "Ctrl+G"], ["Ungroup subsystem", () => C.ungroup()],
  ["Model variables", C.vars], ["Parameter sweep", C.sweep], ["Bode plot", () => C.analyze("bode")], ["Step response", () => C.analyze("step")], ["Poles and zeros", () => C.analyze("pz")],
  ["Step-response specifications", C.stepInfo], ["Export results as CSV", () => C.exportCsv()], ["Send results to Datum Plot", C.toPlot],
  ["Diagram", () => setView("model")], ["Scopes", () => setView("scopes")], ["Results table", () => setView("results")], ["Issues", () => setView("issues")],
  ["Zoom in", C.zoomIn], ["Zoom out", C.zoomOut], ["Fit diagram", fit], ["Options", () => setBackstage("options")], ["Check for updates", () => setBackstage("about")],
  ["Keyboard shortcuts", () => D.shortcuts(), "F1"], ["Getting started", () => D.gettingStarted()], ["Python engine version", () => C.engine()],
];
