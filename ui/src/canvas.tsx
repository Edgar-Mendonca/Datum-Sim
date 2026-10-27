// The block diagram editor: draw blocks and wires, select, move, connect, pan and zoom.
// Geometry (ports, routed wires) comes from the engine; the window only draws and sends edits.
import { For, Show, JSX, createSignal, createMemo, onMount, onCleanup, batch } from "solid-js";
import { BlockView, WireView, Pt, num } from "./api";
import { dview, sel, setSel, act, parent, enter, out, opts, issues, results, chans, openMenu, defOf, lib, openDialog, closeDialog, setScopes, scopes, Item, running, setView, setBackstage } from "./store";
import { C } from "./cmds";
import { helpCard } from "./help";

const GRID = 10;
export const [zoom, setZoom] = createSignal(1);
export const [pan, setPan] = createSignal<Pt>([40, 40]);
let svgEl: SVGSVGElement | undefined;

const snap = (v: number) => (opts().snap ? Math.round(v / GRID) * GRID : Math.round(v));
const toDiagram = (cx: number, cy: number): Pt => {
  const r = svgEl!.getBoundingClientRect();
  return [(cx - r.left - pan()[0]) / zoom(), (cy - r.top - pan()[1]) / zoom()];
};
export const viewCenter = (): Pt => {
  if (!svgEl) return [200, 150];
  const r = svgEl.getBoundingClientRect();
  return toDiagram(r.left + r.width / 2, r.top + r.height / 2);
};
export const fit = () => {
  const v = dview(); if (!v || !svgEl || !v.blocks.length) { setPan([40, 40]); setZoom(1); return; }
  const r = svgEl.getBoundingClientRect();
  const x0 = Math.min(...v.blocks.map(b => b.x)) - 40, y0 = Math.min(...v.blocks.map(b => b.y)) - 40;
  const x1 = Math.max(...v.blocks.map(b => b.x + b.w)) + 40, y1 = Math.max(...v.blocks.map(b => b.y + b.h)) + 50;
  const z = Math.min(2, Math.max(0.2, Math.min(r.width / (x1 - x0), r.height / (y1 - y0))));
  batch(() => { setZoom(z); setPan([(r.width - (x1 - x0) * z) / 2 - x0 * z, (r.height - (y1 - y0) * z) / 2 - y0 * z]); });
};
export const zoomBy = (f: number, at?: Pt) => {
  const r = svgEl?.getBoundingClientRect(); if (!r) return;
  const [cx, cy] = at ?? [r.width / 2, r.height / 2];
  const z0 = zoom(), z = Math.min(3, Math.max(0.15, z0 * f));
  setPan([cx - (cx - pan()[0]) * z / z0, cy - (cy - pan()[1]) * z / z0]);
  setZoom(z);
};

// ---------- block pictures ----------
const T = (p: { x: number; y: number; children: JSX.Element; size?: number; anchor?: "start" | "middle" | "end"; bold?: boolean }) =>
  <text x={p.x} y={p.y} font-size={String(p.size ?? 11)} text-anchor={p.anchor ?? "middle"} dominant-baseline="central" class="btxt" font-weight={p.bold ? "600" : "400"}>{p.children}</text>;
const clip = (s: string, n: number) => (s.length > n ? s.slice(0, n - 1) + "…" : s);

