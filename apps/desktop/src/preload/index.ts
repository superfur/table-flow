// TODO(detail-impl): contextBridge.exposeInMainWorld('electronAPI', { ... })
// 当前只声明 window.electronAPI 的 TS 形状供 renderer 使用。

export interface ElectronAPI {
  onStateUpdate(cb: (event: unknown) => void): () => void;
  onRecommendationUpdate(cb: (event: unknown) => void): () => void;
  discoverTables(): Promise<string[]>;
  startCapture(config: { tableId: string; windowTitle: string }): Promise<void>;
  stopCapture(tableId: string): Promise<void>;
  getTableState(tableId: string): Promise<unknown>;
  calibrateTable(tableId: string): Promise<unknown>;
}

declare global {
  interface Window {
    electronAPI: ElectronAPI;
  }
}

export {};
