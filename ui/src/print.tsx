// File › Print: settings on the left, a live page preview on the right (FAMILY §8). Pages are
// laid out here at real size (the diagram as vector SVG, the scopes as pictures, a results
// report) and printed with the WebView's print dialog, where "Save as PDF" makes a PDF.
// Built on Datum Plan's print page (paper sizes, header and footer slots and codes, @page).
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import { createStore } from "solid-js/store";
import { invoke, DView, num } from "./api";
import { out, opts, results, chans, call, toast } from "./store";
import { Pick, Check } from "./ui";
import { sinks } from "./scope";

export const PAPERS: [string, number, number][] = [["A4", 210, 297], ["A3", 297, 420], ["A2", 420, 594], ["Letter", 215.9, 279.4], ["Legal", 215.9, 355.6], ["Tabloid", 279.4, 431.8]];
const MM = 96 / 25.4;
type Slots = [string, string, string];
type Setup = { paper: string; landscape: boolean; margins: [number, number, number, number]; header: Slots; footer: Slots; diagram: boolean; subs: boolean; scopes: boolean; report: boolean };
export const CODES: [string, string][] = [["&[Page]", "Page number"], ["&[Pages]", "Total pages"], ["&[Date]", "Today's date"], ["&[Model]", "Model name"], ["&[Company]", "Company"], ["&[File]", "File name"]];
const [ps, setPs] = createStore<Setup>({
  paper: "A4", landscape: true, margins: [12, 10, 12, 10], header: ["&[Company]", "&[Model]", "&[Date]"], footer: ["Datum Sim", "Page &[Page] of &[Pages]", "&[File]"],
  diagram: true, subs: true, scopes: true, report: true,
});
const codes = (t: string, page: number, pages: number) => t.replace(/&\[Page\]/g, String(page)).replace(/&\[Pages\]/g, String(pages)).replace(/&\[Date\]/g, new Date().toLocaleDateString())
  .replace(/&\[Model\]/g, out()?.name ?? "").replace(/&\[Company\]/g, opts().company).replace(/&\[File\]/g, out()?.path?.split(/[\\/]/).pop() ?? "not saved");

type Page = { kind: "diagram"; v: DView } | { kind: "scope"; id: number; img: string; name: string } | { kind: "report" };

/** One printed page: header, body, footer. */
const Sheet = (p: { pg: Page; n: number; pages: number; w: number; h: number }) => {
  const inner = () => [p.w - (ps.margins[1] + ps.margins[3]) * MM, p.h - (ps.margins[0] + ps.margins[2]) * MM] as const;
  return (
    <div class="ppage" style={{ width: `${p.w}px`, height: `${p.h}px` }}>
      <div class="pin" style={{ left: `${ps.margins[3] * MM}px`, top: `${ps.margins[0] * MM}px`, width: `${inner()[0]}px`, height: `${inner()[1]}px` }}>
        <div class="phead"><For each={ps.header}>{(t, i) => <span class={`ps${i()}`}>{codes(t, p.n, p.pages)}</span>}</For></div>
        <div class="pbody">{body(p.pg, inner()[0], inner()[1] - 90)}</div>
        <div class="pfoot"><For each={ps.footer}>{(t, i) => <span class={`ps${i()}`}>{codes(t, p.n, p.pages)}</span>}</For></div>
      </div>
    </div>
  );
};

