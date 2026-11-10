// Stage B (0.3.x): drives in depth. Spectrum and THD of a signal; more drive tools follow.
import { For, Show, createSignal } from "solid-js";
import { num, invoke } from "./api";
import { openDialog, results, chans } from "./store";
import { Dialog } from "./ui";
import { OKABE } from "./scope";

/** Analyze › Spectrum: harmonics and THD of a logged signal over whole cycles of its fundamental. */
export const spectrumDialog = (chan0 = 0) => openDialog(() => {
  const [k, setK] = createSignal(chan0), [f0, setF0] = createSignal(""), [cyc, setCyc] = createSignal(4), [hmax, setH] = createSignal(40);
  const [r, setR] = createSignal<any>(null), [busy, setBusy] = createSignal(false);
  const go = async () => {
    setBusy(true);
    try { setR(await invoke<any>("spectrum", { chan: k(), f0: +f0() || 0, cycles: cyc(), hmax: hmax() })); } catch (e) { setR({ error: String(e) }); } finally { setBusy(false); }
  };
  if (results()) go();
  /** Bars: each harmonic's amplitude as a percentage of the fundamental (the fundamental at 100). */
  const bars = () => {
    const x = r(); if (!x || x.error) return "";
    const hs: number[] = x.harmonics, a1 = hs[0], W = 620, H = 220, pad = 34, n = hs.length;
    const top = Math.max(5, ...hs.slice(1).map(v => v / a1 * 100)) * 1.15;
    const bw = (W - pad - 8) / n;
    const y = (p: number) => H - 22 - Math.min(p, top) / top * (H - 40);
    const b = hs.map((v, i) => { const p = i === 0 ? top : v / a1 * 100; return `<rect x="${pad + i * bw + 1}" y="${y(p)}" width="${Math.max(1, bw - 2)}" height="${H - 22 - y(p)}" fill="${i === 0 ? OKABE[1] : OKABE[0]}"><title>h${i + 1}: ${num(v, 4)} (${num(v / a1 * 100, 3)} %)</title></rect>`; }).join("");
    const ticks = [0, top / 2, top].map(p => `<text x="${pad - 4}" y="${y(p) + 3}" text-anchor="end" class="xyt">${num(p, 2)}%</text>`).join("");
    const labels = hs.map((_, i) => (i + 1) % 5 === 0 || i === 0 ? `<text x="${pad + (i + 0.5) * bw}" y="${H - 8}" text-anchor="middle" class="xyt">${i + 1}</text>` : "").join("");
    return `<svg viewBox="0 0 ${W} ${H}" class="specplot">${b}${ticks}${labels}<text x="${W - 4}" y="${H - 8}" text-anchor="end" class="xyt">harmonic</text></svg>`;
  };
  const top = () => { const x = r(); if (!x || x.error) return []; return x.harmonics.map((v: number, i: number) => [i + 1, v]).slice(1).sort((a: number[], b: number[]) => b[1] - a[1]).slice(0, 6); };
  return <Dialog title="Spectrum and THD" wide help="fft">
    <Show when={results()} fallback={<p>Run the model first.</p>}>
      <div class="form">
        <label>Signal</label><div><select class="spsig" value={k()} onChange={e => { setK(+e.currentTarget.value); go(); }}><For each={chans()}>{(c, i) => <option value={i()}>{c.name}</option>}</For></select></div>
        <label>Fundamental</label><div class="larow"><input class="mono spf0" placeholder="find it" value={f0()} onChange={e => { setF0(e.currentTarget.value); go(); }} /> Hz</div>
        <label>Cycles</label><div class="larow"><input class="mono spcyc" value={cyc()} onChange={e => { setCyc(Math.max(1, Math.round(+e.currentTarget.value) || 1)); go(); }} /> <span class="muted small">the last whole periods of the run</span></div>
        <label>Harmonics up to</label><div><input class="mono" value={hmax()} onChange={e => { setH(Math.max(2, Math.round(+e.currentTarget.value) || 40)); go(); }} /></div>
      </div>
      <Show when={!busy() || r()}><Show when={r() && !r().error} fallback={<p class="issue">{r()?.error}</p>}>
        <div class="anplot" innerHTML={bars()} />
        <table class="tbl facts"><tbody>
          <tr><td>THD</td><td class="r mono"><b>{num(r().thd * 100, 4)} %</b></td></tr>
          <tr><td>Fundamental</td><td class="r mono">{num(r().f0, 5)} Hz, amplitude {num(r().harmonics[0], 5)} (rms {num(r().harmonics[0] / Math.SQRT2, 5)})</td></tr>
          <tr><td>DC</td><td class="r mono">{num(r().dc, 4)}</td></tr>
          <tr><td>Largest harmonics</td><td class="r mono">{top().map((h: number[]) => `h${h[0]} ${num(h[1] / r().harmonics[0] * 100, 3)} %`).join(" · ")}</td></tr>
          <tr><td>Window</td><td class="r mono">{r().cycles} cycles, {num(r().t0, 5)} s to {num(r().t1, 5)} s</td></tr>
        </tbody></table>
        <p class="muted small">Amplitudes are peaks; the fundamental's bar is drawn full height. THD = √(Σ Aₕ², h = 2 … {hmax()}) / A₁. Let the run reach steady state, and log enough points (a variable-step solver may need a smaller largest step) for the harmonics you want.</p>
      </Show></Show>
    </Show>
  </Dialog>;
});
