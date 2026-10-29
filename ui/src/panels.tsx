// The Library panel (blocks to drag in) and the Inspector (the selected block's parameters, or the
// model's settings when nothing is selected).
import { For, Show, createSignal, createMemo, createEffect, on, onMount } from "solid-js";
import { Def, BlockView, Issue, num, invoke, MyBlock } from "./api";
import { lib, selBlocks, selWires, act, out, defOf, issues, results, chans, setView, sel, enter, setIssues, myBlocks, refreshMyBlocks, openMenu, toast, call } from "./store";
import * as D from "./dialogs";
import { C } from "./cmds";
import { dropBlock } from "./canvas";
import { Help, helpCard } from "./help";
import { Icon } from "./ui";

/** A small picture of each block kind for the library. */
const LIB_ICON: Record<string, string> = { Scope: "scope", XY: "scope", Subsystem: "sub", Inport: "wire", Outport: "wire", DCMotor: "motor", PMSM: "motor", ToFile: "csv", FromFile: "csv", ToPlot: "plot", PyBlock: "py" };

/** Dragging a block from the library: plain mouse events (HTML drag and drop does not reach the
 *  diagram in every WebView). The drag starts after 4 px, so click and double-click still work. */
const [ghost, setGhost] = createSignal<{ d: Def; x: number; y: number; over: boolean } | null>(null);
const startDrag = (e: MouseEvent, d: Def) => {
  e.preventDefault();
  const x0 = e.clientX, y0 = e.clientY;
  let on = false;
  const overCanvas = (x: number, y: number) => !!document.elementFromPoint(x, y)?.closest(".canvas");
  const mv = (m: MouseEvent) => {
    if (!on && Math.hypot(m.clientX - x0, m.clientY - y0) < 4) return;
    on = true;
    setGhost({ d, x: m.clientX, y: m.clientY, over: overCanvas(m.clientX, m.clientY) });
  };
  const up = (m: MouseEvent) => {
    removeEventListener("mousemove", mv); removeEventListener("mouseup", up);
    setGhost(null);
    if (on && overCanvas(m.clientX, m.clientY)) dropBlock(d.kind, m.clientX, m.clientY);
  };
  addEventListener("mousemove", mv); addEventListener("mouseup", up);
};
export const LibGhost = () => (
  <Show when={ghost()}>{g => <div class="libghost" classList={{ over: g().over }} style={{ left: `${g().x - g().d.w / 2}px`, top: `${g().y - g().d.h / 2}px`, width: `${g().d.w}px`, height: `${g().d.h}px` }}><span>{g().d.name}</span></div>}</Show>
);

/** A masked subsystem read as a block definition, so the Inspector shows it like any block. */
export const maskDef = (b: BlockView): Def => ({
  kind: "Subsystem", name: b.name, cat: "", what: b.mask?.help || "A masked subsystem.", how: "", w: b.w, h: b.h, planned: "",
  params: (b.mask?.params ?? []).map(p => ({ key: p.key, label: p.label || p.key, def: p.def, unit: p.unit, hint: p.hint, kind: p.choices.length ? "choice" : "num", choices: p.choices })),
  presets: b.mask?.presets ?? [],
});

/** My blocks are dragged like library blocks: their kind is "my:" and the file. */
const myDef = (m: MyBlock): Def => ({ kind: `my:${m.file}`, name: m.name, cat: "My blocks", what: m.what || "Your saved block.", how: "", params: [], w: 100, h: 70, planned: "", presets: [] });