const glyph = (kind: string, w: number, h: number): JSX.Element | null => {
  const cx = w / 2, cy = h / 2, gw = Math.min(w - 16, 34), gh = Math.min(h - 14, 20), x0 = cx - gw / 2, x1 = cx + gw / 2, y0 = cy + gh / 2, y1 = cy - gh / 2;
  const P = (d: string) => <path d={d} class="bglyph" />;
  switch (kind) {
    case "Step": return P(`M${x0} ${y0}H${cx}V${y1}H${x1}`);
    case "Ramp": return P(`M${x0} ${y0}H${cx - gw / 4}L${x1} ${y1}`);
    case "Sine": return P(`M${x0} ${cy}C${x0 + gw / 6} ${y1 - 4} ${cx - gw / 6} ${y1 - 4} ${cx} ${cy}S${x1 - gw / 6} ${y0 + 4} ${x1} ${cy}`);
    case "Pulse": return P(`M${x0} ${y0}H${x0 + gw * .2}V${y1}H${x0 + gw * .45}V${y0}H${x0 + gw * .7}V${y1}H${x1}`);
    case "Clock": return <><circle cx={cx} cy={cy} r={Math.min(gw, gh) / 2 + 2} class="bglyph" />{P(`M${cx} ${cy}V${cy - 6}M${cx} ${cy}H${cx + 5}`)}</>;
    case "Noise": return P(`M${x0} ${cy}l3 -6l3 9l3 -12l3 14l3 -8l3 5l3 -9l3 11l3 -4l3 2`);
    case "Saturation": return P(`M${x0} ${y0}H${cx - gw / 5}L${cx + gw / 5} ${y1}H${x1}`);
    case "DeadZone": return P(`M${x0} ${y0}L${cx - gw / 5} ${cy}H${cx + gw / 5}L${x1} ${y1}`);
    case "Relay": return P(`M${x0} ${y0}H${cx + 4}V${y1}H${x1}M${x1 - 6} ${y1}H${cx - 4}V${y0}`);
    case "RateLimiter": return P(`M${x0} ${y0}L${x1} ${y1}M${x0} ${y0}H${x1}`);
    case "Backlash": return P(`M${x0} ${y0}L${cx - 4} ${cy}M${cx - 4} ${cy - 5}V${cy + 5}M${cx + 4} ${cy - 5}V${cy + 5}M${cx + 4} ${cy}L${x1} ${y1}`);
    case "Quantizer": return P(`M${x0} ${y0}h7v-5h7v-5h7v-5h7v-5h6`);
    case "Abs": return P(`M${x0} ${y1}L${cx} ${y0}L${x1} ${y1}`);
    case "Sign": return P(`M${x0} ${y0}H${cx}V${y1}H${x1}`);
    case "Scope": case "XY": return <><rect x={6} y={6} width={w - 12} height={h - 18} rx={2} class="bscreen" />{kind === "Scope" ? <path d={`M10 ${h / 2 - 2}c4 0 4-10 8-10s4 14 8 14 4-6 ${w - 34} -6`} class="bwave" /> : <circle cx={w / 2} cy={h / 2 - 6} r={7} class="bwave" />}</>;
    case "DCMotor": case "PMSM": return <><circle cx={cx} cy={cy} r={Math.min(w, h) / 4} class="bglyph" /><T x={cx} y={cy} bold>{kind === "DCMotor" ? "M" : "M3~"}</T></>;
    case "Lookup1D": case "Table": return <>{P(`M${x0} ${y0}V${y1}M${x0} ${y0}H${x1}`)}{P(`M${x0 + 3} ${y0 - 3}L${cx - 4} ${cy}L${cx + 4} ${cy + 2}L${x1 - 2} ${y1 + 2}`)}</>;
    case "Lookup2D": return <>{P(`M${x0} ${y1}h${gw}v${gh}h${-gw}zM${x0} ${cy}h${gw}M${cx} ${y1}v${gh}`)}</>;
    case "FromFile": case "ToFile": case "ToPlot": return P(`M${cx - 8} ${cy - 10}h11l5 5v15h-16zM${cx + 3} ${cy - 10}v5h5`);
    default: return null;
  }
};

