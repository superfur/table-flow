import { app, BrowserWindow } from "electron";
import * as path from "node:path";

export interface MainWindow {
  readonly id: number;
  readonly win: BrowserWindow;
}

export async function createMainWindow(): Promise<MainWindow> {
  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    title: "TableFlow",
    backgroundColor: "#171717",
    webPreferences: {
      preload: path.join(__dirname, "../preload/index.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (app.isPackaged) {
    await win.loadFile(path.join(__dirname, "../renderer/index.html"));
  } else {
    await win.loadURL("http://localhost:5173");
    win.webContents.openDevTools({ mode: "detach" });
  }

  return { id: win.id, win };
}
