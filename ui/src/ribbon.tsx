// Title bar, quick access toolbar, Tell Me and the ribbon (Model · Simulate · Analyze · View · Help,
// with Format as a contextual tab when blocks are selected). Copied from Datum Plan's ribbon.
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { call, out, tab, setTab, setBackstage, view, openMenu, Item, openDialog, closeDialog, theme, setTheme, running, paused, selBlocks, opts, setOpts, scopes, setScopes, lib } from "./store";
import { C, ALL_CMDS } from "./cmds";
import * as D from "./dialogs";
import { Icon } from "./ui";
import { Help, helpCard, HELP } from "./help";
import { SolverPick } from "./panels";
import { invoke } from "./api";

const GH = "https://github.com/Edgar-Mendonca/Datum-Sim";

const B = (p: { i: string; l: string; run?: (e: MouseEvent) => void; big?: boolean; on?: boolean; off?: boolean; dd?: (e: MouseEvent) => void; title?: string }) => p.big && p.run && p.dd ? (
  <div class="rbw big splitbig" classList={{ on: p.on }}>
    <button class="rb top" title={p.title ?? p.l} disabled={p.off} onClick={e => p.run!(e)}><Icon n={p.i} /></button>
    <button class="rb bot" title={`${p.l} options`} onClick={e => p.dd!(e)}><span>{p.l}</span><b class="car">▾</b></button>
  </div>
) : (
  <div class="rbw" classList={{ big: p.big }}>
    <button class="rb" classList={{ big: p.big, on: p.on }} disabled={p.off} title={p.title ?? p.l} onClick={e => (p.run ?? p.dd)?.(e)}><Icon n={p.i} />{p.big ? <span>{p.l}{p.dd && !p.run ? " ▾" : ""}</span> : <span>{p.l}</span>}{!p.big && p.dd && !p.run && <b class="car">▾</b>}</button>
    {p.run && p.dd && <button class="rb split" onClick={e => p.dd!(e)}>▾</button>}
  </div>
);
const G = (p: { l: string; h?: string; children: JSX.Element }) => <div class="grp"><div class="items">{p.children}</div><div class="cap">{p.l}{p.h && <Help k={p.h} />}</div></div>;
const S = (p: { children: JSX.Element }) => <div class="stack">{p.children}</div>;
/** A planned tool: shown dimmed with the release it arrives in. */
const Planned = (p: { i: string; l: string; v: string }) => <B i={p.i} l={p.l} off title={`${p.l}: planned for ${p.v}`} />;

/** Tell Me / command search (Ctrl+K). */
export const palette = () => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const all = (): [string, () => void, string?][] => [...ALL_CMDS, ...lib().blocks.filter(d => !d.planned).map(d => [`Add ${d.name}`, () => C.addBlock(d.kind), d.cat] as [string, () => void, string])];
  const list = createMemo(() => { const s = q().toLowerCase(); return all().filter(c => c[0].toLowerCase().includes(s)).slice(0, 14); });
  const go = (i: number) => { const c = list()[i]; if (c) { closeDialog(); setTimeout(() => c[1]()); } };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov top" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Tell me what you want to do" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(c, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{c[0]}</span>{c[2] && <kbd>{c[2]}</kbd>}</button>}</For></div>
      </div>
    </div>
  );
});