export const LibraryPanel = () => {
  const [q, setQ] = createSignal("");
  const [shut, setShut] = createSignal<string[]>([]);
  onMount(refreshMyBlocks);
  const mine = createMemo(() => { const s = q().toLowerCase(); return myBlocks().filter(m => !s || m.name.toLowerCase().includes(s) || m.what.toLowerCase().includes(s)); });
  const mineMenu = (e: MouseEvent, m: MyBlock) => { e.preventDefault(); openMenu(e, [
    { label: "Add to the diagram", run: () => C.addBlock(`my:${m.file}`) },
    { label: "Remove from My blocks", run: async () => { await call("delete_myblock", { file: m.file }); refreshMyBlocks(); toast(`Removed ${m.name} from My blocks. Models that use it keep their copy.`, true); } },
  ]); };
  const groups = createMemo(() => {
    const s = q().toLowerCase();
    return lib().cats.map(c => [c, lib().blocks.filter(d => d.cat === c && (!s || d.name.toLowerCase().includes(s) || d.what.toLowerCase().includes(s) || d.kind.toLowerCase().includes(s)))] as [string, Def[]]).filter(g => g[1].length);
  });
  return (
    <aside class="lib">
      <header><b>Library</b><Help k="library" /></header>
      <input class="libq" placeholder="Search blocks" value={q()} onInput={e => setQ(e.currentTarget.value)} />
      <div class="libl">
        <Show when={mine().length}>
          <section class="mylib">
            <button class="libc" onClick={() => setShut(shut().includes("My blocks") ? shut().filter(x => x !== "My blocks") : [...shut(), "My blocks"])}><b class="car">{shut().includes("My blocks") && !q() ? "▸" : "▾"}</b>My blocks</button>
            <Show when={!shut().includes("My blocks") || q()}>
              <For each={mine()}>{m => (
                <div class="libi mine" title={m.what || m.name} onMouseDown={e => e.button === 0 && startDrag(e, myDef(m))} onDblClick={() => C.addBlock(`my:${m.file}`)} onContextMenu={e => mineMenu(e, m)}>
                  <Icon n="sub" /><span>{m.name}</span>
                </div>
              )}</For>
            </Show>
          </section>
        </Show>
        <For each={groups()}>{([cat, defs]) => (
          <section>
            <button class="libc" onClick={() => setShut(shut().includes(cat) ? shut().filter(x => x !== cat) : [...shut(), cat])}><b class="car">{shut().includes(cat) && !q() ? "▸" : "▾"}</b>{cat}</button>
            <Show when={!shut().includes(cat) || q()}>
              <For each={defs}>{d => (
                <div class="libi" classList={{ off: !!d.planned }} title={d.planned ? `Planned for ${d.planned}: ${d.what}` : d.what}
                  onMouseDown={e => !d.planned && e.button === 0 && !(e.target as HTMLElement).closest(".help") && startDrag(e, d)}
                  onDblClick={() => !d.planned && C.addBlock(d.kind)}>
                  <Icon n={LIB_ICON[d.kind] ?? (d.cat === "Motors" ? "motor" : "block")} /><span>{d.name}</span>
                  <Show when={d.planned} fallback={<button class="help" title={`About ${d.name}`} onClick={e => { e.stopPropagation(); helpCard(`block:${d.kind}`); }}>?</button>}><small class="pill">{d.planned}</small></Show>
                </div>
              )}</For>
            </Show>
          </section>
        )}</For>
      </div>
    </aside>
  );
};

/** Typical ratings: one choice sets every parameter it lists, as one undoable edit. */
const Presets = (p: { b: BlockView; def: Def }) => {
  const [pick, setPick] = createSignal("");
  createEffect(on(() => p.b.id, () => setPick("")));
  const apply = (name: string) => {
    const pr = p.def.presets.find(x => x.name === name); if (!pr) return;
    setPick(name);
    act({ Batch: pr.values.map(([key, value]) => ({ SetParam: { id: p.b.id, key, value } })) })
      .then(o => o && invoke<Issue[]>("check").then(setIssues).catch(() => {}));
  };
  return (
    <div class="iprow">
      <label title="Typical values for a machine of this rating; replace them with nameplate or test data when you have it.">Typical ratings</label>
      <select class="presets" value={pick()} onChange={e => apply(e.currentTarget.value)}>
        <option value="">Choose a rating…</option>
        <For each={p.def.presets}>{x => <option value={x.name}>{x.name}</option>}</For>
      </select>
      <small class="hint">Typical values; replace them with nameplate data</small>
    </div>
  );
};

