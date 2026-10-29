// File (backstage): home with examples and recent models, export, print, options, roadmap, updates.
import { For, Show, createSignal, createResource, onMount, JSX } from "solid-js";
import { PAPERS, PrintPage } from "./print";
import { RoadmapPage } from "./roadmap";
import { Options, Out, Example, listen, invoke } from "./api";
import { backstage, setBackstage, call, loadOut, out, setOut, opts, setOpts, toast, setThemeRaw, applyTheme, results } from "./store";
import { C } from "./cmds";
import * as D from "./dialogs";
import { Icon, Text, Pick, Check } from "./ui";
import { SOLVERS } from "./panels";

type Info = { current: string; latest: string | null; beta: boolean; available: boolean; notes: string; published: string; url: string | null; sums: string | null; size: number; page: string };
export const [update, setUpdate] = createSignal<Info | null>(null);
export const [checking, setChecking] = createSignal(false), [checkedAt, setCheckedAt] = createSignal("");
export const checkUpdate = async (quiet: boolean) => {
  setChecking(true);
  try { setUpdate(await invoke<Info>("check_update")); setCheckedAt(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })); if (!quiet) toast(update()?.available ? `Version ${update()!.latest} is available.` : "You have the latest version.", true); }
  catch (e) { if (!quiet) toast(`Could not check for updates: ${e}`); }
  finally { setChecking(false); }
};
const GH = "https://github.com/Edgar-Mendonca/Datum-Sim";
export const GitHubLinks = () => (
  <div class="ghlinks">
    <For each={[["", "GitHub", "The project and its code"], ["/releases", "Releases", "Every version and its change log"], ["/issues/new", "Report an issue", "Tell us about a bug or an idea"]]}>{([path, l, t]) =>
      <button class="btn" title={t} onClick={() => call("open_url", { url: GH + path })}><svg class="ic"><use href={path === "/issues/new" ? "#i-bug" : path ? "#i-tag" : "#i-github"} /></svg>{l}</button>}</For>
  </div>
);
const md = (s: string) => s.split("\n").map(l => {
  const e = l.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>").replace(/`(.+?)`/g, "<code>$1</code>");
  return l.startsWith("## ") ? `<h3>${e.slice(3)}</h3>` : l.startsWith("- ") ? `<li>${e.slice(2)}</li>` : e.trim() ? `<p>${e}</p>` : "";
}).join("");

/** A small drawing for each example tile. */
const ExampleArt = (p: { id: string }) => {
  const s = { fill: "none", stroke: "var(--blue)", "stroke-width": 2, "stroke-linecap": "round" as const, "stroke-linejoin": "round" as const };
  const curves: Record<string, string> = {
    blank: "M10 40h80",
    msd: "M8 46H24C32 46 34 14 42 14S52 38 58 38 66 22 72 24 82 30 92 29",
    dcmotor: "M8 46c6 0 8-30 16-30s10 6 18 6h20c4 0 4 8 8 8s4-6 8-6h14",
    dcpos: "M8 46H14C22 46 26 16 36 15S50 16 92 16",
    pmsm: "M8 46C20 46 22 16 32 16H58C62 16 62 24 66 24S72 16 92 16",
    imdol: "M8 46H12C24 46 36 42 46 30S56 14 62 13 66 16 70 16H92",
    imvf: "M8 46L54 16H68C70 21 72 19 76 19H92",
    pmsmfoc: "M8 46H12L30 16H62C64 22 66 18 70 17H92",
    bldc: "M8 46C14 46 18 16 28 16H62C64 22 66 18 70 17H92",
    syncmotor: "M8 20H34C38 20 38 34 42 34S46 14 50 14 54 28 58 28 62 18 66 18 70 24 74 24 78 22 92 22",
    dcseries: "M8 46C30 44 44 24 60 20S80 16 92 16",
    stepper: "M8 46h10v-6h10v-6h10v-6h10v-6h10v-6h10v-6h14",
    pmsmdigital: "M8 46h4v-6h4v-8h4v-8h4v-6h4v-2h40v4h4v6h4v6h4v6h4v6h4",
    dcdigital: "M8 46h6v-10h6v-8h6v-6h6v-4h6v-2h48",
    srm: "M8 46C16 46 20 18 30 18H56L60 26 66 20 72 22 92 21",
    tank: "M8 34H30C40 34 44 16 56 16H92",
    cruise: "M8 32H24C34 32 40 18 52 18H92",
    thermostat: "M8 46C18 26 24 20 36 20L44 28 52 20 60 28 68 20 76 28 84 20 92 28",
  };
  return <svg viewBox="0 0 100 60" class="exart"><path d="M6 6v48h88" fill="none" stroke="var(--line3)" stroke-width="1.2" /><path d={curves[p.id] ?? curves.blank} {...s} /></svg>;
};


