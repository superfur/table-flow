// TODO(detail-impl): 透明 Overlay 窗口创建与位置同步
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
  _config: OverlayConfig,
): Promise<OverlayWindow> {
  // TODO(detail-impl)
  throw new Error("createOverlayWindow not implemented");
}