/** One parameter: an expression box (or a choice), with its unit and hint. */
const Param = (p: { b: BlockView; def: Def; k: number }) => {
  const pd = () => p.def.params[p.k];
  // Check the model quietly after each change, so a bad value shows here before Run.
  const set = (value: string) => { if (value !== p.b.params[pd().key]) act({ SetParam: { id: p.b.id, key: pd().key, value } }).then(o => o && invoke<Issue[]>("check").then(setIssues).catch(() => {})); };
  return (
    <div class="iprow">
      <label title={pd().hint}>{pd().label}{pd().unit && <small class="unit">{pd().unit}</small>}</label>
      <Show when={pd().kind === "choice"} fallback={
        <input value={p.b.params[pd().key] ?? ""} placeholder={pd().def} spellcheck={false} onChange={e => set(e.currentTarget.value)}
          onKeyDown={e => { if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur(); }} classList={{ mono: pd().kind === "num" }} />}>
        <select value={p.b.params[pd().key]} onChange={e => set(e.currentTarget.value)}><For each={pd().choices}>{c => <option value={c}>{c}</option>}</For></select>
      </Show>
      <Show when={pd().hint}><small class="hint">{pd().hint}</small></Show>
    </div>
  );
};

export const Inspector = () => {
  const b = () => selBlocks()[0];
  const def = () => (b() ? (b()!.mask ? maskDef(b()!) : defOf(b()!.kind)) : undefined);
  const [name, setName] = createSignal("");
  createEffect(on(b, x => setName(x?.name ?? "")));
  const blockIssues = () => issues().filter(i => i.id === b()?.id);
  const display = () => {
    const r = results(); if (!r || !b()) return null;
    const ks = chans().map((c, i) => [c, i] as const).filter(([c]) => !c.probe && c.sink === b()!.id);
    return ks.length ? ks.map(([c, i]) => `${c.name} = ${num(r.last[i])}`) : null;
  };
  return (
    <aside class="insp">
      <Show when={b() && def()} fallback={<ModelPanel />}>
        <header><b>{b()!.mask ? `${def()!.name} (masked)` : def()!.name}</b><button class="help" title={`About ${def()!.name}`} onClick={() => (b()!.mask ? D.maskHelp(b()!) : helpCard(`block:${b()!.kind}`))}>?</button></header>
        <div class="ibody">
          <div class="iprow"><label>Name</label><input value={name()} onChange={e => act({ Rename: { id: b()!.id, name: e.currentTarget.value } }).then(o => { if (!o) setName(b()!.name); })} /></div>
          <p class="muted iwhat">{def()!.what}</p>
          <Show when={def()!.presets?.length}><Presets b={b()!} def={def()!} /></Show>
          <For each={def()!.params}>{(_, k) => <Param b={b()!} def={def()!} k={k()} />}</For>
          <Show when={b()!.sub}>
            <p class="ibtns"><button class="btn sm" onClick={() => enter(b()!.id)}><Icon n="sub" /> {b()!.mask ? "Look under the mask" : "Open subsystem"}</button><Show when={!b()!.mask}><button class="btn sm" onClick={() => C.ungroup(b()!.id)}>Ungroup</button></Show></p>
            <p class="ibtns"><button class="btn sm maskbtn" onClick={() => D.maskEditor(b()!)}>{b()!.mask ? "Edit the mask…" : "Add a mask…"}</button><button class="btn sm mybtn" onClick={() => D.saveMyBlock(b()!)}>Save to My blocks…</button></p>
          </Show>
          <Show when={["TransferFcn", "ZPK", "StateSpace", "PID", "DCMotor", "Gain", "Integrator"].includes(b()!.kind)}>
            <h4>Analyze</h4><p class="ibtns"><button class="btn sm" onClick={() => C.analyze("bode")}><Icon n="bode" /> Bode</button><button class="btn sm" onClick={() => C.analyze("step")}><Icon n="specs" /> Step</button><button class="btn sm" onClick={() => C.analyze("pz")}>Poles &amp; zeros</button></p>
          </Show>
          <Show when={display()}><h4>Last run</h4><For each={display()!}>{l => <p class="mono small">{l}</p>}</For></Show>
          <Show when={blockIssues().length}><h4>Issues</h4><For each={blockIssues()}>{i => <p class="issue" classList={{ warn: i.warn }}>{i.msg}</p>}</For></Show>
          <Show when={sel().length > 1}><p class="muted">{sel().length} items selected; the first block is shown.</p></Show>
        </div>
      </Show>
    </aside>
  );
};

