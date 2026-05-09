// TODO(detail-impl): app lifecycle / window manager / native addon loading
// 占位入口，确保 typecheck 通过。
import { createMainWindow } from "./window";
import { loadNative } from "./native";
import { registerIpcHandlers } from "./ipc";

export async function bootstrap(): Promise<void> {
  const native = await loadNative();
  const main = await createMainWindow();
  registerIpcHandlers(main, native);
}
