// UI tests: the built window (ui/dist) in Chromium with the engine's real output for the DC motor
// example (cargo run -p datum-core --example dump > tests_ui/fixture.json) behind a mocked Tauri
// bridge. Run: npm run test:ui (or npm run build:ui && node tests_ui/run.cjs).
const { chromium } = require("playwright");
const fs = require("fs"), http = require("http"), path = require("path");
const root = path.join(__dirname, "../ui/dist"), fix = JSON.parse(fs.readFileSync(path.join(__dirname, "fixture.json")));
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".html": "text/html" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : "  " + info}`); if (!ok) failed++; };

(async () => {
  await new Promise(ok => srv.listen(0, ok));
  const url = `http://localhost:${srv.address().port}/`;
  const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on("pageerror", e => { errors.push(e.message); if (process.env.DEBUG) console.log(e.stack); });
  await p.addInitScript(f => {
    const opts = { autosave: 0, beta: false, theme: "light", openLast: false, solver: "rk45", stop: 10, snap: true, grid: true, paper: "A4", landscape: true, company: "" };
    const handlers = {};
    const m = { state: f.state, library: f.library, examples_list: f.examples_list, options: opts, startup_file: null, recovery: null, check_update: null, recent: [], version: "test", check: [], results: f.results, series: f.series, sweep_series: [], rates: f.rates };
    window.__calls = [];
    window.__emit = (e, x) => (handlers[e] ?? []).forEach(h => h({ payload: x }));
    window.__TAURI__ = {
      core: {
        invoke: async (c, a) => {
          window.__calls.push([c, a]);
          if (c === "diagram") {
            if (window.__motor) return f.motor;
            if (window.__xy) return { ...f.diagram, blocks: f.diagram.blocks.map(b => b.kind === "Scope" ? { ...b, kind: window.__xy } : b) };
            if (window.__dcblock) return { ...f.diagram, blocks: f.diagram.blocks.map(b => b.id === f.sub ? { ...b, kind: "DCMotor", sub: false } : b) };
            if (a.parent === f.sub) return f.subdiagram;
            // A masked DC motor subsystem (0.2.0 checks).
            if (window.__masked) return { ...f.diagram, blocks: f.diagram.blocks.map(b => b.id === f.sub ? { ...b, label: "DC motor", params: { R: "1", L: "0.5" }, mask: { params: [{ key: "R", label: "Resistance R", def: "1", unit: "Ω", hint: "", choices: [] }, { key: "L", label: "Inductance L", def: "0.5", unit: "H", hint: "", choices: [] }], icon: "DC motor", help: "A DC motor.", presets: [{ name: "Small", values: [["R", "2"], ["L", "0.1"]] }] } } : b) };
            return f.diagram;
          }
          if (c === "automation") return { cli: "C:\\Users\\me\\AppData\\Local\\Datum Sim\\datum-sim-cli.exe", py: null, model: "C:\\models\\motor.dsim" };
          if (c === "myblocks") return [{ file: "/blocks/Motor.dsimblock", name: "Motor (saved)", what: "A saved motor." }];
          if (c === "save_myblock" || c === "add_myblock") return c === "save_myblock" ? "/blocks/x.dsimblock" : { ...f.state, created: [998] };
          if (c === "new_model") return f.state;
          if (c === "lin_analysis") return f.lin;
          if (c === "trim_info") return [["Level"], 4];
          if (c === "robot_frames") return f.frames6;
          if (c === "nameplate") return { rs: 0.35, rr: 0.3, lls: 0.004, llr: 0.004, lm: 0.12843, pp: 2, i: 20.7, slip: 0.0267 };
          if (c === "spectrum") return { f0: 50, cycles: 4, t0: 0.12, t1: 0.2, dc: 0.3, harmonics: [2, 0, 0, 0, 0.2, 0, 0.1, 0, 0, 0], thd: 0.1118, f: [0, 12.5], mag: [0.3, 0] };
          if (c === "drive_tune") return { name: "Motor", data: { r: 1, ld: 0.5, lq: 0.5, kt: 0.01, j: 0.01, b: 0.1 }, tune: { iq: { p: a.wcI * 0.5, i: a.wcI, d: 0, n: 100 }, id: { p: a.wcI * 0.5, i: a.wcI, d: 0, n: 100 }, speed: { p: 200, i: 10000, d: 0, n: 100 }, current_margins: { wgc: a.wcI, pm: 90 }, speed_margins: { wgc: a.wcS, pm: 72 } } };
          if (c === "pid_plant") return { plant: { a: [[-2]], b: [[1]], c: [[2]], d: [[0]] }, cur: { p: 100, i: 200, d: 10, n: 100 }, name: "PID", wgc: 5 };
          if (c === "pid_design") return { pid: { p: 2 + a.pm / 100, i: 4, d: 0, n: 100 }, rule: "PI by loop shaping", note: "", report: { w: [1, 10], mag0: [0, -20], mag1: [3, -17], ph0: [-90, -120], ph1: [-80, -110], before: { pm: 40, gm_db: null, wgc: 5 }, after: { pm: a.pm, gm_db: null, wgc: a.wc }, step0: { t: [0, 1], y: [0, 1], unstable: false }, step1: { t: [0, 1], y: [0, 1], unstable: false } } };
          if (c === "trim") return { op: { name: "", t: 0, x: [3.53, 0, 1.41, 2], vars: [["H", 2]], note: "Level = 2" }, met: true, dmax: 1e-14, ymax: 1e-14, iters: 5, unmet: [], outputs: [["Level", 2]] };
          // The run ends before its start call returns (a model with no states takes 0.1 ms).
          if (c === "run") {
            const gen = window.__gen = (window.__gen ?? 0) + 1;
            window.__emit("sim-done", { gen, ok: true, error: null, issues: [], stats: f.results.stats, truncated: false, files: [] });
            // With Log all signals the run also logs block outputs for the probe.
            if (window.__probeRun) { const pid = f.diagram.blocks.find(b => b.kind === "PID").id; return { ...f.started, gen, chans: [...f.started.chans, { sink: pid, port: 0, elem: 0, name: "PID", probe: true }] }; }
            return { ...f.started, gen };
          }
          if (c === "apply") return { ...f.state, created: [999] };
          if (c === "value_at") return a.chans.map((_, k) => 0.5 + k);
          if (c === "step_marks") return { y0: 0, yf: 1, t10: 0.2, t90: 0.8, rise: 0.6, peak: 1.1, tpeak: 1.2, overshoot: 10, settle: 2.5 };
          if (c === "runs") return [{ id: 1, label: "Run 1 · 10:00:00" }, { id: 2, label: "Run 2 · 10:01:00" }];
          if (c === "run_series") return { t: f.series.t, y: a.names.map((_, k) => f.series.y[k] ?? f.series.y[0]) };
          if (c === "save_file") { window.__saved = a; return "/tmp/x"; }
          if (c === "find") { const s = (a.q || "").toLowerCase(); return f.diagram.blocks.filter(b => s && (b.name.toLowerCase().includes(s) || b.kind.toLowerCase().includes(s))).map(b => ({ id: b.id, name: b.name, kind: b.kind, parent: 0, path: "" })); }
          if (c in m) return m[c];
          return null;
        },
      },
      event: { listen: async (e, h) => { (handlers[e] ??= []).push(h); return () => {}; } },
    };
  }, fix);
  await p.goto(url);
  await p.waitForTimeout(800);

  // 1. File page: example tiles, orange sidebar.
  const tiles = await p.$$eval(".home .tile b", els => els.map(e => e.textContent));
  check("File page shows every worked example", fix.examples_list.every(e => tiles.includes(e.name)), JSON.stringify(tiles));
  check("File sidebar is the orange accent", (await p.$eval(".bs aside", el => getComputedStyle(el).backgroundColor)) === "rgb(194, 65, 12)");
  await p.click(".tile");
  await p.waitForTimeout(600);

  // 2. The diagram draws the engine's blocks, ports and routed wires.
  check("diagram draws every block", (await p.$$(".blk")).length === fix.diagram.blocks.length);
  check("diagram draws every wire", (await p.$$(".wire")).length === fix.diagram.wires.length);
  const nports = fix.diagram.blocks.reduce((a, b) => a + b.ins.length + b.outs.length, 0);
  check("every port is drawn", (await p.$$(".port")).length === nports);
  check("ribbon tabs", JSON.stringify(await p.$$eval(".tabs > button:not(.file)", els => els.map(e => e.textContent))) === JSON.stringify(["Model", "Simulate", "Analyze", "Control", "Modules", "View", "Help"]));
  check("library lists categories with planned blocks dimmed", (await p.$$(".libc")).length >= 8 && (await p.$$(".libi.off")).length >= 3);

  // 2b. Dragging a block from the library onto the diagram adds it there (0.1.0 bug on Windows: HTML drag and drop never reached the diagram).
  const libItem = await p.$('.libi:has-text("Gain")');
  const lb = await libItem.boundingBox();
  await p.mouse.move(lb.x + 30, lb.y + lb.height / 2); await p.mouse.down();
  await p.mouse.move(lb.x + 120, lb.y + 40, { steps: 4 });
  check("a ghost follows the pointer while dragging from the library", (await p.$$(".libghost")).length === 1);
  await p.mouse.move(800, 300, { steps: 8 }); await p.mouse.up();
  await p.waitForTimeout(200);
  const dropped = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).find(o => o.Add && o.Add.kind === "Gain");
  check("dragging from the library onto the diagram adds the block", !!dropped && dropped.Add.x % 10 === 0 && dropped.Add.y % 10 === 0, JSON.stringify(dropped));
  check("the ghost goes away after the drop", (await p.$$(".libghost")).length === 0);
  check("wires that split show a branch dot", (await p.$$(".wdot")).length >= 1);

  // 3. Selecting a block shows its parameters; the contextual tab appears without moving the ribbon (0.1 bug).
  const ribbonTop = await p.$eval(".ribbon", el => el.getBoundingClientRect().top);
  const pid = fix.diagram.blocks.find(b => b.kind === "PID");
  const box = async id => p.evaluate(i => { const g = [...document.querySelectorAll(".blk")].find(x => x.querySelector("title")?.textContent.startsWith(i)); const r = g.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; }, id);
  const pc = await box(pid.name + " (");
  await p.mouse.click(pc.x, pc.y);
  await p.waitForTimeout(200);
  check("inspector shows the block's parameters", (await p.textContent(".insp header")).includes("PID controller") && (await p.$$(".insp .iprow")).length >= 7);
  check("Block Tools tab shows for a selection", await p.$eval(".ctxgrp", el => getComputedStyle(el).visibility === "visible"));
  check("the ribbon does not move when the contextual tab appears", Math.abs((await p.$eval(".ribbon", el => el.getBoundingClientRect().top)) - ribbonTop) < 0.5);

  // 3b. Changing a parameter checks the model at once (errors show before Run).
  const nChecks = await p.evaluate(() => window.__calls.filter(c => c[0] === "check").length);
  await p.fill(".insp .iprow:nth-of-type(3) input", "Kp*2");
  await p.keyboard.press("Enter");
  await p.waitForTimeout(250);
  check("a parameter change runs a quiet check", (await p.evaluate(() => window.__calls.filter(c => c[0] === "check").length)) > nChecks);

  // 4. Dragging a block sends one Move with grid-snapped offsets.
  await p.mouse.move(pc.x, pc.y); await p.mouse.down(); await p.mouse.move(pc.x + 37, pc.y + 23, { steps: 5 }); await p.mouse.up();
  await p.waitForTimeout(200);
  const mv = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).find(o => o.Move);
  check("dragging a block lands it on the grid", mv && mv.Move.ids[0] === pid.id && (pid.x + mv.Move.dx) % 10 === 0 && (pid.y + mv.Move.dy) % 10 === 0 && mv.Move.dx > 0, JSON.stringify(mv));

  // 5. Dragging from an output port to an input port connects them.
  const ports = await p.evaluate(() => ({ outs: [...document.querySelectorAll(".port.out")].map(e => { const r = e.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }), ins: [...document.querySelectorAll(".port.in")].map(e => { const r = e.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }) }));
  const [ox, oy] = ports.outs[0], [ix, iy] = ports.ins[ports.ins.length - 1];
  await p.mouse.move(ox, oy); await p.mouse.down(); await p.mouse.move((ox + ix) / 2, (oy + iy) / 2, { steps: 4 }); await p.mouse.move(ix, iy, { steps: 4 }); await p.mouse.up();
  await p.waitForTimeout(200);
  check("dragging from an output to an input sends Connect", (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").some(c => c[1].op.Connect))));

  // 6. Double-click on empty space: quick insert adds the block typed.
  await p.mouse.dblclick(330, 760);
  await p.waitForTimeout(200);
  await p.keyboard.type("integr"); await p.keyboard.press("Enter");
  await p.waitForTimeout(200);
  const add = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).find(o => o.Add && o.Add.kind !== "Gain");
  check("quick insert adds the block typed", add && add.Add.kind === "Integrator", JSON.stringify(add));

  // 7. Delete removes the selection.
  await p.mouse.click(pc.x + 37, pc.y + 23);
  await p.keyboard.press("Delete");
  await p.waitForTimeout(150);
  check("Delete sends the selection", (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").some(c => c[1].op.Delete))));

  // 8. A run that finishes before its start call returns still ends (0.1 bug: it stayed "Running").
  await p.keyboard.press("F5");
  await p.waitForTimeout(700);
  check("a very short run ends with Done", (await p.textContent(".status")).includes("Done"), await p.textContent(".status"));
  check("the scopes open with plots after a run", (await p.$$(".dock .scope")).length >= 1 && (await p.$$(".dock .scope canvas")).length >= 1);

  // 8b. Scope tools (0.1.5): cursors, marks, compare, axes, pictures; the probe hint on a wire.
  const sc0 = ".dock .scope";
  const calls = async c => p.evaluate(n => window.__calls.filter(x => x[0] === n), c);
  await p.click(`${sc0} .cursbtn`);
  await p.waitForTimeout(300);
  check("cursors show a readout with A, B and B − A", (await p.$$(`${sc0} .cursread tr`)).length >= 3 && (await p.textContent(`${sc0} .cursread`)).includes("B − A"));
  check("cursor values come from the engine", (await calls("value_at")).length >= 2);
  await p.click(`${sc0} .marksbtn`);
  await p.waitForTimeout(200);
  check("step marks are asked of the engine", (await calls("step_marks")).length === 1 && await p.$eval(`${sc0} .marksbtn`, e => e.classList.contains("on")));
  await p.click(`${sc0} .cmpbtn`);
  await p.waitForTimeout(200);
  check("compare lists the earlier runs", (await p.textContent(".menu")).includes("Run 1"));
  await p.click('.menu button:has-text("Run 1")');
  await p.waitForTimeout(300);
  check("an earlier run is overlaid", (await calls("run_series")).some(c => c[1].run === 1) && await p.$eval(`${sc0} .cmpbtn`, e => e.classList.contains("on")));
  await p.click(`${sc0} .morebtn`);
  await p.waitForTimeout(150);
  await p.click('.menu button:has-text("stacked")');
  await p.waitForTimeout(150);
  const lay = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.SetParam && o.SetParam.key === "layout");
  check("the scope header fits on one line", await p.$eval(`${sc0} header`, e => e.scrollWidth <= e.clientWidth + 1 && e.clientHeight < 40));
  check("the stacked layout is saved in the Scope block", !!lay && lay.SetParam.value === "stacked", JSON.stringify(lay));
  await p.click(`${sc0} .morebtn`);
  await p.waitForTimeout(150);
  await p.click('.menu button:has-text("SVG")');
  await p.waitForTimeout(300);
  const svgOk = await p.evaluate(() => !!window.__saved && window.__saved.name.endsWith(".svg") && window.__saved.data.startsWith("<svg"));
  await p.click(`${sc0} .morebtn`);
  await p.waitForTimeout(150);
  await p.click('.menu button:has-text("high resolution")');
  await p.waitForTimeout(600);
  check("a high-resolution PNG is three times the size and not blank", await p.evaluate(async () => {
    const d = window.__saved; if (!d || !d.name.endsWith("3x.png")) return false;
    const img = new Image(); img.src = d.data; await img.decode();
    const c = document.createElement("canvas"); c.width = img.width; c.height = img.height; const g = c.getContext("2d"); g.drawImage(img, 0, 0);
    const px = g.getImageData(0, 0, c.width, c.height).data; let ink = 0; for (let i = 0; i < px.length; i += 4) if (px[i] < 200 || px[i + 1] < 200 || px[i + 2] < 200) ink++;
    const host = document.querySelector(".dock .scope .plot");
    return img.width >= 2.5 * host.clientWidth && ink > 1000;
  }));
  check("a scope saves as SVG", svgOk);
  await p.click(`${sc0} .cursbtn`);
  const wbox = await p.evaluate(() => { const g = document.querySelector(".wire .whit"); const r = g.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; });
  await p.mouse.move(wbox.x, wbox.y);
  await p.waitForTimeout(300);
  check("hovering a wire offers the signal probe", (await p.$$(".probetip")).length === 1 && (await p.textContent(".probetip")).includes("Log all signals"));
  await p.mouse.move(5, 5);
  await p.click('.tabs > button:text-is("Simulate")');
  await p.waitForTimeout(100);
  await p.click(".logall");
  await p.waitForTimeout(150);
  const ss = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.SetSim);
  check("Log all signals is a run setting", !!ss && ss.SetSim.sim.log_all === true, JSON.stringify(ss));
  // A run that logs every signal opens only the real scopes (0.1.5 bug: a probe signal opened an empty one).
  const nScopes = fix.diagram.blocks.filter(b => b.kind === "Scope").length;
  await p.evaluate(() => { window.__probeRun = true; document.querySelectorAll(".dock .scope .x").forEach(b => b.click()); });
  await p.waitForTimeout(200);
  await p.keyboard.press("F5");
  await p.waitForTimeout(700);
  check("probe signals do not open scopes of their own", (await p.$$(".dock .scope")).length === nScopes, String((await p.$$(".dock .scope")).length));
  await p.evaluate(() => { window.__probeRun = false; });
  await p.click('.tabs > button:text-is("Model")');

  // 8d. Masks and My blocks (0.2.0).
  await p.evaluate(() => { window.__masked = true; });
  const subB = fix.diagram.blocks.find(b => b.id === fix.sub);
  const sbx = await box(subB.name + " (");
  await p.mouse.click(sbx.x, sbx.y);
  // Any edit makes the window fetch the diagram again (now with the mask).
  await p.keyboard.press("ArrowRight");
  await p.waitForTimeout(400);
  await p.mouse.click(sbx.x, sbx.y);
  await p.waitForTimeout(250);
  check("a masked subsystem shows its own parameters in the Inspector", (await p.textContent(".insp header")).includes("(masked)") && (await p.textContent(".insp")).includes("Resistance R"));
  check("a masked subsystem offers its typical ratings", (await p.$$eval(".insp select.presets option", els => els.map(e => e.textContent))).includes("Small"));
  await p.fill('.insp .iprow:has-text("Resistance R") input', "3.3");
  await p.keyboard.press("Enter");
  await p.waitForTimeout(200);
  const mp = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.SetParam);
  check("a mask parameter is set like a block parameter", !!mp && mp.SetParam.id === fix.sub && mp.SetParam.key === "R" && mp.SetParam.value === "3.3", JSON.stringify(mp));
  await p.click(".insp .maskbtn");
  await p.waitForTimeout(200);
  await p.click(".dlg .mk-add");
  await p.waitForTimeout(100);
  const keys = await p.$$(".dlg .mk-key");
  await keys[keys.length - 1].fill("Ke");
  await p.click('.dlg .btn.pri');
  await p.waitForTimeout(250);
  const sm = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.SetMask);
  check("the mask editor saves the parameters", !!sm && sm.SetMask.id === fix.sub && sm.SetMask.mask.params.map(x => x.key).join(",") === "R,L,Ke" && sm.SetMask.mask.presets.length === 1, JSON.stringify(sm)?.slice(0, 300));
  await p.click(".insp .mybtn");
  await p.waitForTimeout(150);
  await p.click('.dlg .btn.pri');
  await p.waitForTimeout(250);
  check("Save to My blocks saves the subsystem", (await p.evaluate(() => window.__calls.filter(c => c[0] === "save_myblock"))).some(c => c[1].id === fix.sub));
  await p.evaluate(() => { window.__masked = false; });
  check("the Library lists My blocks", (await p.textContent(".lib")).includes("My blocks") && (await p.textContent(".lib")).includes("Motor (saved)"));
  await p.dblclick('.libi.mine');
  await p.waitForTimeout(250);
  check("adding a saved block pastes a copy of it", (await p.evaluate(() => window.__calls.filter(c => c[0] === "add_myblock"))).some(c => c[1].file === "/blocks/Motor.dsimblock"));

  // 8c. Wires and diagram tools (0.1.6).
  const lastOp = async pred => (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(pred);
  const scr = async (x, y) => p.evaluate(([x, y]) => { const g = document.querySelector(".dsvg > g"); const q = new DOMPoint(x, y).matrixTransform(g.getScreenCTM()); return { x: q.x, y: q.y }; }, [x, y]);
  const fb = fix.diagram.wires.find(w => w.path.length >= 6);
  // An inner segment no other wire shares (they may run together where a signal branches).
  const [sa, sb] = [fb.path[3], fb.path[4]];
  const mid = await scr((sa[0] + sb[0]) / 2, (sa[1] + sb[1]) / 2);
  const horiz = Math.abs(sa[1] - sb[1]) < 1e-9;
  await p.mouse.move(mid.x, mid.y); await p.mouse.down();
  await p.mouse.move(mid.x + (horiz ? 0 : 40), mid.y + (horiz ? 40 : 0), { steps: 5 }); await p.mouse.up();
  await p.waitForTimeout(200);
  const seg = await lastOp(o => o.SetPts && o.SetPts.id === fb.id);
  const ortho = seg && seg.SetPts.pts.every((q, i, a) => i === 0 || q[0] === a[i - 1][0] || q[1] === a[i - 1][1]);
  check("dragging a wire segment moves it sideways (bend points stay square)", !!seg && seg.SetPts.pts.length === fb.path.length - 2 && ortho, JSON.stringify(seg));
  // Alt-drag from a wire branches its signal to an input.
  // Away from the ports: the middle of the wire's longest segment.
  const w0 = fix.diagram.wires[0], src0 = w0.path[0], dst = fix.diagram.blocks.find(b => b.kind === "Scope" && b.ins.length);
  const li = w0.path.slice(1).map((q, i) => Math.hypot(q[0] - w0.path[i][0], q[1] - w0.path[i][1])).reduce((b, l, i, a) => (l > a[b] ? i : b), 0);
  const wmid = [(w0.path[li][0] + w0.path[li + 1][0]) / 2, (w0.path[li][1] + w0.path[li + 1][1]) / 2];
  const ws = await scr(wmid[0], wmid[1]), wt = await scr(dst.ins[0][0], dst.ins[0][1]);
  await p.keyboard.down("Alt"); await p.mouse.move(ws.x, ws.y); await p.mouse.down();
  await p.mouse.move(wt.x, wt.y, { steps: 6 }); await p.mouse.up(); await p.keyboard.up("Alt");
  await p.waitForTimeout(200);
  const br = await lastOp(o => o.Connect);
  check("Alt-dragging from a wire branches the same signal", !!br && br.Connect.from[0] === w0.from[0] && br.Connect.from[1] === w0.from[1] && br.Connect.to[0] === dst.id, JSON.stringify(br) + " " + JSON.stringify(src0));
  // Double-click a wire to name its signal.
  const wm = await scr(wmid[0], wmid[1]);
  await p.mouse.dblclick(wm.x, wm.y);
  await p.waitForTimeout(200);
  await p.fill(".wnamein", "speed demand");
  await p.keyboard.press("Enter");
  await p.waitForTimeout(200);
  const nm = await lastOp(o => o.NameWire);
  check("double-clicking a wire names its signal", !!nm && nm.NameWire.id === w0.id && nm.NameWire.name === "speed demand", JSON.stringify(nm));
  // Find, colour, note, distribute.
  await p.keyboard.press("Control+f");
  await p.waitForTimeout(150);
  await p.fill(".findpal input", "pid");
  await p.waitForTimeout(250);
  check("Find lists the blocks it matches", (await p.textContent(".findpal .pl")).includes("PID"));
  await p.keyboard.press("Enter");
  await p.waitForTimeout(400);
  check("choosing a hit selects the block", (await p.textContent(".insp header")).includes("PID controller"));
  await p.click('.ctxgrp button');
  await p.waitForTimeout(100);
  await p.click('.ribbon button:has-text("Block Colour")');
  await p.waitForTimeout(150);
  await p.click('.menu button:has-text("Blue")');
  await p.waitForTimeout(150);
  const col = await lastOp(o => o.SetColor);
  check("blocks can be coloured", !!col && col.SetColor.color === "blue" && col.SetColor.ids.length === 1, JSON.stringify(col));
  await p.click('.ribbon button:has-text("Note")');
  await p.waitForTimeout(150);
  check("Note adds a note block", !!(await lastOp(o => o.Add && o.Add.kind === "Note")));
  await p.keyboard.press("Escape");
  await p.evaluate(() => document.querySelector(".canvas").focus());
  await p.keyboard.press("Control+a");
  await p.waitForTimeout(100);
  await p.click('.ribbon button:has-text("Distribute ↔")');
  await p.waitForTimeout(150);
  const dist = await lastOp(o => o.Batch && o.Batch.every(x => x.Move));
  check("Distribute spaces the selected blocks evenly", !!dist && dist.Batch.length === fix.diagram.blocks.length, JSON.stringify(dist)?.slice(0, 200));
  await p.click('.tabs > button:text-is("Model")');

  // 9. Subsystems open with a path back.
  const sub = fix.diagram.blocks.find(b => b.kind === "Subsystem");
  const sc = await box(sub.name + " (");
  await p.mouse.dblclick(sc.x, sc.y);
  await p.waitForTimeout(400);
  check("double-click opens a subsystem", (await p.$$(".blk")).length === fix.subdiagram.blocks.length && (await p.textContent(".crumbs")).includes(sub.name));
  await p.click(".crumbs .up");
  await p.waitForTimeout(300);

  // 10. Help, Tell Me, shortcuts.
  const pc2 = await box(pid.name + " (");
  await p.mouse.click(pc2.x, pc2.y);
  await p.click(".insp header .help");
  await p.waitForTimeout(150);
  check("? card for a block shows its equations", (await p.textContent(".dlg")).includes("Parallel PID"));
  await p.keyboard.press("Escape");
  await p.keyboard.press("Control+k");
  await p.keyboard.type("bode");
  await p.waitForTimeout(150);
  check("Tell Me finds commands", (await p.textContent(".pal")).includes("Bode plot"));
  await p.keyboard.press("Escape");

  // 11. The ribbon never scrolls sideways.
  for (const w of [1280, 1600, 1000]) {
    await p.setViewportSize({ width: w, height: 900 });
    for (const t of ["Model", "Simulate", "Analyze", "Control", "Modules", "View", "Help"]) {
      await p.click(`.tabs > button:text-is("${t}")`);
      await p.waitForTimeout(80);
      const over = await p.$eval(".ribbon", el => el.scrollWidth - el.clientWidth);
      if (over > 1) check(`ribbon ${t} fits at ${w}px`, false, `overflow ${over}`);
    }
  }
  await p.setViewportSize({ width: 1440, height: 900 });

  // 12. Motors: glyphs, port names, and typical ratings that set every parameter in one edit (0.1.2).
  await p.evaluate(() => { window.__motor = true; document.querySelector(".tabs .file").click(); });
  await p.waitForTimeout(200);
  await p.click(".tile");
  await p.waitForTimeout(600);
  const mot = fix.motor.blocks.find(b => b.kind === "InductionSC");
  check("a motor block shows its port names", (await p.$$eval(".blk text", els => els.map(e => e.textContent))).includes("v_abc"));
  const mc = await box(mot.name + " (");
  await p.mouse.click(mc.x, mc.y);
  await p.waitForTimeout(200);
  const nopt = await p.$$eval(".insp select.presets option", els => els.length);
  check("a motor offers typical ratings", nopt === fix.library.blocks.find(d => d.kind === "InductionSC").presets.length + 1, String(nopt));
  const pr = fix.library.blocks.find(d => d.kind === "InductionSC").presets[3];
  await p.selectOption(".insp select.presets", pr.name);
  await p.waitForTimeout(200);
  const batch = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.Batch);
  check("choosing a rating sets every value in one undoable edit", !!batch && batch.Batch.length === pr.values.length && batch.Batch.every(o => o.SetParam && o.SetParam.id === mot.id), JSON.stringify(batch));
  // From nameplate (0.3.5): the circuit from the plate, applied as one edit of six parameters.
  await p.click(".insp .fromplate");
  await p.waitForTimeout(300);
  check("From nameplate shows the estimated circuit", (await p.textContent(".dlg")).includes("Magnetising Lm") && (await p.textContent(".dlg")).includes("128.4 mH"), (await p.textContent(".dlg")).slice(0, 300));
  await p.click(".dlg .btn.pri");
  await p.waitForTimeout(200);
  const npb = (await p.evaluate(() => window.__calls.filter(c => c[0] === "apply").map(c => c[1].op))).reverse().find(o => o.Batch);
  check("Apply sets Rs, Rr, Lls, Llr, Lm and p in one edit", npb?.Batch.length === 6 && npb.Batch.some(o => o.SetParam.key === "Lm" && o.SetParam.value === "0.12843"), JSON.stringify(npb));
  await p.evaluate(() => { window.__motor = false; });

  // 13. Sample-time colours (0.1.4): the View tab colours blocks by rate and shows a legend.
  await p.click('.tabs > button:text-is("View")');
  await p.waitForTimeout(100);
  await p.click(".ratechk");
  await p.waitForTimeout(300);
  check("sample-time colours mark the blocks", (await p.$$(".blk.rated")).length >= 1 && (await p.$$(".ratelegend span")).length >= 1, String((await p.$$(".blk.rated")).length));
  check("the legend names the continuous rate", (await p.textContent(".ratelegend")).includes("Continuous"));
  await p.click(".ratechk");
  await p.waitForTimeout(200);
  check("turning them off removes the colours", (await p.$$(".blk.rated")).length === 0 && (await p.$$(".ratelegend")).length === 0);

  // 13b. Help › Automation (0.2.1): commands for the open model, ready to copy.
  await p.click('.tabs > button:text-is("Help")');
  await p.waitForTimeout(100);
  await p.click('.ribbon button:has-text("Automation")');
  await p.waitForTimeout(300);
  const au = await p.textContent(".dlg");
  check("Help › Automation gives the command line for the open model", au.includes('"C:\\Users\\me\\AppData\\Local\\Datum Sim\\datum-sim-cli.exe" run C:\\models\\motor.dsim --out results.csv') && au.includes("sweep") && au.includes("import datum_sim"), au.slice(0, 300));
  await p.keyboard.press("Escape");
  await p.click('.tabs > button:text-is("Model")');

  // 13b2. Analysis points and Loop analysis (0.2.3).
  await p.click('.tabs > button:text-is("Control")');
  await p.waitForTimeout(100);
  await p.click('.ribbon button:has-text("Loop Analysis")');
  await p.waitForTimeout(500);
  const la = await p.textContent(".dlg");
  check("Loop analysis shows the loop's margins and closed-loop step", la.includes("Gain margin") && la.includes("Phase margin") && la.includes("Closed loop") && (await p.$$(".dlg .uplot")).length === 2, la.slice(0, 300));
  check("Loop analysis names the loop opening", (await p.$$eval(".dlg .lain option", o => o.map(x => x.textContent))).some(t => t.startsWith("Loop at")));
  await p.click('.dlg .dtabs button:has-text("Poles")');
  await p.waitForTimeout(200);
  check("the Poles tab draws the poles", (await p.$$(".dlg .xyplot path")).length >= 1);
  await p.click('.dlg .dtabs button:has-text("Nyquist")');
  await p.waitForTimeout(200);
  check("the Nyquist tab draws L(jω) and the −1 point", (await p.$$(".dlg .xyplot path")).length >= 3 && (await p.textContent(".dlg")).includes("encirclement"));
  await p.click('.dlg .dtabs button:has-text("Root locus")');
  await p.waitForTimeout(200);
  check("the root locus draws the poles as the gain varies", (await p.$$(".dlg .xyplot circle")).length > 20);
  await p.click('.dlg .dtabs button:has-text("Sensitivity")');
  await p.waitForTimeout(200);
  check("the Sensitivity tab plots S and T with Ms", (await p.$$(".dlg .uplot")).length === 1 && (await p.textContent(".dlg")).includes("Peak sensitivity Ms"));
  await p.keyboard.press("Escape");
  await p.click('.tabs > button:text-is("Model")');

  // 13b3. Trim (0.2.5): hold an output, find, save as an operating point (one SetOps edit).
  await p.click('.tabs > button:text-is("Control")');
  await p.click('.ribbon button:has-text("Trim")');
  await p.waitForTimeout(300);
  await p.fill(".trimhold", "2");
  await p.click(".trimgo");
  await p.waitForTimeout(300);
  check("Trim reports the operating point it found", (await p.textContent(".dlg")).includes("the model is at rest") && (await p.textContent(".dlg")).includes("Variable H"));
  await p.fill(".trimname", "2 m");
  await p.click(".trimsave");
  await p.waitForTimeout(300);
  const so = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "apply").pop();
  check("saving sends one SetOps edit with the named point", JSON.stringify(so?.[1] ?? {}).includes('"SetOps"') && JSON.stringify(so[1]).includes('"2 m"'), JSON.stringify(so));
  await p.click('.tabs > button:text-is("Model")');

  // 13b4. PID tuner (0.2.6): the sliders redesign; Apply is one Batch edit of the PID's gains.
  // The diagram has one PID: the tuner takes it without a selection.
  await p.click('.tabs > button:text-is("Control")');
  await p.click('.ribbon button:has-text("PID Tuner")');
  await p.waitForTimeout(400);
  check("the PID tuner shows the present and tuned gains", (await p.textContent(".dlg")).includes("Tuned") && (await p.textContent(".dlg")).includes("phase margin"), (await p.textContent(".dlg")).slice(0, 200));
  await p.$eval(".pidrob", el => { el.value = "70"; el.dispatchEvent(new Event("input", { bubbles: true })); });
  await p.waitForTimeout(300);
  check("moving Robustness redesigns", (await p.textContent(".dlg")).includes("After: phase margin 70°"));
  await p.click(".dlg .btn.pri");
  await p.waitForTimeout(300);
  const pa = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "apply").pop();
  check("Apply sends one Batch edit with the gains", JSON.stringify(pa?.[1] ?? {}).includes('"Batch"') && JSON.stringify(pa[1]).includes("2.7"), JSON.stringify(pa));
  await p.click('.tabs > button:text-is("Model")');

  // 13b5. Drive tuning (0.2.7): no motor → a hint; with a DC motor block → gains, Apply sets the PID.
  await p.click('.tabs > button:text-is("Control")');
  await p.click('.ribbon button:has-text("Drive Tuning")');
  await p.waitForTimeout(200);
  check("drive tuning without a motor block says what it needs", ((await p.textContent("#toast").catch(() => "")) ?? "").includes("no DC, BLDC or PMSM motor"));
  await p.evaluate(() => { window.__dcblock = true; });
  // Any edit makes the window fetch the diagram again (now with a DC motor block).
  await p.click('.tabs > button:text-is("Model")');
  await p.click(".canvas", { position: { x: 5, y: 5 } }).catch(() => {});
  await p.keyboard.press("Control+a");
  await p.click('.ribbon button:has-text("Flip")');
  await p.waitForTimeout(300);
  await p.click('.tabs > button:text-is("Control")');
  await p.click('.ribbon button:has-text("Drive Tuning")');
  await p.waitForTimeout(300);
  const dt = await p.textContent(".dlg").catch(() => "");
  check("drive tuning shows the current and speed PIs", dt.includes("Current PIs") && dt.includes("Speed PI") && dt.includes("phase margin 90"), dt.slice(0, 200));
  if (dt) {
    await p.selectOption(".dtsp", { label: "PID" });
    await p.click(".dlg .btn.pri");
    await p.waitForTimeout(300);
    const dta = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "apply").pop();
    check("Apply sets the chosen PID's gains in one edit", JSON.stringify(dta?.[1] ?? {}).includes('"Batch"') && JSON.stringify(dta[1]).includes("10000"), JSON.stringify(dta));
  }
  await p.evaluate(() => { window.__dcblock = false; });
  await p.click('.tabs > button:text-is("Model")');

  // 13b6. Live tuning (0.2.8): add a variable, move its slider while running → live_set; Keep → SetVars.
  await p.click('.tabs > button:text-is("Control")');
  await p.click('.ribbon button:has-text("Live Tuning")');
  await p.waitForTimeout(150);
  check("the Live panel opens", (await p.$$(".livepanel")).length === 1);
  await p.selectOption(".laddvar", "Kp");
  await p.waitForTimeout(200);
  const lt = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "apply").pop();
  check("adding a variable marks it tunable with a range", JSON.stringify(lt?.[1] ?? {}).includes('"tune":true') && JSON.stringify(lt[1]).includes('"hi":200'), JSON.stringify(lt));
  await p.click('.ribbon button:has-text("Live Tuning")');
  await p.click('.tabs > button:text-is("Simulate")');
  await p.selectOption(".pacesel", "1");
  await p.waitForTimeout(150);
  const pce = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "apply").pop();
  check("Pacing sets the run's pace", JSON.stringify(pce?.[1] ?? {}).includes('"pace":1'), JSON.stringify(pce));

  // 13b7. Spectrum and THD (0.3.1), after a run.
  await p.click('.tabs > button:text-is("Model")');
  await p.click('.canvas', { position: { x: 700, y: 500 } });
  await p.click('.tabs > button:text-is("Model")');
  await p.keyboard.press("F5");
  await p.waitForTimeout(700);
  await p.waitForTimeout(400);
  await p.click('.tabs > button:text-is("Analyze")');
  if (process.env.DEBUG) console.log("status:", await p.textContent(".status"));
  await p.click('.ribbon button:has-text("Spectrum")');
  await p.waitForTimeout(300);
  const sp = await p.textContent(".dlg");
  check("Spectrum shows the THD, the fundamental and the largest harmonics", sp.includes("11.18 %") && sp.includes("50 Hz") && sp.includes("h5 10 %") && (await p.$$(".dlg .specplot rect")).length === 10, sp.slice(0, 300));
  await p.keyboard.press("Escape");
  await p.click('.tabs > button:text-is("Model")');

  // 13b7b. An XY graph (and the Arm view, drawn the same way) opens in the dock without errors
  // (0.4.0 bug: the plot's resize observer had no plot area to watch).
  await p.evaluate(() => { window.__xy = "XY"; });
  await p.click('.side button[title="Diagram"]');
  await p.click('.tabs > button:text-is("Model")');
  const xb = await box(pid.name + " (");
  await p.mouse.click(xb.x, xb.y);
  await p.click('.ribbon button:has-text("Flip")');
  await p.waitForTimeout(300);
  const e0 = errors.length;
  await p.evaluate(() => document.querySelectorAll(".dock .scope .x").forEach(b => b.click()));
  await p.waitForTimeout(200);
  await p.keyboard.press("F5");
  await p.waitForTimeout(700);
  const xt = await p.textContent("#toast").catch(() => "") ?? "";
  check("an XY graph opens in the scopes dock without errors", errors.length === e0 && !xt.includes("Error") && (await p.$$(".dock .plot.xy")).length >= 1, errors.slice(e0).join(" | "));
  // The 3D view (0.4.3) opens the same way, draws with WebGL and shows its play bar.
  await p.evaluate(() => { window.__xy = "RobotView"; document.querySelectorAll(".dock .scope .x").forEach(b => b.click()); });
  await p.click('.ribbon button:has-text("Flip")');
  await p.waitForTimeout(300);
  await p.keyboard.press("F5");
  await p.waitForTimeout(1500);
  if (process.env.DEBUG) { await p.screenshot({ path: process.env.DEBUG }); console.log("DOCK", await p.evaluate(() => document.querySelector(".dock")?.innerHTML.slice(0, 600))); }
  const r3 = await p.$$(".dock .robot3d canvas");
  const xt3 = await p.textContent("#toast").catch(() => "") ?? "";
  const r3d = await p.evaluate(() => { const h = document.querySelector(".dock .robot3d"); return h ? [h.dataset.parts, h.dataset.failed, h.dataset.why] : null; });
  // The real app's content security policy must let the window fetch its own files (the meshes);
  // the browser here has no such policy, so check the setting itself (0.4.3 bug).
  const csp = JSON.parse(fs.readFileSync(path.join(__dirname, "../src-tauri/tauri.conf.json"))).app.security.csp;
  check("the app's security policy lets the window load its own robot meshes", /connect-src[^;]*'self'/.test(csp), csp);
  check("the 3D view draws the 6-joint arm's own parts", !!r3d && +r3d[0] >= 6 && r3d[1] === "0", JSON.stringify(r3d));
  check("the 3D view opens in the dock with a WebGL canvas and its play bar", r3.length >= 1 && (await p.$$(".dock .robot3d .armplay")).length >= 1 && !xt3.includes("Error"), xt3 + " " + String(r3.length));
  await p.evaluate(() => { window.__xy = false; });

  // 13b8. Modules (0.3.6): the robot arm group opens its worked example.
  await p.click('.tabs > button:text-is("Modules")');
  await p.click('.ribbon button:has-text("Robot Arm Example")');
  await p.waitForTimeout(300);
  const nmr = (await p.evaluate(() => window.__calls)).filter(c => c[0] === "new_model").pop();
  check("Modules › Robot Arm Example opens the robot arm model", JSON.stringify(nmr?.[1] ?? {}).includes("robotarm"), JSON.stringify(nmr));
  check("the library has the robot arm blocks", fix.library.blocks.filter(b => b.cat === "Robot arm").length === 7);
  await p.click('.tabs > button:text-is("Model")');

  // 13c. Planned features (0.2.2): dimmed on their tab with the release; a greyed dialog; a switch hides them.
  await p.click('.tabs > button:text-is("Model")');
  await p.waitForTimeout(150);
  const pl = await p.$$('.ribbon .rb.planned');
  check("the planned features show their release", pl.length >= 1 && /\d+\.\d+/.test(await p.textContent('.ribbon .rb.planned .pv')), String(pl.length));
  await p.evaluate(() => document.querySelector(".ribbon .rb.planned").click());
  await p.waitForTimeout(200);
  check("a planned feature opens the dialog it will have, disabled", (await p.textContent(".dlg .pband")).includes("Available in") && (await p.$$(".dlg fieldset:not([disabled])")).length === 0);
  await p.click(".dlg .x");
  await p.click('.tabs > button:text-is("View")');
  await p.click(".plannedchk");
  await p.click('.tabs > button:text-is("Model")');
  await p.waitForTimeout(150);
  check("View › Planned features hides them", (await p.$$('.ribbon .rb.planned')).length === 0);
  await p.click('.tabs > button:text-is("View")');
  await p.click(".plannedchk");
  await p.click('.tabs > button:text-is("Model")');

  // 14. Every view and File page opens without errors.
  for (const v of ["Variables", "Scopes", "Results", "Issues", "Diagram"]) { await p.click(`.side button[title="${v}"]`); await p.waitForTimeout(150); }
  for (const k of ["print", "options", "roadmap", "about", "home"]) { await p.evaluate(() => document.querySelector(".tabs .file").click()); await p.waitForTimeout(100); await p.click(`.bs aside button:has-text("${{ print: "Print", options: "Options", roadmap: "Roadmap", about: "Updates", home: "Home" }[k]}")`); await p.waitForTimeout(200); if (k === "roadmap") check("roadmap lists the planned releases and every planned feature", (await p.textContent(".bsmain")).includes("Feature by feature")); }
  await p.click(".bs aside .back");
  for (let i = 0; i < 3 && (await p.evaluate(() => document.documentElement.dataset.theme)) !== "dark"; i++) await p.click(".theme");
  check("theme switches to dark", (await p.evaluate(() => document.documentElement.dataset.theme)) === "dark");
  check("every view opens without errors", errors.length === 0, errors.join(" | "));

  await b.close(); srv.close();
  console.log(failed ? `\n${failed} check(s) failed` : "\nall UI checks passed");
  process.exit(failed ? 1 : 0);
})();
