// Datum Sim window: layout, sidebar, status bar, keyboard shortcuts, startup and recovery.
import { render } from "solid-js/web";
import { For, Show, Switch, Match, onMount } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Library, Options, num } from "./api";
import {
  out, setOut, view, setView, View, backstage, setBackstage, setLib, setOpts, opts, dialogs, toast, loadOut, setThemeRaw, applyTheme, refreshView,
  scopes, dockH, setDockH, running, paused, progress, lastStatus, issues, sel, dview, results,
} from "./store";
import { C, wireRunEvents } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Backstage, update, checkUpdate } from "./backstage";
import { Canvas, fit, zoom } from "./canvas";
import { LibraryPanel, Inspector } from "./panels";
import { Dock, ScopesPage, ResultsPage } from "./scope";
import { Menu, Toast, Icon } from "./ui";
import * as D from "./dialogs";

const SIDE: ([View, string, string] | "-")[] = [
  ["model", "model", "Diagram"], ["vars", "var", "Variables"], "-", ["scopes", "scope", "Scopes"], ["results", "results", "Results"], "-", ["issues", "issues", "Issues"],
];

const ModelView = () => {
  const drag = (e: MouseEvent) => {
    e.preventDefault();
    const y0 = e.clientY, h0 = dockH();
    const mv = (m: MouseEvent) => setDockH(Math.min(innerHeight - 260, Math.max(140, h0 - (m.clientY - y0))));
    const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); };
    addEventListener("mousemove", mv); addEventListener("mouseup", up);
  };
  return (
    <div class="modelview">
      <LibraryPanel />
      <div class="center">
        <Canvas />
        <Show when={scopes().length}><div class="dsplit" onMouseDown={drag} /><div style={{ height: `${dockH()}px`, display: "flex" }}><Dock /></div></Show>
      </div>
      <Inspector />
    </div>
  );
};

const Status = () => {
  const errs = () => issues().filter(i => !i.warn).length, warns = () => issues().filter(i => i.warn).length;
  return (
    <footer class="status">
      <span classList={{ late: errs() > 0 }}>{running() ? (paused() ? "Paused" : "Running") : lastStatus()}</span>
      <Show when={running()}><span class="runprog"><i style={{ width: `${Math.round(progress() * 100)}%` }} /></span><span class="muted">{Math.round(progress() * 100)} %</span></Show>
      <Show when={errs() || warns()}><button class="mode" onClick={() => setView("issues")}>{errs() ? `${errs()} error${errs() > 1 ? "s" : ""}` : ""}{errs() && warns() ? " · " : ""}{warns() ? `${warns()} warning${warns() > 1 ? "s" : ""}` : ""}</button></Show>
      <span class="muted">{dview() ? `${dview()!.blocks.length} blocks · ${dview()!.wires.length} wires` : ""}{sel().length ? ` · ${sel().length} selected` : ""}</span>
      <span class="muted">{out() ? `${num(out()!.sim.start)}–${num(out()!.sim.stop)} s · ${out()!.sim.solver}` : ""}</span>
      <Show when={results()}><span class="muted">{results()!.n.toLocaleString()} points</span></Show>
      <span class="grow" />
      <span class="vsc">
        <button title="Diagram" onClick={() => setView("model")}><Icon n="model" /></button><button title="Scopes" onClick={() => setView("scopes")}><Icon n="scope" /></button><button title="Results" onClick={() => setView("results")}><Icon n="results" /></button>
      </span>
      <button onClick={C.zoomOut}>−</button><span class="muted zoomv">{Math.round(zoom() * 100)}%</span><button onClick={C.zoomIn}>+</button>
    </footer>
  );
};

const App = () => {
  onMount(async () => {
    try {
      setLib(await invoke<Library>("library"));
      const o = await invoke<Options>("options"); setOpts(o); setThemeRaw(o.theme || "auto"); applyTheme();
      setOut(await invoke<Out>("state"));
      refreshView();
      wireRunEvents();
      const start = await invoke<string | null>("startup_file");
      if (start) { await loadOut("open", { path: start }); setTimeout(fit, 80); }
      const rec = await invoke<{ name: string; saved: string } | null>("recovery");
      if (rec) D.recoverDialog(rec.name, rec.saved, () => { setBackstage(null); refreshView(); });
      checkUpdate(true);
      setInterval(() => { if (opts().autosave > 0) invoke("autosave").catch(() => {}); }, Math.max(1, opts().autosave) * 60000);
      listen("close-requested", () => D.confirmClose(() => invoke("quit")));
    } catch (e) { toast(String(e)); }
  });
  const keys = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (dialogs().length) return;
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
    if ((ctrl && k === "k") || (e.altKey && k === "q")) { e.preventDefault(); palette(); }
    else if (ctrl && k === "s") { e.preventDefault(); C.save(e.shiftKey); }
    else if (ctrl && k === "o") { e.preventDefault(); C.open(); }
    else if (ctrl && k === "n") { e.preventDefault(); setBackstage("home"); }
    else if (ctrl && k === "p") { e.preventDefault(); C.print(); }
    else if (e.key === "F5") { e.preventDefault(); if (e.shiftKey) C.stop(); else C.run(); }
    else if (ctrl && k === "z" && !typing) { e.preventDefault(); C.undo(); }
    else if (ctrl && k === "y" && !typing) { e.preventDefault(); C.redo(); }
    else if (e.key === "F1") { e.preventDefault(); D.shortcuts(); }
    else if (e.key === "Escape" && backstage() && out()) setBackstage(null);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <main class="work">
        <nav class="side"><For each={SIDE}>{x => x === "-" ? <hr /> : <button classList={{ on: view() === x[0] }} onClick={() => setView(x[0])} title={x[2]}><Icon n={x[1]} /><span>{x[2]}</span></button>}</For></nav>
        <div class="content">
          <Show when={out()}>
            <Switch fallback={<ModelView />}>
              <Match when={view() === "vars"}><D.VarsPage /></Match>
              <Match when={view() === "scopes"}><ScopesPage /></Match>
              <Match when={view() === "results"}><ResultsPage /></Match>
              <Match when={view() === "issues"}><D.IssuesPage /></Match>
            </Switch>
          </Show>
        </div>
      </main>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