const BlockArt = (p: { b: BlockView }) => {
  const b = () => p.b, w = () => p.b.w, h = () => p.b.h;
  const shape = (): JSX.Element => {
    const k = b().kind;
    if (k === "Sum") return <circle cx={w() / 2} cy={h() / 2} r={Math.min(w(), h()) / 2} class="bshape" />;
    if (k === "Gain") return b().flip ? <path d={`M${w()} 0V${h()}L0 ${h() / 2}z`} class="bshape" /> : <path d={`M0 0V${h()}L${w()} ${h() / 2}z`} class="bshape" />;
    if (k === "Mux" || k === "Demux") return <rect width={w()} height={h()} class="bbar" />;
    if (k === "Inport" || k === "Outport") return <rect width={w()} height={h()} rx={h() / 2} class="bshape bport" />;
    return <rect width={w()} height={h()} rx={3} class="bshape" classList={{ bsub: b().sub }} />;
  };
  const inner = (): JSX.Element => {
    const k = b().kind, cx = w() / 2, cy = h() / 2;
    if (k === "Sum") return <T x={cx} y={cy} size={14} bold>Σ</T>;
    if (k === "Gain") return <T x={b().flip ? cx + w() / 6 : cx - w() / 6} y={cy} size={10}>{clip(b().label, 6)}</T>;
    if (k === "Product") return <T x={cx} y={cy} size={14} bold>{b().label.includes("/") ? "÷" : "×"}</T>;
    if (k === "Divide") return <T x={cx} y={cy} size={14} bold>÷</T>;
    if (k === "Mux" || k === "Demux") return null;
    if (k === "TransferFcn" || k === "ZPK") {
      const [n, d] = b().label.split("\n");
      return <><T x={cx} y={cy - 8} size={10}>{clip(n, 14)}</T><line x1={8} x2={w() - 8} y1={cy} y2={cy} class="bfrac" /><T x={cx} y={cy + 8} size={10}>{clip(d, 14)}</T></>;
    }
    if (k === "Display") {
      const v = displayValue(b().id);
      return <T x={cx} y={cy} size={12} bold>{v}</T>;
    }
    if (k === "Subsystem") return <T x={cx} y={cy} size={10} bold>{clip(b().name, 16)}</T>;
    if (k === "PID") return <T x={cx} y={cy} size={12} bold>PID</T>;
    const g = glyph(k, w(), h());
    if (g) return g;
    return <T x={cx} y={cy} size={k === "Integrator" || k === "Derivative" ? 12 : 10}>{clip(b().label || defOf(k)?.name || k, Math.max(4, Math.floor(w() / 6.5)))}</T>;
  };
  const portLabels = () => {
    const bb = b(), out: JSX.Element[] = [];
    bb.in_names.forEach((n, i) => { const p = bb.ins[i]; if (p && n) out.push(<T x={p[0] - bb.x + (bb.flip ? -4 : 4)} y={p[1] - bb.y} size={8.5} anchor={bb.flip ? "end" : "start"}>{n}</T>); });
    bb.out_names.forEach((n, i) => { const p = bb.outs[i]; if (p && n) out.push(<T x={p[0] - bb.x + (bb.flip ? 4 : -4)} y={p[1] - bb.y} size={8.5} anchor={bb.flip ? "start" : "end"}>{n}</T>); });
    return out;
  };
  const signs = () => {
    const bb = b(); if (bb.kind !== "Sum") return null;
    const s = (bb.params.signs ?? "").replace(/[^+-]/g, "");
    return <For each={bb.ins}>{(p, i) => <T x={p[0] - bb.x + (bb.flip ? -7 : 7)} y={p[1] - bb.y - 7} size={10} bold>{s[i()] === "-" ? "−" : "+"}</T>}</For>;
  };
  return <>{shape()}{inner()}{(b().sub || b().kind === "DCMotor" || b().kind === "PMSM") && portLabels()}{signs()}</>;
};

/** The latest value shown on a Display block. */
const displayValue = (id: number) => {
  const r = results(), cs = r?.chans ?? chans();
  const k = cs.findIndex(c => c.sink === id);
  if (k < 0 || !r) return "—";
  return num(r.last[k]);
};

// ---------- the editor ----------
type Drag =
  | { k: "move"; ids: number[]; x0: number; y0: number; dx: number; dy: number }
  | { k: "wire"; from: [number, number] | null; to: [number, number] | null; a: Pt; b: Pt }
  | { k: "box"; a: Pt; b: Pt; add: boolean }
  | { k: "pan"; cx: number; cy: number; p0: Pt }
  | { k: "size"; id: number; x0: number; y0: number; b: BlockView; w: number; h: number }
  | { k: "bend"; id: number; a: Pt; b: Pt };