/** Nothing selected: the run settings and the model's variables. */
const ModelPanel = () => {
  const s = () => out()?.sim;
  const set = (k: string, v: string) => { const x = +v; if (v.trim() !== "" && isFinite(x)) C.setSim(k, x); };
  return (
    <>
      <header><b>{selWires().length ? "Wire" : "Model"}</b><Help k="inspector" /></header>
      <div class="ibody">
        <Show when={selWires().length}><p class="muted">A wire carries a signal from an output to an input. Drag a segment to move it; double-click a bend to remove it; Alt-drag from the wire to branch the signal to another input; press Delete to remove it.</p>
          <div class="iprow"><label>Signal name</label><input class="wnameinsp" value={selWires()[0].name ?? ""} placeholder="(none)" onChange={e => act({ NameWire: { id: selWires()[0].id, name: e.currentTarget.value } })} onKeyDown={e => { if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur(); }} /><small class="hint">Shown on the wire and in scope legends</small></div>
          <p><button class="btn sm" onClick={() => act({ SetPts: { id: selWires()[0].id, pts: [] } })}>Straighten (remove bend points)</button></p></Show>
        <Show when={s()}>
          <h4>Run <Help k="run" /></h4>
          <div class="iprow"><label>Start time<small class="unit">s</small></label><input class="mono" value={s()!.start} onChange={e => set("start", e.currentTarget.value)} /></div>
          <div class="iprow"><label>Stop time<small class="unit">s</small></label><input class="mono" value={s()!.stop} onChange={e => set("stop", e.currentTarget.value)} /></div>
          <div class="iprow"><label>Solver</label><SolverPick /></div>
          <Show when={["rk4", "heun", "euler"].includes(s()!.solver)} fallback={<>
            <div class="iprow"><label>Relative tolerance</label><input class="mono" value={s()!.rtol} onChange={e => set("rtol", e.currentTarget.value)} /></div>
            <div class="iprow"><label>Absolute tolerance</label><input class="mono" value={s()!.atol} onChange={e => set("atol", e.currentTarget.value)} /></div>
            <div class="iprow"><label>Largest step<small class="unit">s</small></label><input class="mono" value={s()!.max_step || ""} placeholder="automatic" onChange={e => set("max_step", e.currentTarget.value || "0")} /></div>
          </>}>
            <div class="iprow"><label>Step<small class="unit">s</small></label><input class="mono" value={s()!.step} onChange={e => set("step", e.currentTarget.value)} /></div>
          </Show>
          <h4>Variables <Help k="vars" /></h4>
          <Show when={out()!.vars.length} fallback={<p class="muted">No model variables yet.</p>}>
            <table class="vmini"><tbody><For each={out()!.vars}>{v => <tr><td class="mono">{v.name}</td><td class="mono">{v.expr}</td><td class="muted">{v.unit}</td></tr>}</For></tbody></table>
          </Show>
          <p><button class="btn sm" onClick={() => setView("vars")}><Icon n="var" /> Edit variables</button></p>
          <Show when={out()!.notes}><h4>About this model</h4><p class="muted">{out()!.notes}</p></Show>
        </Show>
      </div>
    </>
  );
};

export const SOLVERS: [string, string][] = [["rk45", "Variable step (Dormand–Prince 4/5)"], ["stiff", "Stiff (Rosenbrock 2/3)"], ["rk4", "Fixed step: Runge–Kutta 4"], ["heun", "Fixed step: Heun"], ["euler", "Fixed step: Euler"]];
export const SolverPick = () => (
  <select value={out()?.sim.solver} onChange={e => C.setSim("solver", e.currentTarget.value)}><For each={SOLVERS}>{([k, l]) => <option value={k}>{l}</option>}</For></select>
);
