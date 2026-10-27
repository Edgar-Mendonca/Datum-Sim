// The Library panel (blocks to drag in) and the Inspector (the selected block's parameters, or the
// model's settings when nothing is selected).
import { For, Show, createSignal, createMemo, createEffect, on } from "solid-js";
import { Def, BlockView, num } from "./api";
import { lib, selBlocks, selWires, act, out, defOf, issues, results, chans, setView, sel, enter } from "./store";
import { C } from "./cmds";
import { Help, helpCard } from "./help";
import { Icon } from "./ui";

/** A small picture of each block kind for the library. */
const LIB_ICON: Record<string, string> = { Scope: "scope", XY: "scope", Subsystem: "sub", Inport: "wire", Outport: "wire", DCMotor: "motor", PMSM: "motor", ToFile: "csv", FromFile: "csv", ToPlot: "plot", PyBlock: "py" };

export const LibraryPanel = () => {
  const [q, setQ] = createSignal("");
  const [shut, setShut] = createSignal<string[]>([]);
  const groups = createMemo(() => {
    const s = q().toLowerCase();
    return lib().cats.map(c => [c, lib().blocks.filter(d => d.cat === c && (!s || d.name.toLowerCase().includes(s) || d.what.toLowerCase().includes(s) || d.kind.toLowerCase().includes(s)))] as [string, Def[]]).filter(g => g[1].length);
  });
  return (
    <aside class="lib">
      <header><b>Library</b><Help k="library" /></header>
      <input class="libq" placeholder="Search blocks" value={q()} onInput={e => setQ(e.currentTarget.value)} />
      <div class="libl">
        <For each={groups()}>{([cat, defs]) => (
          <section>
            <button class="libc" onClick={() => setShut(shut().includes(cat) ? shut().filter(x => x !== cat) : [...shut(), cat])}><b class="car">{shut().includes(cat) && !q() ? "▸" : "▾"}</b>{cat}</button>
            <Show when={!shut().includes(cat) || q()}>
              <For each={defs}>{d => (
                <div class="libi" classList={{ off: !!d.planned }} draggable={!d.planned} title={d.planned ? `Planned for ${d.planned}: ${d.what}` : d.what}
                  onDragStart={e => { e.dataTransfer?.setData("text/datum-block", d.kind); e.dataTransfer!.effectAllowed = "copy"; }}
                  onDblClick={() => !d.planned && C.addBlock(d.kind)}>
                  <Icon n={LIB_ICON[d.kind] ?? "block"} /><span>{d.name}</span>
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

/** One parameter: an expression box (or a choice), with its unit and hint. */
const Param = (p: { b: BlockView; def: Def; k: number }) => {
  const pd = () => p.def.params[p.k];
  const set = (value: string) => { if (value !== p.b.params[pd().key]) act({ SetParam: { id: p.b.id, key: pd().key, value } }); };
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
  const def = () => (b() ? defOf(b()!.kind) : undefined);
  const [name, setName] = createSignal("");
  createEffect(on(b, x => setName(x?.name ?? "")));
  const blockIssues = () => issues().filter(i => i.id === b()?.id);
  const display = () => {
    const r = results(); if (!r || !b()) return null;
    const ks = chans().map((c, i) => [c, i] as const).filter(([c]) => c.sink === b()!.id);
    return ks.length ? ks.map(([c, i]) => `${c.name} = ${num(r.last[i])}`) : null;
  };
  return (
    <aside class="insp">
      <Show when={b() && def()} fallback={<ModelPanel />}>
        <header><b>{def()!.name}</b><button class="help" title={`About ${def()!.name}`} onClick={() => helpCard(`block:${b()!.kind}`)}>?</button></header>
        <div class="ibody">
          <div class="iprow"><label>Name</label><input value={name()} onChange={e => act({ Rename: { id: b()!.id, name: e.currentTarget.value } }).then(o => { if (!o) setName(b()!.name); })} /></div>
          <p class="muted iwhat">{def()!.what}</p>
          <For each={def()!.params}>{(_, k) => <Param b={b()!} def={def()!} k={k()} />}</For>
          <Show when={b()!.sub}><p><button class="btn sm" onClick={() => enter(b()!.id)}><Icon n="sub" /> Open subsystem</button> <button class="btn sm" onClick={() => C.ungroup(b()!.id)}>Ungroup</button></p></Show>
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
        <Show when={selWires().length}><p class="muted">A wire carries a signal from an output to an input. Drag it to add a bend point; press Delete to remove it.</p>
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
