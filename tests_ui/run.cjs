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
          if (c === "diagram") return window.__motor ? f.motor : a.parent === f.sub ? f.subdiagram : f.diagram;
          if (c === "new_model") return f.state;
          // The run ends before its start call returns (a model with no states takes 0.1 ms).
          if (c === "run") {
            window.__emit("sim-done", { gen: 1, ok: true, error: null, issues: [], stats: f.results.stats, truncated: false, files: [] });
            // With Log all signals the run also logs block outputs for the probe.
            if (window.__probeRun) { const pid = f.diagram.blocks.find(b => b.kind === "PID").id; return { ...f.started, chans: [...f.started.chans, { sink: pid, port: 0, elem: 0, name: "PID", probe: true }] }; }
            return f.started;
          }
          if (c === "apply") return { ...f.state, created: [999] };
          if (c === "value_at") return a.chans.map((_, k) => 0.5 + k);
          if (c === "step_marks") return { y0: 0, yf: 1, t10: 0.2, t90: 0.8, rise: 0.6, peak: 1.1, tpeak: 1.2, overshoot: 10, settle: 2.5 };
          if (c === "runs") return [{ id: 1, label: "Run 1 · 10:00:00" }, { id: 2, label: "Run 2 · 10:01:00" }];
          if (c === "run_series") return { t: f.series.t, y: a.names.map((_, k) => f.series.y[k] ?? f.series.y[0]) };
          if (c === "save_file") { window.__saved = a; return "/tmp/x"; }
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
  check("ribbon tabs", JSON.stringify(await p.$$eval(".tabs > button:not(.file)", els => els.map(e => e.textContent))) === JSON.stringify(["Model", "Simulate", "Analyze", "View", "Help"]));
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
    for (const t of ["Model", "Simulate", "Analyze", "View", "Help"]) {
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

  // 14. Every view and File page opens without errors.
  for (const v of ["Variables", "Scopes", "Results", "Issues", "Diagram"]) { await p.click(`.side button[title="${v}"]`); await p.waitForTimeout(150); }
  for (const k of ["print", "options", "roadmap", "about", "home"]) { await p.evaluate(() => document.querySelector(".tabs .file").click()); await p.waitForTimeout(100); await p.click(`.bs aside button:has-text("${{ print: "Print", options: "Options", roadmap: "Roadmap", about: "Updates", home: "Home" }[k]}")`); await p.waitForTimeout(200); }
  check("roadmap lists the planned releases", (await p.textContent(".bsmain")).length > 0);
  await p.click(".bs aside .back");
  for (let i = 0; i < 3 && (await p.evaluate(() => document.documentElement.dataset.theme)) !== "dark"; i++) await p.click(".theme");
  check("theme switches to dark", (await p.evaluate(() => document.documentElement.dataset.theme)) === "dark");
  check("every view opens without errors", errors.length === 0, errors.join(" | "));

  await b.close(); srv.close();
  console.log(failed ? `\n${failed} check(s) failed` : "\nall UI checks passed");
  process.exit(failed ? 1 : 0);
})();