export const TitleBar = (p: { update: string | null }) => (
  <header class="top">
    <div class="bar">
      <div class="qat">
        <img src="logo.svg" alt="" class="logo" />
        <button title="Save (Ctrl+S)" onClick={() => C.save()}><Icon n="save" /></button>
        <button title="Undo (Ctrl+Z)" disabled={!out()?.undo} onClick={C.undo}><Icon n="undo" /></button>
        <button title="Redo (Ctrl+Y)" disabled={!out()?.redo} onClick={C.redo}><Icon n="redo" /></button>
        <button title="Run (F5)" disabled={running()} onClick={C.run}><Icon n="run" /></button>
        <button title="Print (Ctrl+P)" onClick={C.print}><Icon n="print" /></button>
      </div>
      <div class="title">{out() ? `${out()!.name}${out()!.dirty ? " •" : ""} — Datum Sim` : "Datum Sim"}</div>
      <button class="search" onClick={palette} title="Tell me what you want to do (Ctrl+K)"><Icon n="search" /><span>Tell me what you want to do</span><kbd>Ctrl+K</kbd></button>
      <Show when={p.update}><button class="upd" onClick={() => setBackstage("about")}>Update {p.update} available</button></Show>
      <button class="theme" title="Theme: Auto follows Windows" onClick={() => setTheme(theme() === "auto" ? "light" : theme() === "light" ? "dark" : "auto")}>Theme · {theme()}</button>
    </div>
    <nav class="tabs">
      <button class="file" onClick={() => setBackstage("home")}>File</button>
      <For each={[["model", "Model"], ["simulate", "Simulate"], ["analyze", "Analyze"], ["view", "View"], ["help", "Help"]]}>{([k, l]) => <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For>
      <div class="ctxgrp" style={{ visibility: selBlocks().length ? "visible" : "hidden" }}><small>Block Tools</small><button classList={{ on: tab() === "format" }} onClick={() => setTab("format")}>Format</button></div>
    </nav>
  </header>
);

/** Shrinks the ribbon in steps instead of scrolling sideways (FAMILY §6). */
const fitRibbon = (el: HTMLElement) => {
  const fit = () => {
    const gs = [...el.querySelectorAll<HTMLElement>(":scope > .grp")];
    el.dataset.fit = "0"; gs.forEach(g => delete g.dataset.fit);
    const over = () => el.scrollWidth > el.clientWidth + 1;
    if (!over()) return;
    el.dataset.fit = "1";
    for (const lvl of ["2", "3"]) for (let i = gs.length - 1; i >= 0 && over(); i--) gs[i].dataset.fit = lvl;
  };
  new ResizeObserver(fit).observe(el);
  new MutationObserver(() => requestAnimationFrame(fit)).observe(el, { childList: true, subtree: false });
};

const exampleMenu = async (e: MouseEvent) => {
  const list = await invoke<{ id: string; name: string }[]>("examples_list").catch(() => []);
  openMenu({ x: (e.currentTarget as HTMLElement).getBoundingClientRect().left, y: (e.currentTarget as HTMLElement).getBoundingClientRect().bottom + 2 }, list.map(x => ({ label: x.name, run: () => C.newModel(x.id) })));
};
const scopeMenu = (): Item[] => [{ label: "Show all scopes", run: () => C.view("scopes") }, { label: "Close the scopes dock", run: () => setScopes([]), off: !scopes().length }];

export const Ribbon = () => (
  <section class="ribbon" ref={fitRibbon}>
    <Show when={tab() === "model"}>
      <G l="Clipboard" h="clipboard"><B big i="paste" l="Paste" run={C.paste} off={!out()?.clip} /><S><B i="cut" l="Cut" run={C.cut} /><B i="copy" l="Copy" run={C.copy} /></S></G>
      <G l="Blocks" h="blocks"><B big i="block" l="Add Block" run={C.quickAdd} title="Find a block by name and add it (or double-click the diagram)" /><S><B i="del" l="Delete" run={C.del} /><B i="flip" l="Flip" run={C.flip} /><B i="rename" l="Rename" run={C.rename} /></S></G>
      <G l="Subsystems" h="subsystems"><B big i="sub" l="Create Subsystem" run={C.group} /><S><B i="ungroup" l="Ungroup" run={() => C.ungroup()} /><Planned i="sub" l="Mask" v="0.2" /></S></G>
      <G l="Variables" h="vars"><B big i="var" l="Variables" run={C.vars} on={view() === "vars"} /></G>
      <G l="Arrange" h="arrange"><S><B i="align" l="Align Left" run={() => C.align("left")} /><B i="align" l="Align Top" run={() => C.align("top")} /><B i="align" l="Align Centres" run={() => C.align("middle")} /></S><S><B i="fit" l="Fit Diagram" run={C.fit} /><B i="grid" l={`Snap ${opts().snap ? "on" : "off"}`} on={opts().snap} run={() => setOpts({ ...opts(), snap: !opts().snap })} /></S></G>
      <G l="Examples" h="blocks"><B big i="open" l="Examples" dd={exampleMenu} /></G>
    </Show>
    <Show when={tab() === "simulate"}>
      <G l="Run" h="run">
        <Show when={!running()} fallback={<B big i="stop" l="Stop" run={C.stop} title="Stop the run (Shift+F5)" />}><B big i="run" l="Run" run={C.run} title="Run the model (F5)" /></Show>
        <S><B i="pause" l={paused() ? "Resume" : "Pause"} run={C.pause} off={!running()} /><B i="step" l="Step" run={C.step} off={!running()} title="While paused: one solver step" /><B i="check" l="Check Model" run={C.check} /></S>
      </G>
      <G l="Time" h="time"><div class="rfields">
        <label>Start <input class="mono" value={out()?.sim.start ?? 0} onChange={e => isFinite(+e.currentTarget.value) && C.setSim("start", +e.currentTarget.value)} /> s</label>
        <label>Stop <input class="mono" value={out()?.sim.stop ?? 10} onChange={e => isFinite(+e.currentTarget.value) && C.setSim("stop", +e.currentTarget.value)} /> s</label></div></G>
      <G l="Solver" h="solver"><div class="rfields"><SolverPick />
        <Show when={["rk4", "heun", "euler"].includes(out()?.sim.solver ?? "")} fallback={<label>Tolerance <input class="mono" value={out()?.sim.rtol ?? 1e-6} onChange={e => +e.currentTarget.value > 0 && C.setSim("rtol", +e.currentTarget.value)} /></label>}>
          <label>Step <input class="mono" value={out()?.sim.step ?? 0.01} onChange={e => +e.currentTarget.value > 0 && C.setSim("step", +e.currentTarget.value)} /> s</label></Show></div></G>
      <G l="Studies" h="sweep"><B big i="sweep" l="Parameter Sweep" run={C.sweep} /><S><Planned i="sweep" l="Monte Carlo" v="0.6" /><Planned i="specs" l="Sensitivity" v="0.6" /><Planned i="solver" l="Operating Point" v="0.3" /></S></G>
      <G l="Scopes" h="scopes"><B big i="scope" l="Scopes" run={() => C.view("scopes")} dd={e => openMenu(e, scopeMenu())} /></G>
    </Show>
    <Show when={tab() === "analyze"}>
      <G l="Linear Analysis" h="linear"><B big i="bode" l="Bode" run={() => C.analyze("bode")} /><B big i="specs" l="Step Response" run={() => C.analyze("step")} /><S><B i="solver" l="Poles and Zeros" run={() => C.analyze("pz")} /><Planned i="bode" l="Nyquist" v="0.3" /><Planned i="bode" l="Root Locus" v="0.3" /></S></G>
      <G l="Control Design" h="linear"><S><Planned i="solver" l="Linearise Model" v="0.3" /><Planned i="specs" l="PID Tuning" v="0.3" /><Planned i="solver" l="Trim" v="0.3" /></S></G>
      <G l="Signals" h="specs"><B big i="specs" l="Step Specs" run={C.stepInfo} /><S><Planned i="plot" l="Fit to Data" v="0.6" /><Planned i="report" l="Report" v="0.6" /></S></G>
      <G l="Results" h="exportres"><B big i="results" l="Results" run={() => C.view("results")} /><S><B i="csv" l="Save as CSV" run={() => C.exportCsv()} /><B i="plot" l="Send to Datum Plot" run={C.toPlot} /><Planned i="export" l="HDF5" v="0.2" /></S></G>
      <G l="Python" h="linear"><S><B i="py" l="Engine Version" run={() => C.engine()} /><B i="py" l="Restart Engine" run={() => C.engine(true)} /></S></G>
    </Show>
    <Show when={tab() === "view"}>
      <G l="Views" h="views"><B big i="model" l="Diagram" run={() => C.view("model")} on={view() === "model"} /><B big i="scope" l="Scopes" run={() => C.view("scopes")} on={view() === "scopes"} />
        <S><B i="var" l="Variables" run={C.vars} on={view() === "vars"} /><B i="results" l="Results" run={() => C.view("results")} on={view() === "results"} /><B i="issues" l="Issues" run={() => C.view("issues")} on={view() === "issues"} /></S></G>
      <G l="Zoom" h="zoom"><S><B i="zin" l="Zoom In" run={C.zoomIn} /><B i="zout" l="Zoom Out" run={C.zoomOut} /><B i="fit" l="Fit" run={C.fit} /></S></G>
      <G l="Show" h="show"><S><label class="chk"><input type="checkbox" checked={opts().grid} onChange={e => setOpts({ ...opts(), grid: e.currentTarget.checked })} />Grid</label><label class="chk"><input type="checkbox" checked={opts().snap} onChange={e => setOpts({ ...opts(), snap: e.currentTarget.checked })} />Snap to grid</label><label class="chk"><input type="checkbox" checked={scopes().length > 0} onChange={e => setScopes(e.currentTarget.checked ? [] : [])} disabled={!scopes().length} />Scopes dock</label></S></G>
    </Show>
    <Show when={tab() === "format"}>
      <G l="Block" h="blocks"><B big i="flip" l="Flip" run={C.flip} /><B big i="rename" l="Rename" run={C.rename} /><S><B i="align" l="Align Left" run={() => C.align("left")} /><B i="align" l="Align Top" run={() => C.align("top")} /><B i="align" l="Align Centres" run={() => C.align("middle")} /></S></G>
      <G l="Look" h="blocks"><S><Planned i="grid" l="Block Colour" v="0.2" /><Planned i="rename" l="Annotation" v="0.2" /></S></G>
    </Show>
    <Show when={tab() === "help"}>
      <G l="Learn"><B big i="report" l="Getting Started" run={() => D.gettingStarted()} /><B big i="info" l="Glossary" run={() => D.glossary()} /><B big i="help" l="What Is This Tool?" dd={e => openMenu(e, Object.entries(HELP).map(([k, h]) => ({ label: h.t, run: () => helpCard(k) })))} /><B big i="keys" l="Shortcuts" run={() => D.shortcuts()} /><B big i="search" l="Tell Me" run={palette} /></G>
      <G l="Datum Sim on GitHub"><B big i="github" l="GitHub" run={() => call("open_url", { url: GH })} /><B big i="tag" l="Releases" run={() => call("open_url", { url: `${GH}/releases` })} /><B big i="bug" l="Report an Issue" run={() => call("open_url", { url: `${GH}/issues/new` })} /></G>
      <G l="Updates"><B big i="upd" l="Check for Updates" run={() => setBackstage("about")} /><B big i="list" l="Roadmap" run={() => setBackstage("roadmap")} /></G>
    </Show>
  </section>
);