const Home = () => {
  const [ex] = createResource(() => call<Example[]>("examples_list"));
  const [recent] = createResource(() => call<{ path: string; name: string; modified: string }[]>("recent"));
  return (
    <div class="home">
      <div>
        <h1>Good {new Date().getHours() < 12 ? "morning" : new Date().getHours() < 18 ? "afternoon" : "evening"}</h1>
        <div class="tiles">
          <For each={ex() ?? []}>{t => <button class="tile" onClick={() => C.newModel(t.id === "blank" ? null : t.id)}><div class="pic"><ExampleArt id={t.id} /></div><b>{t.name}</b><span>{t.what}</span></button>}</For>
          <button class="tile" onClick={() => setBackstage("roadmap")}><div class="pic"><svg class="ic"><use href="#i-list" /></svg></div><b>Roadmap</b><span>Everything Datum Sim will do, and when.</span></button>
        </div>
        <h2>Recent</h2>
        <table class="recent"><tbody>
          <For each={recent() ?? []} fallback={<tr><td class="muted">Models you open or save appear here.</td></tr>}>{r => <tr class="f" onClick={() => loadOut("open", { path: r.path })}><td><b>{r.name}</b><div class="p">{r.path}</div></td><td class="p">{r.modified}</td></tr>}</For>
        </tbody></table>
      </div>
      <UpdateCard />
    </div>
  );
};

const UpdateCard = () => {
  const [pct, setPct] = createSignal<number | null>(null);
  const [ver] = createResource(() => call<string>("version"));
  onMount(() => { listen("update-progress", (p: number) => setPct(p)); if (!update()) checkUpdate(true); });
  const install = async () => { const u = update(); if (!u?.url) return; setPct(0); await call("install_update", { url: u.url, sums: u.sums }); setPct(null); };
  return (
    <div class="card">
      <div class="brand"><img src="logo.svg" alt="" /><div><b>Datum Sim</b><span class="muted">Version {ver()}{opts().beta ? " · beta channel" : ""}</span></div></div>
      <Show when={update()} fallback={<><span class="pill">{checking() ? "Checking for updates…" : "Not checked yet"}</span><button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check for updates"}</button><GitHubLinks /></>}>{u => <>
        <Show when={u().available} fallback={<span class="pill ok">You have the latest version</span>}><span class="pill warn">Version {u().latest}{u().beta ? " (beta)" : ""} is ready</span></Show>
        <Show when={u().available && u().url}><button class="btn pri wide" disabled={pct() !== null} onClick={install}>{pct() === null ? `Update now (${Math.round(u().size / 1048576)} MB)` : `Downloading… ${pct()}%`}</button></Show>
        <Show when={pct() !== null}><div class="prog"><i style={{ width: `${pct()}%` }} /></div></Show>
        <button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check again"}</button>
        <Show when={checkedAt() && !checking()}><span class="muted small">Checked at {checkedAt()}</span></Show>
        <GitHubLinks />
        <div class="notes"><h4>What's new{u().latest ? ` in ${u().latest}` : ""}</h4><div innerHTML={md(u().notes || "Release notes will appear here.")} /></div>
      </>}</Show>
    </div>
  );
};

const Export = () => (
  <div><h1>Export</h1>
    <div class="tiles">
      <button class="tile" onClick={() => C.exportCsv()} disabled={!results()}><div class="pic"><svg class="ic"><use href="#i-csv" /></svg><small>.csv</small></div><b>Results as CSV</b><span>Time and every logged signal, for any spreadsheet.</span></button>
      <button class="tile" onClick={C.toPlot} disabled={!results()}><div class="pic"><svg class="ic"><use href="#i-plot" /></svg></div><b>Send to Datum Plot</b><span>The results as a CSV Datum Plot opens, to analyse and make publication graphs.</span></button>
      <button class="tile" onClick={() => setBackstage("print")}><div class="pic"><svg class="ic"><use href="#i-print" /></svg></div><b>Print or save as PDF</b><span>The diagram, the scopes and a results report.</span></button>
      <button class="tile" disabled><div class="pic"><svg class="ic"><use href="#i-export" /></svg><small>0.5</small></div><b>FMU</b><span>Export the model for other simulation tools (FMI standard). Planned for 0.5.</span></button>
    </div>
    <Show when={!results()}><p class="muted">Run the model to export its results.</p></Show>
  </div>
);

const Opt = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="opt"><div><b>{p.l}</b>{p.hint && <small>{p.hint}</small>}</div><div class="ctl">{p.children}</div></div>;
const Sec = (p: { t: string; about: string; children: JSX.Element }) => <section class="osec"><h3>{p.t}</h3><p class="muted">{p.about}</p>{p.children}</section>;

