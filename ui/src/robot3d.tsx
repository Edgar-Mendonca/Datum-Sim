// The 3D view (0.4.3): the robot after a run, drawn with three.js (MIT). The engine sends every
// link's frame for each time; the library robots are drawn with their own meshes (from the
// window's robots/ folder), others as a skeleton through their joints. Play, a time slider, an
// orbit camera (drag to turn, wheel to zoom), and the tool's path.
import { Show, createSignal, createEffect, on, onCleanup, onMount } from "solid-js";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js";
import { num, invoke } from "./api";
import { results, running, liveTick, themeTick, dview } from "./store";

interface Mesh { body: number | null; file: string; r: number[][]; p: number[]; scale: number[] }
interface Frames { lib: string; meshes: Mesh[]; parents: (number | null)[]; t: number[]; links: number[][]; joints: number[][]; tool: number[][]; object?: number[][] }

const COLOURS = [0xc2410c, 0x3b4859, 0xe58a5a, 0x5b6878, 0xa3360a, 0x8a96a3, 0xf0c2a6, 0x2f4157];

export const Robot3D = (p: { id: number }) => {
  let host!: HTMLDivElement, cv!: HTMLDivElement;
  const [f, setF] = createSignal<Frames | null>(null), [err, setErr] = createSignal("");
  const [k, setK] = createSignal(0), [play, setPlay] = createSignal(false);
  const renderer = (() => { try { return new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true }); } catch { return null; } })();
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(40, 1, 0.01, 100);
  camera.up.set(0, 0, 1);
  camera.position.set(1.8, -1.6, 1.2);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x445566, 1.6));
  const sun = new THREE.DirectionalLight(0xffffff, 1.4);
  sun.position.set(2, -1, 3);
  scene.add(sun);
  const grid = new THREE.GridHelper(2, 20, 0x8899aa, 0xc8d0d8);
  grid.rotation.x = Math.PI / 2;
  scene.add(grid);
  const robot = new THREE.Group();
  scene.add(robot);
  let parts: { body: number | null; obj: THREE.Object3D }[] = [];
  let skel: THREE.Line | null = null, path: THREE.Line | null = null, toolDot: THREE.Mesh | null = null, box: THREE.Mesh | null = null;
  const size = () => { const v = parseFloat(dview()?.blocks.find(b => b.id === p.id)?.params?.size ?? ""); return isFinite(v) && v > 0 ? v : 0.05; };
  let controls: OrbitControls | null = null;
  const draw = () => renderer?.render(scene, camera);

  const fit = () => {
    if (!renderer || !host) return;
    const w = Math.max(160, cv.clientWidth), h = Math.max(120, cv.clientHeight);
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    draw();
  };

  /** Put every part where frame i says. */
  const place = (i: number) => {
    const x = f(); if (!x || !x.links.length) return;
    const L = x.links[Math.min(i, x.links.length - 1)];
    const mat = (b: number | null) => {
      const m = new THREE.Matrix4();
      if (b == null) return m;
      const o = 12 * b;
      m.set(L[o], L[o + 1], L[o + 2], L[o + 9], L[o + 3], L[o + 4], L[o + 5], L[o + 10], L[o + 6], L[o + 7], L[o + 8], L[o + 11], 0, 0, 0, 1);
      return m;
    };
    for (const pt of parts) { pt.obj.matrixAutoUpdate = false; pt.obj.matrix.copy(mat(pt.body).multiply((pt.obj.userData.local as THREE.Matrix4))); }
    if (skel) {
      const J = x.joints[Math.min(i, x.joints.length - 1)], T = x.tool[Math.min(i, x.tool.length - 1)];
      const pts: THREE.Vector3[] = [new THREE.Vector3(0, 0, 0)];
      for (let b = 0; b < J.length / 3; b++) pts.push(new THREE.Vector3(J[3 * b], J[3 * b + 1], J[3 * b + 2]));
      pts.push(new THREE.Vector3(T[0], T[1], T[2]));
      skel.geometry.setFromPoints(pts);
    }
    if (toolDot) { const T = x.tool[Math.min(i, x.tool.length - 1)]; toolDot.position.set(T[0], T[1], T[2]); }
    if (box && x.object?.length) { const O = x.object[Math.min(i, x.object.length - 1)]; box.position.set(O[0], O[1], O[2]); }
    draw();
  };

  const build = async (x: Frames) => {
    robot.clear();
    parts = [];
    const loader = new STLLoader();
    const loads = x.lib ? x.meshes.map(async (m, i) => {
      const geo = await new Promise<THREE.BufferGeometry>((ok, no) => loader.load(`robots/${x.lib}/${m.file.replace(/\.[^.]+$/, ".stl")}`, ok, undefined, no));
      geo.computeVertexNormals();
      const mesh = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ color: COLOURS[i % COLOURS.length], metalness: 0.2, roughness: 0.6 }));
      const local = new THREE.Matrix4().set(m.r[0][0], m.r[0][1], m.r[0][2], m.p[0], m.r[1][0], m.r[1][1], m.r[1][2], m.p[1], m.r[2][0], m.r[2][1], m.r[2][2], m.p[2], 0, 0, 0, 1).multiply(new THREE.Matrix4().makeScale(m.scale[0], m.scale[1], m.scale[2]));
      mesh.userData.local = local;
      robot.add(mesh);
      parts.push({ body: m.body, obj: mesh });
    }) : [];
    const settled = await Promise.allSettled(loads);
    const failed = settled.filter(r => r.status === "rejected").length;
    // For checks: how many parts were drawn and how many could not load (and why).
    host.dataset.parts = String(parts.length);
    host.dataset.failed = String(failed);
    const why = settled.find(r => r.status === "rejected") as PromiseRejectedResult | undefined;
    if (why) host.dataset.why = String(why.reason?.message ?? why.reason);
    skel = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial({ color: 0x0c1a2b }));
    if (!x.lib || failed) robot.add(skel); else skel = null;
    path = new THREE.Line(new THREE.BufferGeometry().setFromPoints(x.tool.map(t => new THREE.Vector3(t[0], t[1], t[2]))), new THREE.LineDashedMaterial({ color: 0x2563eb, dashSize: 0.02, gapSize: 0.015 }));
    path.computeLineDistances();
    robot.add(path);
    toolDot = new THREE.Mesh(new THREE.SphereGeometry(0.015), new THREE.MeshBasicMaterial({ color: 0x2563eb }));
    robot.add(toolDot);
    box = null;
    if (x.object?.length) { const s = size(); box = new THREE.Mesh(new THREE.BoxGeometry(s, s, s), new THREE.MeshStandardMaterial({ color: 0x16a34a })); robot.add(box); }
    host.dataset.object = x.object?.length ? "1" : "0";
    place(k());
  };

  createEffect(on([liveTick, running], async () => {
    if (running() || !results()) return;
    try { const x = await invoke<Frames>("robot_frames", { id: p.id }); setErr(""); setF(x); setK(x.t.length - 1); await build(x); }
    catch (e) { setErr(String(e)); }
  }));
  createEffect(on(themeTick, () => { renderer?.setClearColor(getComputedStyle(document.documentElement).getPropertyValue("--bg2").trim() || "#ffffff"); draw(); }));

  let raf = 0;
  const stop = () => { cancelAnimationFrame(raf); setPlay(false); };
  const start = () => {
    const x = f(); if (!x) return;
    if (k() >= x.t.length - 1) setK(0);
    const t0 = x.t[k()], w0 = performance.now();
    setPlay(true);
    const step = () => {
      const t = t0 + (performance.now() - w0) / 1000;
      let j = k(); while (j < x.t.length - 1 && x.t[j + 1] <= t) j++;
      setK(j); place(j);
      if (j >= x.t.length - 1) return stop();
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
  };
  onMount(() => {
    if (!renderer) return;
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    cv.appendChild(renderer.domElement);
    controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, 0, 0.35);
    controls.addEventListener("change", draw);
    controls.update();
    const ro = new ResizeObserver(fit);
    ro.observe(cv);
    fit();
    onCleanup(() => { ro.disconnect(); stop(); controls?.dispose(); renderer.dispose(); });
  });
  return <div class="plot robot3d" ref={host}>
    <div class="r3canvas" ref={cv} />
    <Show when={renderer} fallback={<p class="muted pempty">3D drawing is not available on this computer (no WebGL).</p>}>
      <Show when={!err()} fallback={<p class="muted pempty">{err()}</p>}>
        <Show when={f()} fallback={<p class="muted pempty r3wait">Run the model to see the robot.</p>}>{x =>
          <div class="armbar">
            <button class="btn sm armplay" onClick={() => (play() ? stop() : start())}>{play() ? "Pause" : "Play"}</button>
            <input type="range" class="armtime" min="0" max={x().t.length - 1} value={k()} onInput={e => { stop(); setK(+e.currentTarget.value); place(k()); }} />
            <span class="mono small">t = {num(x().t[k()], 3)} s</span>
          </div>}
        </Show>
      </Show>
    </Show>
  </div>;
};
