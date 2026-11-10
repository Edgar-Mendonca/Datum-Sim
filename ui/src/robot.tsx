// The robot arm module (0.3.6): the Arm view, an animated drawing of the two-link arm from the
// logged joint angles, with play and a time slider, and the tip's path traced.
import { Show, createSignal, createEffect, createMemo, on, onCleanup } from "solid-js";
import { num, invoke } from "./api";
import { chans, results, running, liveTick, dview } from "./store";
import { OKABE } from "./scope";

export const ArmPlot = (p: { id: number }) => {
  const [s, setS] = createSignal<{ t: number[]; q1: number[]; q2: number[] } | null>(null);
  const [k, setK] = createSignal(0), [play, setPlay] = createSignal(false);
  const ix = createMemo(() => chans().map((c, i) => [c, i] as const).filter(([c]) => c.sink === p.id && !c.probe).map(([, i]) => i));
  const len = (key: string, d: number) => { const v = parseFloat(dview()?.blocks.find(b => b.id === p.id)?.params?.[key] ?? ""); return isFinite(v) && v > 0 ? v : d; };
  createEffect(on([liveTick, ix, running], async () => {
    const i = ix(); const r = results();
    if (i.length < 2 || running() || !r) return;
    const x = await invoke<{ t: number[]; y: number[][] }>("series", { chans: i.slice(0, 2), t0: r.t0, t1: r.t1, buckets: 1500 });
    setS({ t: x.t, q1: x.y[0], q2: x.y[1] });
    setK(x.t.length - 1);
  }));
  // Play at real time from the current frame (from the start when at the end).
  let raf = 0;
  const stop = () => { cancelAnimationFrame(raf); setPlay(false); };
  const start = () => {
    const d = s(); if (!d) return;
    if (k() >= d.t.length - 1) setK(0);
    const t0 = d.t[k()], w0 = performance.now();
    setPlay(true);
    const step = () => {
      const t = t0 + (performance.now() - w0) / 1000;
      let j = k(); while (j < d.t.length - 1 && d.t[j + 1] <= t) j++;
      setK(j);
      if (j >= d.t.length - 1) return stop();
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
  };
  onCleanup(stop);
  const W = 360, H = 260;
  const view = createMemo(() => {
    const l1 = len("l1", 0.5), l2 = len("l2", 0.4), R = (l1 + l2) * 1.1, sc = Math.min(W, H) / (2 * R);
    const X = (x: number) => W / 2 + x * sc, Y = (y: number) => H / 2 - y * sc;
    return { l1, l2, X, Y };
  });
  const arm = (q1: number, q2: number) => {
    const { l1, l2 } = view();
    const e = [l1 * Math.cos(q1), l1 * Math.sin(q1)], t = [e[0] + l2 * Math.cos(q1 + q2), e[1] + l2 * Math.sin(q1 + q2)];
    return { e, t };
  };
  const trace = createMemo(() => { const d = s(); if (!d) return ""; const { X, Y } = view(); return d.q1.map((q, i) => { const { t } = arm(q, d.q2[i]); return `${i ? "L" : "M"}${X(t[0]).toFixed(1)} ${Y(t[1]).toFixed(1)}`; }).join(""); });
  return <div class="plot armview"><Show when={s()} fallback={<p class="muted pempty">Run the model to see the arm move.</p>}>{d => {
    const { X, Y } = view();
    const cur = () => arm(d().q1[k()], d().q2[k()]);
    return <>
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMidYMid meet">
        <line x1={0} x2={W} y1={Y(0)} y2={Y(0)} class="pzaxis" />
        <path d={trace()} fill="none" stroke={OKABE[0]} stroke-width="1" stroke-dasharray="3 3" opacity="0.7" />
        <rect x={X(0) - 14} y={Y(0)} width="28" height="10" class="armbase" />
        <line x1={X(0)} y1={Y(0)} x2={X(cur().e[0])} y2={Y(cur().e[1])} class="armlink" />
        <line x1={X(cur().e[0])} y1={Y(cur().e[1])} x2={X(cur().t[0])} y2={Y(cur().t[1])} class="armlink l2" />
        <circle cx={X(0)} cy={Y(0)} r="6" class="armjoint" /><circle cx={X(cur().e[0])} cy={Y(cur().e[1])} r="5" class="armjoint" /><circle cx={X(cur().t[0])} cy={Y(cur().t[1])} r="4" class="armtip" />
      </svg>
      <div class="armbar">
        <button class="btn sm armplay" onClick={() => (play() ? stop() : start())}>{play() ? "Pause" : "Play"}</button>
        <input type="range" class="armtime" min="0" max={d().t.length - 1} value={k()} onInput={e => { stop(); setK(+e.currentTarget.value); }} />
        <span class="mono small">t = {num(d().t[k()], 3)} s · q = ({num(d().q1[k()], 3)}, {num(d().q2[k()], 3)}) rad</span>
      </div>
    </>;
  }}</Show></div>;
};