const OptionsPage = () => {
  const [o, setO] = createSignal<Options>({ ...opts() });
  const save = async () => { const r = await call<Out>("set_options", { opts: o() }); if (r) { setOpts(o()); setOut(r); toast("Options saved.", true); } };
  const put = <K extends keyof Options>(k: K, v: Options[K]) => setO({ ...o(), [k]: v });
  return (
    <div class="options"><h1>Options</h1>
      <p class="muted">Settings for Datum Sim on this computer. Defaults for new models apply when a model is created; the run settings of an open model are in Simulate.</p>
      <Sec t="General" about="How the window looks.">
        <Opt l="Theme" hint="Auto follows the Windows light or dark setting."><Pick value={o().theme || "auto"} set={v => { put("theme", v); setThemeRaw(v); applyTheme(); }} options={[["auto", "Auto (follow Windows)"], ["light", "Light"], ["dark", "Dark"]]} /></Opt>
        <Opt l="Diagram grid" hint="Dots on the diagram, and blocks snapping to them while dragging."><span class="inline"><Check label="Show the grid" value={o().grid} set={v => put("grid", v)} /><Check label="Snap to the grid" value={o().snap} set={v => put("snap", v)} /></span></Opt>
      </Sec>
      <Sec t="Start-up and saving" about="What happens when Datum Sim starts, and how your work is protected.">
        <Opt l="When Datum Sim starts" hint="Show the File page, or reopen the model you used last."><Pick value={o().openLast ? "last" : "home"} set={v => put("openLast", v === "last")} options={[["home", "Show the File page"], ["last", "Reopen the last model"]]} /></Opt>
        <Opt l="Recovery copy every" hint="A copy is kept while you work, so a crash or power cut loses little. 0 turns it off."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={o().autosave} onChange={e => put("autosave", +e.currentTarget.value)} /> minutes</span></Opt>
      </Sec>
      <Sec t="New models" about="Defaults given to every new model. Each model keeps its own settings afterwards.">
        <Opt l="Solver" hint="Variable step suits most models; Stiff is for very fast and very slow parts together."><Pick value={o().solver} set={v => put("solver", v)} options={SOLVERS} /></Opt>
        <Opt l="Stop time" hint="How long a new model runs, in seconds."><span class="inline"><input type="number" min="0" style={{ width: "90px" }} value={o().stop} onChange={e => put("stop", +e.currentTarget.value)} /> s</span></Opt>
      </Sec>
      <Sec t="Printing" about="Defaults for File › Print.">
        <Opt l="Paper" hint="Default paper for printouts."><span class="inline"><Pick value={o().paper || "A4"} set={v => put("paper", v)} options={PAPERS.map(p => p[0])} /><Pick value={o().landscape === false ? "0" : "1"} set={v => put("landscape", v === "1")} options={[["1", "Landscape"], ["0", "Portrait"]]} /></span></Opt>
        <Opt l="Company" hint="Shown in the header of printouts."><Text value={o().company} set={v => put("company", v)} /></Opt>
      </Sec>
      <Sec t="Updates" about="Datum Sim checks GitHub for new versions when it starts. Updates install from File › Updates.">
        <Opt l="Beta versions" hint="Get new features early. They may have bugs."><Check label="Get beta versions" value={o().beta} set={v => put("beta", v)} /></Opt>
      </Sec>
      <p class="osave"><button class="btn pri" onClick={save}>Save options</button><span class="muted">Datum Sim keeps everything on this computer. Its only connection is the update check to GitHub.</span></p>
    </div>
  );
};

export const Backstage = () => (
  <div class="bs">
    <aside>
      <button class="back" onClick={() => out() && setBackstage(null)} title="Back">←</button>
      <For each={[["home", "home", "Home"], ["new", "new", "New"], ["open", "open", "Open"], ["save", "save", "Save"], ["saveas", "save", "Save As"], ["export", "export", "Export"], ["print", "print", "Print"], ["roadmap", "list", "Roadmap"]]}>{([k, i, l]) =>
        <button classList={{ on: backstage() === k }} onClick={() => {
          if (k === "open") C.open(); else if (k === "save") C.save(); else if (k === "saveas") C.save(true);
          else setBackstage(k === "new" ? "home" : k);
        }}><Icon n={i} size={22} />{l}</button>}</For>
      <span class="grow" />
      <button classList={{ on: backstage() === "options" }} onClick={() => setBackstage("options")}><Icon n="opts" size={22} />Options</button>
      <button classList={{ on: backstage() === "about" }} onClick={() => setBackstage("about")}><Icon n="upd" size={22} />Updates</button>
    </aside>
    <section class="bsmain">
      <Show when={backstage() === "home"}><Home /></Show>
      <Show when={backstage() === "export"}><Export /></Show>
      <Show when={backstage() === "options"}><OptionsPage /></Show>
      <Show when={backstage() === "print"}><PrintPage /></Show>
      <Show when={backstage() === "roadmap"}><RoadmapPage /></Show>
      <Show when={backstage() === "about"}><div class="about"><h1>Updates</h1><UpdateCard /><p><button class="btn" onClick={() => D.shortcuts()}>Keyboard shortcuts</button></p></div></Show>
    </section>
  </div>
);
