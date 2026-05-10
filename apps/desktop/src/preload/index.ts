import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("electronAPI", {
  onStateUpdate: (cb: (event: unknown) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: unknown) =>
      cb(data);
    ipcRenderer.on("stateUpdate", handler);
    return () => ipcRenderer.removeListener("stateUpdate", handler);
  },
  onRecommendationUpdate: (cb: (event: unknown) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: unknown) =>
      cb(data);
    ipcRenderer.on("recommendationUpdate", handler);
    return () => ipcRenderer.removeListener("recommendationUpdate", handler);
  },
  onError: (cb: (event: unknown) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: unknown) =>
      cb(data);
    ipcRenderer.on("error", handler);
    return () => ipcRenderer.removeListener("error", handler);
  },
  discoverTables: () => ipcRenderer.invoke("discoverTables"),
  startCapture: (config: { tableId: string; windowTitle: string }) =>
    ipcRenderer.invoke("startCapture", config),
  stopCapture: (tableId: string) =>
    ipcRenderer.invoke("stopCapture", tableId),
  getTableState: (tableId: string) =>
    ipcRenderer.invoke("getTableState", tableId),
  calibrateTable: (tableId: string) =>
    ipcRenderer.invoke("calibrateTable", tableId),
  shutdown: () => ipcRenderer.invoke("shutdown"),
  getRecommendation: (input: unknown) =>
    ipcRenderer.invoke("getRecommendation", input),
  sidecarHealth: () => ipcRenderer.invoke("sidecarHealth"),
  getSessionStats: () => ipcRenderer.invoke("getSessionStats"),
});