export const Canvas = () => {
  const [drag, setDrag] = createSignal<Drag | null>(null);
  const [space, setSpace] = createSignal(false);
  const [hover, setHover] = createSignal<number | null>(null);
  const v = dview;
  const blockMap = createMemo(() => new Map((v()?.blocks ?? []).map(b => [b.id, b])));
  const selSet = createMemo(() => new Set(sel()));
  const errSet = createMemo(() => new Set(issues().filter(i => !i.warn).map(i => i.id)));
  const warnSet = createMemo(() => new Set(issues().filter(i => i.warn).map(i => i.id)));

  const portAt = (p: Pt, kind: "in" | "out"): [number, number] | null => {
    let best: [number, number] | null = null, bd = 12 / zoom();
    for (const b of v()?.blocks ?? []) for (const [i, q] of (kind === "in" ? b.ins : b.outs).entries()) {
      const d = Math.hypot(q[0] - p[0], q[1] - p[1]);
      if (d < bd) { bd = d; best = [b.id, i]; }
    }
    return best;
  };
  const blockAt = (p: Pt) => [...(v()?.blocks ?? [])].reverse().find(b => p[0] >= b.x - 2 && p[0] <= b.x + b.w + 2 && p[1] >= b.y - 2 && p[1] <= b.y + b.h + 2);

  const down = (e: MouseEvent) => {
    if (!v()) return;
    (e.currentTarget as HTMLElement).focus?.();
    const p = toDiagram(e.clientX, e.clientY);
    if (e.button === 1 || space()) { e.preventDefault(); setDrag({ k: "pan", cx: e.clientX, cy: e.clientY, p0: pan() }); return; }
    if (e.button !== 0) return;
    const t = e.target as Element;
    const add = e.shiftKey || e.ctrlKey || e.metaKey;
    // Resize handle
    const hid = t.getAttribute("data-size");
    if (hid) { const b = blockMap().get(+hid)!; setDrag({ k: "size", id: b.id, x0: p[0], y0: p[1], b, w: b.w, h: b.h }); return; }
    // Ports start wires.
    const po = portAt(p, "out");
    if (po) { const b = blockMap().get(po[0])!; setDrag({ k: "wire", from: po, to: null, a: b.outs[po[1]], b: p }); return; }
    const pi = portAt(p, "in");
    if (pi) {
      const b = blockMap().get(pi[0])!;
      // Dragging from a connected input picks up that wire's source end.
      const w = v()!.wires.find(x => x.to[0] === pi[0] && x.to[1] === pi[1]);
      if (w) { act({ Delete: { ids: [w.id] } }, true); const s = blockMap().get(w.from[0])!; setDrag({ k: "wire", from: w.from, to: null, a: s.outs[w.from[1]], b: p }); return; }
      setDrag({ k: "wire", from: null, to: pi, a: b.ins[pi[1]], b: p }); return;
    }
    const wid = t.closest("[data-wire]")?.getAttribute("data-wire");
    if (wid) {
      const id = +wid;
      setSel(add ? (selSet().has(id) ? sel().filter(x => x !== id) : [...sel(), id]) : [id]);
      setDrag({ k: "bend", id, a: p, b: p });
      return;
    }
    const b = blockAt(p);
    if (b) {
      let ids = sel();
      if (add) ids = selSet().has(b.id) ? ids.filter(x => x !== b.id) : [...ids, b.id];
      else if (!selSet().has(b.id)) ids = [b.id];
      setSel(ids);
      const moving = ids.filter(id => blockMap().has(id));
      if (moving.length) setDrag({ k: "move", ids: moving, x0: p[0], y0: p[1], dx: 0, dy: 0 });
      return;
    }
    if (!add) setSel([]);
    setDrag({ k: "box", a: p, b: p, add });
  };

  const move = (e: MouseEvent) => {
    const d = drag(); if (!d) return;
    const p = svgEl ? toDiagram(e.clientX, e.clientY) : [0, 0] as Pt;
    if (d.k === "pan") setPan([d.p0[0] + e.clientX - d.cx, d.p0[1] + e.clientY - d.cy]);
    else if (d.k === "move") {
      // Snap the first block's corner, so blocks land on the grid.
      const first = blockMap().get(d.ids[0])!;
      const dx = snap(first.x + p[0] - d.x0) - first.x, dy = snap(first.y + p[1] - d.y0) - first.y;
      setDrag({ ...d, dx, dy });
    } else if (d.k === "wire" || d.k === "box" || d.k === "bend") setDrag({ ...d, b: p });
    else if (d.k === "size") setDrag({ ...d, w: Math.max(20, snap(d.b.w + p[0] - d.x0)), h: Math.max(16, snap(d.b.h + p[1] - d.y0)) });
  };

  const up = async (e: MouseEvent) => {
    const d = drag(); setDrag(null); if (!d) return;
    const p = svgEl ? toDiagram(e.clientX, e.clientY) : d.k === "wire" ? d.b : [0, 0] as Pt;
    if (d.k === "move" && (d.dx || d.dy)) act({ Move: { ids: d.ids, dx: d.dx, dy: d.dy } });
    else if (d.k === "size" && (d.w !== d.b.w || d.h !== d.b.h)) act({ Resize: { id: d.id, x: d.b.x, y: d.b.y, w: d.w, h: d.h } });
    else if (d.k === "bend") {
      // Dragging a wire puts a bend point where it is dropped.
      if (Math.hypot(d.b[0] - d.a[0], d.b[1] - d.a[1]) > 6 / zoom()) {
        const w = v()?.wires.find(x => x.id === d.id);
        if (w) act({ SetPts: { id: w.id, pts: [...w.pts, [snap(d.b[0]), snap(d.b[1])]] } });
      }
    } else if (d.k === "box") {
      const [x0, x1] = [Math.min(d.a[0], d.b[0]), Math.max(d.a[0], d.b[0])], [y0, y1] = [Math.min(d.a[1], d.b[1]), Math.max(d.a[1], d.b[1])];
      if (x1 - x0 < 3 && y1 - y0 < 3) return;
      const inBox = (v()?.blocks ?? []).filter(b => b.x >= x0 && b.x + b.w <= x1 && b.y >= y0 && b.y + b.h <= y1).map(b => b.id);
      const ws = (v()?.wires ?? []).filter(w => inBox.includes(w.from[0]) && inBox.includes(w.to[0])).map(w => w.id);
      setSel(d.add ? [...new Set([...sel(), ...inBox, ...ws])] : [...inBox, ...ws]);
    } else if (d.k === "wire") {
      if (d.from) {
        let to = portAt(p, "in");
        if (!to) { const b = blockAt(p); if (b && b.id !== d.from[0]) { const used = new Set(v()!.wires.filter(w => w.to[0] === b.id).map(w => w.to[1])); const i = b.ins.findIndex((_, k) => !used.has(k)); if (i >= 0) to = [b.id, i]; } }
        if (to) act({ Connect: { from: d.from, to, pts: [] } });
      } else if (d.to) {
        let from = portAt(p, "out");
        if (!from) { const b = blockAt(p); if (b && b.outs.length && b.id !== d.to[0]) from = [b.id, 0]; }
        if (from) act({ Connect: { from, to: d.to, pts: [] } });
      }
    }
  };

  const dbl = (e: MouseEvent) => {
    const p = toDiagram(e.clientX, e.clientY);
    const b = blockAt(p);
    if (!b) { quickInsert(p); return; }
    if (b.sub) enter(b.id);
    else if (b.kind === "Scope" || b.kind === "XY") { if (!scopes().includes(b.id)) setScopes([...scopes(), b.id]); }
    else { setSel([b.id]); document.querySelector<HTMLInputElement>(".insp input, .insp select")?.focus(); }
  };

  const wheel = (e: WheelEvent) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) { const r = svgEl!.getBoundingClientRect(); zoomBy(Math.exp(-e.deltaY / 400), [e.clientX - r.left, e.clientY - r.top]); }
    else setPan([pan()[0] - (e.shiftKey ? e.deltaY : e.deltaX), pan()[1] - (e.shiftKey ? 0 : e.deltaY)]);
  };

  const keys = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (k === " ") { setSpace(true); return; }
    if (e.key === "Delete" || e.key === "Backspace") { e.preventDefault(); C.del(); }
    else if (ctrl && k === "c") { e.preventDefault(); C.copy(); }
    else if (ctrl && k === "x") { e.preventDefault(); C.cut(); }
    else if (ctrl && k === "v") { e.preventDefault(); C.paste(); }
    else if (ctrl && k === "a") { e.preventDefault(); setSel([...(v()?.blocks.map(b => b.id) ?? []), ...(v()?.wires.map(w => w.id) ?? [])]); }
    else if (ctrl && k === "g") { e.preventDefault(); C.group(); }
    else if (ctrl && k === "r") { e.preventDefault(); C.flip(); }
    else if (e.key === "F2") { e.preventDefault(); C.rename(); }
    else if (e.key === "Escape") setSel([]);
    else if (e.key.startsWith("Arrow") && sel().length) {
      e.preventDefault();
      const s = e.shiftKey ? 1 : GRID, dx = e.key === "ArrowLeft" ? -s : e.key === "ArrowRight" ? s : 0, dy = e.key === "ArrowUp" ? -s : e.key === "ArrowDown" ? s : 0;
      const ids = sel().filter(id => blockMap().has(id)); if (ids.length) act({ Move: { ids, dx, dy } });
    }
  };
  const keyup = (e: KeyboardEvent) => { if (e.key === " ") setSpace(false); };
  onMount(() => { addEventListener("mousemove", move); addEventListener("mouseup", up); addEventListener("keyup", keyup); });
  onCleanup(() => { removeEventListener("mousemove", move); removeEventListener("mouseup", up); removeEventListener("keyup", keyup); });

  const drop = (e: DragEvent) => {
    e.preventDefault();
    const kind = e.dataTransfer?.getData("text/datum-block"); if (!kind) return;
    const p = toDiagram(e.clientX, e.clientY), d = defOf(kind);
    C.addBlock(kind, [snap(p[0] - (d?.w ?? 60) / 2), snap(p[1] - (d?.h ?? 40) / 2)]);
  };

  const ctx = (e: MouseEvent) => {
    e.preventDefault();
    const p = toDiagram(e.clientX, e.clientY);
    const b = blockAt(p);
    if (b && !selSet().has(b.id)) setSel([b.id]);
    const items: Item[] = b ? [
      ...(b.sub ? [{ label: "Open subsystem", run: () => enter(b.id) } as Item] : []),
      ...(b.kind === "Scope" || b.kind === "XY" ? [{ label: "Open scope", icon: "scope", run: () => setScopes([...new Set([...scopes(), b.id])]) } as Item] : []),
      { label: "Rename…", icon: "rename", run: C.rename, keys: "F2" }, { label: "Flip", icon: "flip", run: C.flip, keys: "Ctrl+R" },
      "-", { label: "Cut", icon: "cut", run: C.cut, keys: "Ctrl+X" }, { label: "Copy", icon: "copy", run: C.copy, keys: "Ctrl+C" }, { label: "Delete", icon: "del", run: C.del, keys: "Del" },
      "-", { label: "Create subsystem", icon: "sub", run: C.group, keys: "Ctrl+G" }, ...(b.sub ? [{ label: "Ungroup", icon: "ungroup", run: () => C.ungroup(b.id) } as Item] : []),
      "-", { label: `About ${defOf(b.kind)?.name ?? b.kind}`, icon: "help", run: () => helpCard(`block:${b.kind}`) },
      ...(["TransferFcn", "ZPK", "StateSpace", "PID", "DCMotor"].includes(b.kind) ? ["-" as Item, { label: "Bode plot", icon: "bode", run: () => C.analyze("bode") } as Item, { label: "Step response", icon: "specs", run: () => C.analyze("step") } as Item] : []),
    ] : [
      { label: "Add block here…", icon: "block", run: () => quickInsert(p) }, { label: "Paste", icon: "paste", run: C.paste, keys: "Ctrl+V", off: !out()?.clip },
      "-", { label: "Select all", run: () => setSel(v()?.blocks.map(b => b.id) ?? []), keys: "Ctrl+A" }, { label: "Fit to window", icon: "fit", run: fit },
    ];
    openMenu(e, items);
  };

  // Wires attached to blocks being moved are drawn as simple previews until the drop.
  const preview = (w: WireView): string | null => {
    const d = drag(); if (!d || d.k !== "move") return null;
    const fm = d.ids.includes(w.from[0]), tm = d.ids.includes(w.to[0]);
    if (!fm && !tm) return null;
    const a = w.path[0], b = w.path[w.path.length - 1];
    const A: Pt = fm ? [a[0] + d.dx, a[1] + d.dy] : a, B: Pt = tm ? [b[0] + d.dx, b[1] + d.dy] : b;
    const mx = (A[0] + B[0]) / 2;
    return `M${A[0]} ${A[1]}H${mx}V${B[1]}H${B[0]}`;
  };
  const off = (id: number) => { const d = drag(); return d?.k === "move" && d.ids.includes(id) ? [d.dx, d.dy] : [0, 0]; };
  const pathD = (pts: Pt[]) => pts.map((p, i) => `${i ? "L" : "M"}${p[0]} ${p[1]}`).join("");

  return (
    <div class="canvas" classList={{ grid: opts().grid, panning: space() || drag()?.k === "pan" }} tabindex="0" onKeyDown={keys}
      style={{ "background-size": `${GRID * 2 * zoom()}px ${GRID * 2 * zoom()}px`, "background-position": `${pan()[0]}px ${pan()[1]}px` }}
      onDragOver={e => e.preventDefault()} onDrop={drop}>
      <Crumbs />
      <svg ref={svgEl} class="dsvg" onMouseDown={down} onDblClick={dbl} onWheel={wheel} onContextMenu={ctx}>
        <defs>
          <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" class="warrow" /></marker>
          <marker id="arrs" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" class="warrow sel" /></marker>
        </defs>
        <g transform={`translate(${pan()[0]} ${pan()[1]}) scale(${zoom()})`}>
          <For each={v()?.wires ?? []}>{w => {
            const s = () => selSet().has(w.id), pv = () => preview(w);
            return <g data-wire={w.id} class="wire" classList={{ sel: s() }}>
              <path d={pv() ?? pathD(w.path)} class="whit" />
              <path d={pv() ?? pathD(w.path)} class="wline" marker-end={s() ? "url(#arrs)" : "url(#arr)"} />
            </g>;
          }}</For>
          <For each={v()?.blocks ?? []}>{b => {
            const o = () => off(b.id);
            return <g transform={`translate(${b.x + o()[0]} ${b.y + o()[1]})`} class="blk" classList={{ sel: selSet().has(b.id), err: errSet().has(b.id), warn: warnSet().has(b.id), hover: hover() === b.id }}
              onMouseEnter={() => setHover(b.id)} onMouseLeave={() => setHover(null)}>
              <title>{`${b.name} (${defOf(b.kind)?.name ?? b.kind})${issues().filter(i => i.id === b.id).map(i => "\n" + i.msg).join("")}`}</title>
              <BlockArt b={b} />
              <text x={b.w / 2} y={b.h + 11} class="bname" text-anchor="middle">{b.kind === "Subsystem" ? "" : b.name}</text>
              <Show when={sel().length === 1 && selSet().has(b.id) && b.kind !== "Sum"}><rect data-size={b.id} x={b.w - 4} y={b.h - 4} width={8} height={8} class="bsize" /></Show>
            </g>;
          }}</For>
          <For each={v()?.blocks ?? []}>{b => {
            const o = () => off(b.id);
            const connected = createMemo(() => new Set((v()?.wires ?? []).filter(w => w.to[0] === b.id).map(w => w.to[1])));
            return <g transform={`translate(${o()[0]} ${o()[1]})`}>
              <For each={b.ins}>{(p, i) => <path d={b.flip ? `M${p[0] + 1} ${p[1] - 4}l-6 4l6 4z` : `M${p[0] - 1} ${p[1] - 4}l6 4l-6 4z`} class="port in" classList={{ open: !connected().has(i()) }} />}</For>
              <For each={b.outs}>{p => <path d={b.flip ? `M${p[0]} ${p[1] - 4}l-6 4l6 4z` : `M${p[0]} ${p[1] - 4}l6 4l-6 4z`} class="port out" />}</For>
            </g>;
          }}</For>
          <Show when={drag()}>{d => {
            const x = d();
            if (x.k === "wire") return <path d={`M${x.a[0]} ${x.a[1]}H${(x.a[0] + x.b[0]) / 2}V${x.b[1]}H${x.b[0]}`} class="wline temp" />;
            if (x.k === "box") return <rect x={Math.min(x.a[0], x.b[0])} y={Math.min(x.a[1], x.b[1])} width={Math.abs(x.b[0] - x.a[0])} height={Math.abs(x.b[1] - x.a[1])} class="marquee" />;
            if (x.k === "size") return <rect x={x.b.x} y={x.b.y} width={x.w} height={x.h} class="marquee" />;
            if (x.k === "bend") return <circle cx={x.b[0]} cy={x.b[1]} r={4} class="bendpt" />;
            return null;
          }}</Show>
        </g>
      </svg>
      <Show when={v() && !v()!.blocks.length}>
        <div class="empty"><b>{parent() ? "This subsystem is empty" : "An empty model"}</b><span>Drag blocks from the Library on the left, or double-click here to add one. Then drag from an output (▸ on the right of a block) to an input to connect them.</span>
          <span><button class="btn sm" onClick={() => setBackstage("home")}>Open an example</button></span></div>
      </Show>
      <MiniMap />
      <div class="zoomctl"><button title="Zoom out" onClick={() => zoomBy(1 / 1.2)}>−</button><span>{Math.round(zoom() * 100)}%</span><button title="Zoom in" onClick={() => zoomBy(1.2)}>+</button><button title="Fit the diagram to the window" onClick={fit}>Fit</button></div>
      <Show when={running()}><div class="runbadge" onClick={() => setView("scopes")}>Running…</div></Show>
    </div>
  );
};

