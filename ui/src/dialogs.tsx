// Dialogs and pages: rename, variables, sweep, analyses, step specifications, issues, help pages.
import { For, Show, createSignal, createResource, JSX } from "solid-js";
import uPlot from "uplot";
import { num, Var, Out, Mask, MaskParam, BlockView } from "./api";
import { openDialog, closeDialog, act, call, out, setOut, toast, issues, results, chans, setLiveTick, setResults, refreshMyBlocks } from "./store";
import { invoke } from "./api";
import { C } from "./cmds";
import { Dialog, Icon } from "./ui";
import { Help } from "./help";
import { OKABE } from "./scope";

export const rename = (id: number, name: string) => openDialog(() => {
  const [v, setV] = createSignal(name);
  return <Dialog title="Rename block" ok={async () => (await act({ Rename: { id, name: v() } })) ? undefined : false}>
    <div class="form"><label>Name</label><input value={v()} onInput={e => setV(e.currentTarget.value)} /></div>
  </Dialog>;
});

/** The mask editor: a subsystem's own parameters (names the blocks inside use), the text on the
 *  block, its help, and typical ratings saved from the current values. */
export const maskEditor = (b: BlockView) => openDialog(() => {
  const m0: Mask = b.mask ?? { params: [], icon: "", help: "", presets: [] };
  const [rows, setRows] = createSignal<(MaskParam & { ch: string })[]>(m0.params.map(p => ({ ...p, ch: p.choices.join(", ") })));
  const [icon, setIcon] = createSignal(m0.icon), [help, setHelp] = createSignal(m0.help);
  const [presets, setPresets] = createSignal(m0.presets);
  const put = (i: number, k: keyof MaskParam | "ch", v: string) => setRows(r => r.map((x, j) => (j === i ? { ...x, [k]: v } : x)));
  const mask = (): Mask => ({
    params: rows().map(r => ({ key: r.key.trim(), label: r.label.trim(), def: r.def.trim(), unit: r.unit.trim(), hint: r.hint.trim(), choices: r.ch.split(",").map(x => x.trim()).filter(Boolean) })),
    icon: icon(), help: help(), presets: presets(),
  });
  const [rname, setRname] = createSignal("");
  const saveRating = () => {
    const name = rname().trim(); if (!name) return toast("Give the rating a name, for example 400 V · 11 kW.");
    setRname("");
    const vals: [string, string][] = rows().filter(r => r.key.trim()).map(r => [r.key.trim(), b.params[r.key.trim()] ?? r.def]);
    setPresets(p => [...p.filter(x => x.name !== name), { name, values: vals }]);
  };
  return <Dialog title={`Mask: ${b.name}`} wide okText="Save mask" ok={async () => (await act({ SetMask: { id: b.id, mask: mask() } })) ? undefined : false}
    extra={<Show when={b.mask}><button class="btn" onClick={() => { closeDialog(); act({ SetMask: { id: b.id, mask: null } }); }}>Remove the mask</button></Show>}>
    <p class="muted">A mask gives this subsystem parameters of its own. The blocks inside use them by name (for example a Gain of <span class="mono">R*2</span>); set their values on the block like any other block's.</p>
    <table class="tbl maskt"><thead><tr><th>Name</th><th>Label</th><th>Default</th><th>Unit</th><th>Choices (comma-separated)</th><th /></tr></thead><tbody>
      <For each={rows()}>{(r, i) => <tr>
        <td><input class="mono mk-key" value={r.key} placeholder="R" onInput={e => put(i(), "key", e.currentTarget.value)} /></td>
        <td><input value={r.label} placeholder="Resistance R" onInput={e => put(i(), "label", e.currentTarget.value)} /></td>
        <td><input class="mono" value={r.def} placeholder="1" onInput={e => put(i(), "def", e.currentTarget.value)} /></td>
        <td><input value={r.unit} placeholder="Ω" onInput={e => put(i(), "unit", e.currentTarget.value)} /></td>
        <td><input value={r.ch} placeholder="(a number)" onInput={e => put(i(), "ch", e.currentTarget.value)} /></td>
        <td><button class="x" title="Remove" onClick={() => setRows(x => x.filter((_, j) => j !== i()))}>×</button></td></tr>}</For>
    </tbody></table>
    <p><button class="btn sm mk-add" onClick={() => setRows(r => [...r, { key: "", label: "", def: "1", unit: "", hint: "", choices: [], ch: "" }])}>Add a parameter</button></p>
    <div class="form"><label>Text on the block</label><input class="mk-icon" value={icon()} placeholder={b.name} onInput={e => setIcon(e.currentTarget.value)} />
      <label>Help</label><textarea rows="3" value={help()} placeholder="What this block is and how to use it (its ? card)." onInput={e => setHelp(e.currentTarget.value)} /></div>
    <h4>Typical ratings</h4>
    <Show when={presets().length} fallback={<p class="muted">None yet. Set the block's values, then save them here as a rating.</p>}>
      <ul class="plain"><For each={presets()}>{p => <li>{p.name} <button class="x" onClick={() => setPresets(x => x.filter(y => y.name !== p.name))}>×</button></li>}</For></ul>
    </Show>
    <p class="inline"><input class="mk-rname" value={rname()} placeholder="Rating name, e.g. 400 V · 11 kW" onInput={e => setRname(e.currentTarget.value)} /> <button class="btn sm" onClick={saveRating}>Save the block's current values as this rating</button></p>
  </Dialog>;
});

/** A masked block's ? card: its own help and parameters. */
export const maskHelp = (b: BlockView) => openDialog(() => (
  <Dialog title={b.name}>
    <div class="helpcard"><p>{b.mask?.help || "A masked subsystem."}</p>
      <Show when={b.mask?.params.length}><h4>Parameters</h4><ul><For each={b.mask!.params}>{p => <li><span class="mono">{p.key}</span>{p.label && p.label !== p.key ? ` · ${p.label}` : ""}{p.unit ? ` (${p.unit})` : ""}{p.hint ? `: ${p.hint}` : ""}</li>}</For></ul></Show>
      <p class="muted">Look under the mask (Inspector, or right-click) to see the blocks inside.</p></div>
  </Dialog>
));

/** Save a subsystem to My blocks under a name; it is copied into every model it is used in. */
export const saveMyBlock = (b: BlockView) => openDialog(() => {
  const [n, setN] = createSignal(b.name);
  return <Dialog title="Save to My blocks" okText="Save" ok={async () => {
    const p = await call<string>("save_myblock", { id: b.id, name: n() });
    if (!p) return false;
    toast(`Saved to My blocks: ${n()}`, true);
    refreshMyBlocks();
  }}>
    <p class="muted">The block appears in the Library under My blocks, for every model. A model gets its own copy each time you add it, so changing the saved block later does not change models that already use it.</p>
    <div class="form"><label>Name</label><input class="mb-name" value={n()} onInput={e => setN(e.currentTarget.value)} /></div>
  </Dialog>;
});

/** View › Variables: the model's named values. */
export const VarsPage = () => {
  const [rows, setRows] = createSignal<Var[]>(out()?.vars.map(v => ({ ...v })) ?? []);
  const save = () => act({ SetVars: { vars: rows().filter(r => r.name.trim()) } }).then(o => { if (o) toast("Variables saved.", true); });
  const put = (i: number, k: keyof Var, v: string) => setRows(rows().map((r, j) => (j === i ? { ...r, [k]: v } : r)));
  return (
    <div class="page">
      <h1>Model variables <Help k="vars" /></h1>
      <p class="muted">Block parameters can use these names: write <code>1/L</code> in a gain, or <code>Kp</code> in a PID. A variable may use the ones above it, e.g. <code>c = 2*zeta*sqrt(k*m)</code>.</p>
      <table class="tbl vars"><thead><tr><th>Name</th><th>Value or expression</th><th>Unit</th><th>Note</th><th /></tr></thead>
        <tbody><For each={rows()}>{(r, i) => <tr>
          <td><input class="mono" value={r.name} onChange={e => put(i(), "name", e.currentTarget.value)} /></td>
          <td><input class="mono" value={r.expr} onChange={e => put(i(), "expr", e.currentTarget.value)} /></td>
          <td><input value={r.unit} onChange={e => put(i(), "unit", e.currentTarget.value)} style={{ width: "90px" }} /></td>
          <td><input value={r.note} onChange={e => put(i(), "note", e.currentTarget.value)} /></td>
          <td><button class="x" title="Remove" onClick={() => setRows(rows().filter((_, j) => j !== i()))}>×</button></td></tr>}</For></tbody></table>
      <p><button class="btn" onClick={() => setRows([...rows(), { name: "", expr: "0", unit: "", note: "" }])}>+ Add variable</button> <button class="btn pri" onClick={save}>Save variables</button> <button class="btn" onClick={() => setRows(out()?.vars.map(v => ({ ...v })) ?? [])}>Undo changes</button></p>
    </div>
  );
};

/** View › Issues. */
export const IssuesPage = () => (
  <div class="page">
    <h1>Issues <Help k="issues" /></h1>
    <p><button class="btn" onClick={C.check}><Icon n="check" /> Check model again</button></p>
    <Show when={issues().length} fallback={<p class="muted">No issues. Run or check the model to look for problems.</p>}>
      <table class="tbl"><tbody><For each={issues()}>{i => <tr class="link" onClick={() => i.id && C.reveal(i.id)}><td style={{ width: "70px" }}><span class="pill" classList={{ warn: i.warn, err: !i.warn }}>{i.warn ? "Warning" : "Error"}</span></td><td>{i.msg}</td></tr>}</For></tbody></table>
    </Show>
  </div>
);

export const sweepDialog = () => openDialog(() => {
  const vars = out()?.vars ?? [];
  const [v, setV] = createSignal(vars[0]?.name ?? ""), [vals, setVals] = createSignal(""), [busy, setBusy] = createSignal(false);
  const run = async () => {
    setBusy(true);
    try {
      const n = await invoke<number>("run_sweep", { var: v(), values: vals() });
      const r = await invoke<any>("results");
      setResults(r); setLiveTick(t => t + 1);
      toast(`Ran ${n} simulations. The scopes show every run.`, true);
      C.view("scopes");
    } catch (e) { toast(String(e)); return false; } finally { setBusy(false); }
  };
  return <Dialog title="Parameter sweep" help="sweep" okText={busy() ? "Running…" : "Run sweep"} ok={run}>
    <Show when={vars.length} fallback={<p>Add a model variable first (Model › Variables), then use it in a block parameter.</p>}>
      <div class="form">
        <label>Variable</label><select value={v()} onChange={e => setV(e.currentTarget.value)}><For each={vars}>{x => <option value={x.name}>{x.name} = {x.expr} {x.unit}</option>}</For></select>
        <label>Values</label><input class="mono" placeholder="e.g. 0.1 0.5 1   or   linspace(0.1, 1, 5)" value={vals()} onInput={e => setVals(e.currentTarget.value)} />
      </div>
      <p class="muted">The model runs once for each value, in parallel, and the scopes overlay the runs.</p>
    </Show>
  </Dialog>;
});

/** Linear analysis of one block by the Python engine. */
export const analysis = (id: number, name: string, what: string) => openDialog(() => {
  const [res] = createResource(() => invoke<any>("analyze", { id, what }).catch(e => ({ error: String(e) })));
  let host!: HTMLDivElement;
  const draw = (r: any) => {
    if (!host || r.error) return;
    const ink = getComputedStyle(document.documentElement).getPropertyValue("--text2").trim() || "#3B4859";
    const axis = { stroke: ink, grid: { stroke: "#E6E8E5", width: 1 }, font: "11px 'IBM Plex Sans', sans-serif" };
    host.innerHTML = "";
    const w = Math.max(420, host.clientWidth);
    if (r.what === "bode") {
      const mk = (y: number[], label: string, unit: string) => new uPlot({ width: w, height: 190, scales: { x: { time: false, distr: 3 } }, legend: { show: false },
        axes: [{ ...axis, label: "Frequency (rad/s)", labelSize: 18 }, { ...axis, label: unit, labelSize: 18, size: 56 }],
        series: [{}, { label, stroke: OKABE[0], width: 1.6 }] }, [r.w, y], host);
      mk(r.mag_db, "Magnitude", "dB"); mk(r.phase_deg, "Phase", "degrees");
    } else if (r.what === "step") {
      new uPlot({ width: w, height: 260, scales: { x: { time: false } }, legend: { show: false },
        axes: [{ ...axis, label: "Time (s)", labelSize: 18 }, { ...axis, size: 56 }], series: [{}, { label: "Output", stroke: OKABE[0], width: 1.6 }] }, [r.t, r.y], host);
    } else if (r.what === "pz") {
      const xs = [...r.poles.map((p: number[]) => p[0]), ...r.zeros.map((p: number[]) => p[0]), 0], ys = [...r.poles.map((p: number[]) => p[1]), ...r.zeros.map((p: number[]) => p[1]), 0];
      const m = Math.max(1, ...xs.map(Math.abs), ...ys.map(Math.abs)) * 1.2, W = 420, H = 300, sx = (v: number) => W / 2 + v / m * (W / 2 - 20), sy = (v: number) => H / 2 - v / m * (H / 2 - 20);
      host.innerHTML = `<svg viewBox="0 0 ${W} ${H}" class="pzplot"><line x1="0" x2="${W}" y1="${H / 2}" y2="${H / 2}" class="pzaxis"/><line x1="${W / 2}" x2="${W / 2}" y1="0" y2="${H}" class="pzaxis"/>` +
        r.poles.map((p: number[]) => `<path d="M${sx(p[0]) - 5} ${sy(p[1]) - 5}l10 10m0 -10l-10 10" stroke="${OKABE[3]}" stroke-width="2"/>`).join("") +
        r.zeros.map((p: number[]) => `<circle cx="${sx(p[0])}" cy="${sy(p[1])}" r="5" fill="none" stroke="${OKABE[0]}" stroke-width="2"/>`).join("") +
        `<text x="${W - 4}" y="${H / 2 - 4}" text-anchor="end" class="xyt">Re ±${num(m)}</text><text x="${W / 2 + 4}" y="12" class="xyt">Im</text></svg>`;
    }
  };
  const title = what === "bode" ? "Bode plot" : what === "step" ? "Step response" : "Poles and zeros";
  return <Dialog title={`${title}: ${name}`} wide help="linear">
    <Show when={!res.loading} fallback={<p class="muted"><i class="spin" /> Working… The first analysis starts the Python engine, which takes a few seconds.</p>}>
      <Show when={!res()?.error} fallback={<p class="issue">{res()?.error}</p>}>
        <div class="anplot" ref={el => { host = el; queueMicrotask(() => draw(res())); }} />
        <table class="tbl facts"><tbody><For each={res()?.facts ?? []}>{(f: [string, string]) => <tr><td>{f[0]}</td><td class="r mono">{f[1]}</td></tr>}</For></tbody></table>
        <p class="muted small">Computed with python-control {res()?.control}. {res()?.note ?? ""}</p>
      </Show>
    </Show>
  </Dialog>;
});

/** Rise time, overshoot and settling time of a logged signal. */
export const stepInfoDialog = () => openDialog(() => {
  const [k, setK] = createSignal(0), [info, setInfo] = createSignal<any>(null), [busy, setBusy] = createSignal(false);
  const go = async () => { setBusy(true); try { setInfo(await invoke<any>("step_info", { chan: k() })); } catch (e) { setInfo({ error: String(e) }); } finally { setBusy(false); } };
  return <Dialog title="Step-response specifications" help="specs">
    <Show when={results()} fallback={<p>Run the model first.</p>}>
      <div class="form"><label>Signal</label><select value={k()} onChange={e => { setK(+e.currentTarget.value); setInfo(null); }}><For each={chans()}>{(c, i) => <option value={i()}>{c.name}</option>}</For></select></div>
      <p><button class="btn pri" disabled={busy()} onClick={go}>{busy() ? "Working…" : "Measure"}</button></p>
      <Show when={info()}><Show when={!info().error} fallback={<p class="issue">{info().error}</p>}>
        <table class="tbl facts"><tbody><For each={info().facts}>{(f: [string, string]) => <tr><td>{f[0]}</td><td class="r mono">{f[1]}</td></tr>}</For></tbody></table>
        <p class="muted small">Rise time from 10 % to 90 %; settling within 2 % of the final value. Measured from the start of the run, so put the step near t = 0 or read the times relative to it.</p>
      </Show></Show>
    </Show>
  </Dialog>;
});

export const plotHint = (path: string): JSX.Element => (
  <Dialog title="Results ready for Datum Plot">
    <p>The results were saved as <b>{path}</b>: time in the first column, then one column per signal, with names in the first row.</p>
    <p>In Datum Plot, open or import this CSV file to analyse the signals, fit curves and make publication graphs.</p>
    <p><button class="btn" onClick={() => call("open_url", { url: "https://github.com/Edgar-Mendonca/Datum-Plot/releases" })}>Get Datum Plot</button></p>
  </Dialog>
);

export const confirmClose = (quit: () => void) => openDialog(() => (
  <Dialog title="Save changes?" okText="Save" cancel="Cancel" ok={async () => { await C.save(); if (!out()?.dirty) quit(); }}
    extra={<button class="btn" onClick={() => { closeDialog(); quit(); }}>Don't save</button>}>
    <p>“{out()?.name}” has changes that are not saved.</p>
  </Dialog>
));

export const shortcuts = () => openDialog(() => (
  <Dialog title="Keyboard shortcuts">
    <table class="tbl keys"><tbody><For each={[
      ["Run", "F5"], ["Stop", "Shift+F5"], ["Save / Save As", "Ctrl+S / Ctrl+Shift+S"], ["Open", "Ctrl+O"], ["New", "Ctrl+N"], ["Print", "Ctrl+P"], ["Tell me what you want to do", "Ctrl+K"],
      ["Undo / Redo", "Ctrl+Z / Ctrl+Y"], ["Copy / Cut / Paste", "Ctrl+C / Ctrl+X / Ctrl+V"], ["Delete", "Del"], ["Select all", "Ctrl+A"], ["Rename block", "F2"], ["Flip block", "Ctrl+R"],
      ["Create subsystem", "Ctrl+G"], ["Move selection", "Arrows (Shift: 1 px)"], ["Add a block", "Double-click empty space"], ["Zoom", "Ctrl + wheel"], ["Pan", "Wheel, or Space + drag"], ["This list", "F1"],
    ]}>{([a, b]) => <tr><td>{a}</td><td><kbd>{b}</kbd></td></tr>}</For></tbody></table>
  </Dialog>
));

export const gettingStarted = () => openDialog(() => (
  <Dialog title="Getting started" wide>
    <div class="helpcard">
      <p>Datum Sim simulates systems that change over time. You draw the system as blocks joined by wires, press Run, and watch the signals in scopes.</p>
      <h4>Your first model in a minute</h4>
      <ol>
        <li>Drag a <b>Step</b> (Sources), a <b>Transfer function</b> (Continuous) and a <b>Scope</b> (Sinks) from the Library onto the diagram.</li>
        <li>Drag from the small triangle on the right of the Step to the left of the Transfer function, and from the Transfer function to the Scope.</li>
        <li>Click the Transfer function and set its denominator to <code>[1 2 1]</code> in the Inspector on the right.</li>
        <li>Press <b>Run</b> (F5). The scope fills with the response.</li>
      </ol>
      <h4>Good to know</h4>
      <ul>
        <li>Signals flow from outputs (right side of a block) to inputs (left side). Flip a block (Ctrl+R) for feedback paths.</li>
        <li>Put named values in Model › Variables and use the names in parameters: one change updates every block.</li>
        <li>Select blocks and press Ctrl+G to put them in a subsystem; double-click it to go inside.</li>
        <li>Every block has a ? card with its equations. File › New has worked examples: motors, a tank, a car, a thermostat.</li>
        <li>Variable-step solving is the default and right for most models; use Stiff for models with very fast and very slow parts.</li>
      </ul>
    </div>
  </Dialog>
));

export const glossary = () => openDialog(() => {
  const [q, setQ] = createSignal("");
  const G: [string, string][] = [
    ["Block", "One piece of the model: a source, a calculation, a dynamic element or a sink."], ["Signal", "A value that changes over time and flows along a wire."],
    ["State", "A value a block remembers between steps, such as an integrator's output. States are what the solver steps forward."],
    ["Solver", "The method that steps the states through time."], ["Step size", "How far in time one solver step goes. Variable-step solvers choose it to meet the tolerance."],
    ["Tolerance", "How much error each step may make. Relative tolerance 1e-6 means about six correct digits."],
    ["Stiff", "A model with both very fast and very slow behaviour; explicit solvers then need tiny steps. Use the Stiff solver."],
    ["Algebraic loop", "A loop of wires with no block that has a state in it, so an output depends on itself at the same moment. Break it with an Integrator, a Delay or a Unit delay."],
    ["Direct feedthrough", "A block whose output depends on its input at the same moment (a Gain does; an Integrator does not)."],
    ["Zero-crossing", "The moment a switching block (Relay, Integrator limit) changes state. The solver finds it exactly."],
    ["Transfer function", "A linear system written as a ratio of polynomials in s, the Laplace variable."], ["Subsystem", "A group of blocks shown as one block, with its own diagram."],
    ["Sweep", "Running the model several times, once for each value of a variable."], ["Bode plot", "How much a linear system amplifies and delays sine waves of each frequency."],
    ["Overshoot", "How far a step response goes past its final value, in percent."], ["Settling time", "When a response stays within 2 % of its final value."],
  ];
  return <Dialog title="Glossary"><input placeholder="Search" value={q()} onInput={e => setQ(e.currentTarget.value)} style={{ width: "100%", "margin-bottom": "8px" }} />
    <dl class="gloss"><For each={G.filter(g => (g[0] + g[1]).toLowerCase().includes(q().toLowerCase()))}>{([t, d]) => <><dt>{t}</dt><dd>{d}</dd></>}</For></dl></Dialog>;
});

/** Recover unsaved work after a crash. */
export const recoverDialog = (name: string, saved: string, done: () => void) => openDialog(() => (
  <Dialog title="Recover unsaved work?" okText="Recover" cancel="Discard" ok={async () => { const r = await call<Out>("recover", { keep: true }); if (r) { setOut(r); done(); } }}>
    <p>Datum Sim closed before “{name}” was saved. A copy from {saved} was kept.</p>
  </Dialog>
));

