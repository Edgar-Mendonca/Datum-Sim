// Planned features in the window (0.2.2, copied from Datum Mesh): every feature not built yet shows
// on its ribbon tab and in File › Roadmap, dimmed, with the release that brings it. Clicked, it opens
// the dialog it will have, every field disabled. View › Planned features hides them all.
import { For, Show, createSignal, createMemo } from "solid-js";
import { openDialog, openMenu } from "./store";
import { Dialog, Icon } from "./ui";
import { FEATS, Feat, PField } from "./features";

const KEY = "dsim.planned";
const read = () => { try { return localStorage.getItem(KEY) !== "0"; } catch { return true; } };
const [showPlanned, setShowRaw] = createSignal(read());
export { showPlanned };
export const setShowPlanned = (on: boolean) => { setShowRaw(on); try { localStorage.setItem(KEY, on ? "1" : "0"); } catch { /* private window */ } };

const Fld = (p: { f: PField }) => {
  const f = p.f;
  if (f.k === "num") return <><label>{f.l}</label><div class="pf"><input class="mono" disabled value={f.v ?? ""} />{f.u && <span class="unit">{f.u}</span>}</div></>;
  if (f.k === "choice") return <><label>{f.l}</label><div><select disabled><For each={f.o}>{o => <option>{o}</option>}</For></select></div></>;
  if (f.k === "check") return <><span /><div><label class="chk"><input type="checkbox" disabled checked={!!f.on} />{f.l}</label></div></>;
  if (f.k === "text") return <><label>{f.l}</label><div><input disabled value={f.v ?? ""} /></div></>;
  if (f.k === "pick") return <><label>{f.l}</label><div><button class="btn sm" disabled>Pick {f.what}…</button></div></>;
  return <><label class="top">{f.l}</label><div><table class="tbl ptab"><thead><tr><For each={f.cols}>{c => <th>{c}</th>}</For></tr></thead>
    <tbody><For each={f.rows ?? [f.cols.map(() => ""), f.cols.map(() => "")]}>{r => <tr><For each={r}>{c => <td><input disabled value={c} /></td>}</For></tr>}</For></tbody></table></div></>;
};

/** The dialog a planned feature will have, every field disabled. */
export const plannedDialog = (f: Feat) => openDialog(() => <Dialog title={f.label} wide cancel="Close">
  <div class="pband"><Icon n="list" /><div><b>{f.v === "later" ? "Planned for a later release" : `Available in ${f.v}`}</b><span>{f.what}</span></div></div>
  <Show when={f.fields.length}><fieldset class="pform form" disabled><For each={f.fields}>{x => <Fld f={x} />}</For></fieldset></Show>
  <p class="muted small">This shows what the feature will do{f.fields.length ? " and what you will set" : ""}. Nothing here works yet. Hide planned features in View › Planned features.</p>
</Dialog>);
export const openPlanned = (id: string) => { const f = FEATS.find(x => x.id === id); if (f) plannedDialog(f); };

/** A ribbon tab's planned features: after the working tools, dimmed, each with its release. */
export const PlannedGroups = (p: { tab: string }) => {
  const list = createMemo(() => showPlanned() ? FEATS.filter(x => x.tab === p.tab) : []);
  const groups = createMemo(() => { const g = new Map<string, Feat[]>(); for (const f of list()) g.set(f.group, [...(g.get(f.group) ?? []), f]); return [...g.entries()]; });
  const cols = (fs: Feat[]) => Array.from({ length: Math.ceil(fs.length / 3) }, (_, k) => fs.slice(3 * k, 3 * k + 3));
  return <Show when={list().length}><div class="pdiv" title="Planned: not built yet. Each shows the release that brings it." /><For each={groups()}>{([name, fs]) =>
    <div class="grp planned"><div class="items"><For each={cols(fs)}>{c => <div class="stack"><For each={c}>{f =>
      <div class="rbw"><button class="rb planned" data-planned={f.id} title={`${f.label}: ${f.v === "later" ? "planned for later" : `available in ${f.v}`}. ${f.what}`} onClick={() => plannedDialog(f)}><Icon n={f.icon} /><span>{f.label}</span><i class="pv">{f.v}</i></button></div>}</For></div>}</For></div>
      <div class="cap">{name} · planned</div></div>}</For>
    <div class="grp planned pmore"><div class="items"><div class="rbw big"><button class="rb big planned" title="More planned features that do not fit: click to list them" onClick={e => {
      const hid = [...document.querySelectorAll<HTMLButtonElement>(".ribbon .grp.planned[data-hide] button[data-planned]")];
      openMenu(e, hid.map(b => ({ label: `${b.querySelector("span")?.textContent ?? ""} · ${b.querySelector(".pv")?.textContent ?? ""}`, run: () => b.click() })));
    }}><Icon n="list" /><span>More ▾</span></button></div></div><div class="cap">Planned</div></div></Show>;
};

/** File › Roadmap: every planned feature by release. */
export const PlannedByRelease = () => {
  const by = createMemo(() => { const g = new Map<string, Feat[]>(); for (const f of FEATS) g.set(f.v, [...(g.get(f.v) ?? []), f]); return [...g.entries()].sort((a, b) => a[0] === "later" ? 1 : b[0] === "later" ? -1 : a[0].localeCompare(b[0], undefined, { numeric: true })); });
  return <div class="rmfeats"><h2>Feature by feature</h2><For each={by()}>{([v, fs]) => <section><h3>{v === "later" ? "Later" : v}</h3><ul><For each={fs}>{f => <li><button class="lnk" onClick={() => plannedDialog(f)}>{f.label}</button> <span class="muted">{f.what}</span></li>}</For></ul></section>}</For></div>;
};

/** A menu of every planned feature (Help › Roadmap's split). */
export const plannedMenu = (e: MouseEvent) => openMenu(e, FEATS.map(f => ({ label: `${f.label} · ${f.v}`, run: () => plannedDialog(f) })));
