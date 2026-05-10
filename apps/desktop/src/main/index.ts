import { app, BrowserWindow } from "electron";
import { createMainWindow } from "./window";
import { loadNative } from "./native";
import { registerIpcHandlers } from "./ipc";
import { createTray } from "./tray";
import { RecSidecar } from "./sidecar";

app.whenReady().then(async () => {
  const native = await loadNative();
  const main = await createMainWindow();

  const sidecar = new RecSidecar();
  try {
    await sidecar.start();
    console.log("[TableFlow] Sidecar started");
  } catch (err) {
    console.warn("[TableFlow] Sidecar failed to start:", err);
  }

  registerIpcHandlers(main, native, sidecar);
  createTray();

  app.on("before-quit", async () => {
    await sidecar.shutdown();
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", async () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    const native = await loadNative();
    const main = await createMainWindow();
    const sidecar = new RecSidecar();
    try {
      await sidecar.start();
    } catch (err) {
      console.warn("[TableFlow] Sidecar failed to start:", err);
    }
    registerIpcHandlers(main, native, sidecar);
  }
});
