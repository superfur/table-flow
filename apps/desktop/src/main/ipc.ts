import { ipcMain } from "electron";
import type { MainWindow } from "./window";
import type { NativeAddon } from "./native";
import type { RecSidecar, RecInput } from "./sidecar";

export function registerIpcHandlers(
  main: MainWindow,
  native: NativeAddon,
  sidecar: RecSidecar,
): void {
  ipcMain.handle("discoverTables", async () => {
    return native.discoverTables();
  });

  ipcMain.handle("startCapture", async (_event, config: { tableId: string; windowTitle: string }) => {
    await native.startCapture(config);
  });

  ipcMain.handle("stopCapture", async (_event, tableId: string) => {
    await native.stopCapture(tableId);
  });

  ipcMain.handle("getTableState", async (_event, tableId: string) => {
    return native.getTableState(tableId);
  });

  ipcMain.handle("calibrateTable", async (_event, tableId: string) => {
    return native.calibrateTable(tableId);
  });

  ipcMain.handle("shutdown", async () => {
    await native.shutdown();
  });

  ipcMain.handle(
    "getRecommendation",
    async (_event, input: RecInput) => {
      return sidecar.recommend(input);
    },
  );

  ipcMain.handle("sidecarHealth", async () => {
    return sidecar.health();
  });

  ipcMain.handle("getSessionStats", async () => {
    return native.getSessionStats?.() ?? {
      totalHands: 0,
      handsWithHero: 0,
      heroWins: 0,
      heroNet: 0,
      vpip: 0,
      pfr: 0,
      winRate: 0,
      totalPot: 0,
      biggestPot: 0,
    };
  });

  native.onStateUpdate((event) => {
    main.win.webContents.send("stateUpdate", event);
  });

  native.onRecommendation((event) => {
    main.win.webContents.send("recommendationUpdate", event);
  });

  native.onError((event) => {
    main.win.webContents.send("error", event);
  });
}
