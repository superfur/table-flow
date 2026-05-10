import { app, BrowserWindow } from "electron";
import * as path from "node:path";

export interface OverlayConfig {
  targetWindowTitle: string;
  tableId: string;
}

export interface OverlayWindow {
  readonly id: number;
  readonly tableId: string;
  close(): Promise<void>;
}

export async function createOverlayWindow(
  config: OverlayConfig,
): Promise<OverlayWindow> {
  const win = new BrowserWindow({
    width: 400,
    height: 300,
    transparent: true,
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
    focusable: false,
    webPreferences: {
      preload: path.join(__dirname, "../preload/index.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  win.setIgnoreMouseEvents(true);

  if (app.isPackaged) {
    await win.loadFile(path.join(__dirname, "../renderer/index.html"));
  } else {
    await win.loadURL("http://localhost:5173/overlay");
  }

  return {
    id: win.id,
    tableId: config.tableId,
    async close() {
      win.close();
    },
  };
}