const body = (pg: Page, w: number, h: number): JSX.Element => {
  if (pg.kind === "scope") return <div class="pscope"><h3>{pg.name}</h3><img src={pg.img} style={{ "max-width": `${w}px`, "max-height": `${h - 30}px` }} /></div>;
  if (pg.kind === "report") return <Report />;
  const v = pg.v;
  if (!v.blocks.length) return <p>Empty diagram.</p>;
  const x0 = Math.min(...v.blocks.map(b => b.x)) - 20, y0 = Math.min(...v.blocks.map(b => b.y)) - 20;
  const x1 = Math.max(...v.blocks.map(b => b.x + b.w)) + 20, y1 = Math.max(...v.blocks.map(b => b.y + b.h)) + 30;
  return <div class="pdiag"><h3>{v.path.map(x => x[1]).join(" › ")}</h3>
    <svg viewBox={`${x0} ${y0} ${x1 - x0} ${y1 - y0}`} width={w} height={h - 30} preserveAspectRatio="xMidYMid meet" class="printdiag">
      <defs><marker id="parr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="#33404F" /></marker></defs>
      <For each={v.wires}>{wi => <path d={wi.path.map((p, i) => `${i ? "L" : "M"}${p[0]} ${p[1]}`).join("")} fill="none" stroke="#33404F" stroke-width="1.2" marker-end="url(#parr)" />}</For>
      <For each={v.blocks}>{b => <g transform={`translate(${b.x} ${b.y})`}>
        {b.kind === "Sum" ? <circle cx={b.w / 2} cy={b.h / 2} r={b.h / 2} fill="#fff" stroke="#1B2733" stroke-width="1.2" /> : <rect width={b.w} height={b.h} rx={3} fill={b.sub ? "#FCEEE6" : "#fff"} stroke="#1B2733" stroke-width="1.2" />}
        <text x={b.w / 2} y={b.h / 2} font-size="10" text-anchor="middle" dominant-baseline="central" fill="#1B2733">{b.kind === "Sum" ? "Σ" : (b.label.split("\n")[0] || b.kind).slice(0, 16)}</text>
        <text x={b.w / 2} y={b.h + 11} font-size="9.5" text-anchor="middle" fill="#55606E">{b.name}</text></g>}</For>
    </svg></div>;
};

const Report = () => {
  const r = results();
  return <div class="preport">
    <h3>{out()?.name}</h3>
    <p>{out()?.notes}</p>
    <h4>Run settings</h4>
    <table class="tbl"><tbody>
      <tr><td>Time</td><td>{out()?.sim.start} s to {out()?.sim.stop} s</td></tr>
      <tr><td>Solver</td><td>{out()?.sim.solver} · relative tolerance {out()?.sim.rtol} · step {out()?.sim.step} s</td></tr>
      <Show when={r}><tr><td>Last run</td><td>{r!.stats.steps.toLocaleString()} steps, {r!.stats.events} events, {Math.round(r!.stats.ms)} ms</td></tr></Show>
    </tbody></table>
    <Show when={out()?.vars.length}><h4>Variables</h4><table class="tbl"><tbody><For each={out()!.vars}>{v => <tr><td>{v.name}</td><td>{v.expr} {v.unit}</td><td>{v.note}</td></tr>}</For></tbody></table></Show>
    <Show when={r}><h4>Final values</h4><table class="tbl"><tbody><For each={r!.chans}>{(c, i) => <tr><td>{c.name}</td><td class="r">{num(r!.last[i()])}</td></tr>}</For></tbody></table></Show>
  </div>;
};

const [pages, setPages] = createSignal<Page[]>([]);
const [printing, setPrinting] = createSignal(false);

/** Gather the diagram (and subsystems), scope pictures and the report. */
const collect = async () => {
  const out: Page[] = [];
  if (ps.diagram) {
    const walk = async (id: number, depth: number) => {
      const v = await invoke<DView>("diagram", { parent: id }).catch(() => null); if (!v) return;
      out.push({ kind: "diagram", v });
      if (ps.subs && depth < 6) for (const b of v.blocks.filter(b => b.sub)) await walk(b.id, depth + 1);
    };
    await walk(0, 0);
  }
  if (ps.scopes && results()) {
    for (const s of sinks().filter(s => s.kind === "Scope")) {
      const cv = document.querySelector<HTMLCanvasElement>(`[data-scope="${s.id}"] canvas`);
      if (cv) out.push({ kind: "scope", id: s.id, name: s.name, img: cv.toDataURL("image/png") });
    }
  }
  if (ps.report) out.push({ kind: "report" });
  setPages(out);
};