/** The path to the open subsystem: click a level to go back up. */
const Crumbs = () => (
  <Show when={(dview()?.path.length ?? 0) > 1}>
    <div class="crumbs"><For each={dview()!.path}>{([id, name], i) => <>{i() > 0 && <span class="sep">›</span>}<button classList={{ on: id === parent() }} onClick={() => enter(id)}>{name || "Model"}</button></>}</For>
      <button class="btn sm up" onClick={() => enter(dview()!.path[dview()!.path.length - 2][0])}>↑ Up one level</button></div>
  </Show>
);

const MiniMap = () => {
  const box = createMemo(() => {
    const v = dview(); if (!v || v.blocks.length < 12) return null;
    const x0 = Math.min(...v.blocks.map(b => b.x)) - 20, y0 = Math.min(...v.blocks.map(b => b.y)) - 20;
    const x1 = Math.max(...v.blocks.map(b => b.x + b.w)) + 20, y1 = Math.max(...v.blocks.map(b => b.y + b.h)) + 20;
    return { x0, y0, w: x1 - x0, h: y1 - y0 };
  });
  const go = (e: MouseEvent) => {
    const bx = box(); if (!bx || !svgEl) return;
    const r = (e.currentTarget as SVGElement).getBoundingClientRect(), s = svgEl.getBoundingClientRect();
    const x = bx.x0 + (e.clientX - r.left) / r.width * bx.w, y = bx.y0 + (e.clientY - r.top) / r.height * bx.h;
    setPan([s.width / 2 - x * zoom(), s.height / 2 - y * zoom()]);
  };
  return <Show when={box()}>{bx => {
    const vp = () => { const s = svgEl?.getBoundingClientRect(); if (!s) return null; const [x, y] = [-pan()[0] / zoom(), -pan()[1] / zoom()]; return { x, y, w: s.width / zoom(), h: s.height / zoom() }; };
    return <svg class="minimap" viewBox={`${bx().x0} ${bx().y0} ${bx().w} ${bx().h}`} preserveAspectRatio="xMidYMid meet" onMouseDown={go}>
      <For each={dview()!.blocks}>{b => <rect x={b.x} y={b.y} width={b.w} height={b.h} class="mmb" />}</For>
      <Show when={vp()}>{r => <rect x={r().x} y={r().y} width={r().w} height={r().h} class="mmv" />}</Show>
    </svg>;
  }}</Show>;
};

/** Double-click on empty space: type to find a block and add it there. */
export const quickInsert = (at?: Pt) => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const list = createMemo(() => { const s = q().toLowerCase(); return lib().blocks.filter(d => !d.planned && (d.name.toLowerCase().includes(s) || d.kind.toLowerCase().includes(s) || d.cat.toLowerCase().includes(s))).slice(0, 12); });
  const go = (i: number) => { const d = list()[i]; if (!d) return; closeDialog(); const p = at ?? viewCenter(); C.addBlock(d.kind, [snap(p[0] - d.w / 2), snap(p[1] - d.h / 2)]); };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov top" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Add a block: type its name (gain, integrator, scope…)" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(d, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{d.name}</span><kbd>{d.cat}</kbd></button>}</For></div>
      </div>
    </div>
  );
});

