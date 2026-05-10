const { app, ipcMain, BrowserWindow } = require("electron");
const assert = require("node:assert/strict");

let results = [];
let failures = 0;

function test(name, fn) {
  results.push(
    fn()
      .then(() => console.log(`  ✅ ${name}`))
      .catch((e) => {
        failures++;
        console.error(`  ❌ ${name}: ${e.message}`);
      })
  );
}

function done() {
  Promise.all(results).then(() => {
    console.log(`\n${results.length - failures}/${results.length} passed`);
    app.exit(failures > 0 ? 1 : 0);
  });
}

app.whenReady().then(async () => {
  console.log("\nElectron integration tests\n");

  test("app is ready", async () => {
    assert.ok(app.isReady());
  });

  test("BrowserWindow creation + close", async () => {
    const win = new BrowserWindow({
      width: 800,
      height: 600,
      show: false,
      webPreferences: { offscreen: true },
    });
    assert.ok(win.id > 0);
    win.close();
  });

  test("ipcMain handle registration + response", async () => {
    ipcMain.handle("__test_ping", () => ({ status: "ok" }));
    const win = new BrowserWindow({
      width: 400,
      height: 300,
      show: false,
      webPreferences: {
        nodeIntegration: true,
        contextIsolation: false,
      },
    });
    await win.loadURL("about:blank");
    const result = await win.webContents.executeJavaScript(
      `require('electron').ipcRenderer.invoke('__test_ping')`
    );
    assert.deepEqual(result, { status: "ok" });
    ipcMain.removeHandler("__test_ping");
    win.close();
  });

  test("sidecar module can be imported", async () => {
    const fs = require("fs");
    const sidecarPath = require("path").join(__dirname, "..", "out", "main", "index.js");
    const content = fs.readFileSync(sidecarPath, "utf-8");
    assert.ok(content.includes("RecSidecar"), "RecSidecar class should exist in bundle");
  });

  test("preload exposes electronAPI", async () => {
    const fs = require("fs");
    const preloadPath = require("path").join(__dirname, "..", "out", "preload", "index.js");
    const content = fs.readFileSync(preloadPath, "utf-8");
    assert.ok(content.includes("electronAPI"), "electronAPI should be exposed");
    assert.ok(content.includes("discoverTables"), "discoverTables should exist");
    assert.ok(content.includes("getRecommendation"), "getRecommendation should exist");
  });

  done();
});