export const PrintPage = () => {
  setPs({ paper: opts().paper || "A4", landscape: opts().landscape !== false });
  const [zoomP, setZoomP] = createSignal(0.5);
  const size = createMemo(() => { const p = PAPERS.find(x => x[0] === ps.paper) ?? PAPERS[0]; const [w, h] = ps.landscape ? [p[2], p[1]] : [p[1], p[2]]; return { pw: w, ph: h, w: w * MM, h: h * MM }; });
  onMount(collect);
  const doPrint = () => {
    setPrinting(true);
    const css = document.createElement("style");
    css.textContent = `@page { size: ${size().pw}mm ${size().ph}mm; margin: 0 }`;
    document.head.append(css);
    document.documentElement.classList.add("printing");
    setTimeout(() => { window.print(); document.documentElement.classList.remove("printing"); css.remove(); setPrinting(false); }, 200);
  };
  return (
    <div class="printpage">
      <div class="pset">
        <h1>Print</h1>
        <div class="pbtns"><button class="btn pri big" onClick={doPrint}><svg class="ic"><use href="#i-print" /></svg> Print / Save as PDF</button></div>
        <p class="muted">In the print window choose your printer, or <b>Save as PDF</b>. {pages().length} {pages().length === 1 ? "page" : "pages"} · {ps.paper} {ps.landscape ? "landscape" : "portrait"}.</p>
        <h4>What to print</h4>
        <Check label="The diagram" value={ps.diagram} set={v => { setPs("diagram", v); collect(); }} />
        <Check label="Inside every subsystem" value={ps.subs} set={v => { setPs("subs", v); collect(); }} />
        <Check label="The scopes of the last run" value={ps.scopes} set={v => { setPs("scopes", v); collect(); }} />
        <Check label="A results report" value={ps.report} set={v => { setPs("report", v); collect(); }} />
        <Show when={ps.scopes && !results()}><p class="muted">Run the model to print its scopes.</p></Show>
        <Show when={ps.scopes && results() && !chans().length}><p class="muted">Open the scopes (View › Scopes) to print them.</p></Show>
        <div class="prow"><label>Paper<Pick value={ps.paper} set={v => setPs("paper", v)} options={PAPERS.map(p => p[0])} /></label><label>Orientation<Pick value={ps.landscape ? "1" : "0"} set={v => setPs("landscape", v === "1")} options={[["1", "Landscape"], ["0", "Portrait"]]} /></label></div>
        <h4>Header and footer</h4>
        <For each={["header", "footer"] as const}>{k => <div class="pslots"><b>{k === "header" ? "Header" : "Footer"}</b>
          <For each={["Left", "Center", "Right"]}>{(l, i) => <label><span>{l}</span><textarea rows="1" value={ps[k][i()]} onInput={e => setPs(k, i(), e.currentTarget.value)} /></label>}</For></div>}</For>
        <p class="muted pcodes">Codes: <For each={CODES}>{([c, l]) => <code title={l} onClick={() => navigator.clipboard?.writeText(c).then(() => toast(`${c} copied.`, true))}>{c}</code>}</For></p>
        <div class="prow">{(["Top", "Right", "Bottom", "Left"] as const).map((l, i) => <label>{l} (mm)<input type="number" min="0" max="60" value={ps.margins[i]} onChange={e => setPs("margins", i, +e.currentTarget.value)} /></label>)}</div>
        <p><button class="btn sm" onClick={collect}>Refresh preview</button> <button class="btn sm" onClick={() => call("save_file", { name: `${out()?.name ?? "model"}.svg`, data: document.querySelector(".printdiag")?.outerHTML ?? "" })}>Save diagram as SVG</button></p>
      </div>
      <div class="pprev">
        <div class="pzoom"><span class="muted">Preview</span><input type="range" min="0.25" max="1.2" step="0.05" value={zoomP()} onInput={e => setZoomP(+e.currentTarget.value)} /><span class="muted">{Math.round(zoomP() * 100)}%</span></div>
        <For each={pages()} fallback={<p class="pempty">Nothing chosen to print.</p>}>{(pg, i) =>
          <div class="pwrap" style={{ width: `${size().w * zoomP()}px`, height: `${size().h * zoomP()}px` }}><div style={{ transform: `scale(${zoomP()})`, "transform-origin": "0 0" }}><Sheet pg={pg} n={i() + 1} pages={pages().length} w={size().w} h={size().h} /></div><small>Page {i() + 1} of {pages().length}</small></div>}</For>
      </div>
      <Show when={printing()}><Portal mount={document.body}><div id="printroot"><For each={pages()}>{(pg, i) => <Sheet pg={pg} n={i() + 1} pages={pages().length} w={size().w} h={size().h} />}</For></div></Portal></Show>
    </div>
  );
};

