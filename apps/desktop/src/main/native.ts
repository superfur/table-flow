// TODO(detail-impl): 加载 tf-napi 编译产物 (tf-napi.node)
// 这里先定义 NativeAddon 的 TS 接口（与 tf-napi 公开 API 对齐）。

export interface TableConfig {
  tableId: string;
  windowTitle: string;
}

export interface NativeAddon {
  startCapture(config: TableConfig): Promise<void>;
  stopCapture(tableId: string): Promise<void>;
  discoverTables(): Promise<string[]>;
  getTableState(tableId: string): Promise<unknown>;
  calibrateTable(tableId: string): Promise<unknown>;

  onStateUpdate(cb: (event: unknown) => void): () => void;
  onRecommendation(cb: (event: unknown) => void): () => void;
  onError(cb: (event: unknown) => void): () => void;
}

export async function loadNative(): Promise<NativeAddon> {
  // TODO(detail-impl): require('../../../target/release/tf_napi.node')
  throw new Error("loadNative not implemented");
}
