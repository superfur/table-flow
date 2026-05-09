// TODO(detail-impl): 完整 Vite 配置（Electron multi-target、preload、main、renderer 三入口）
// 当前只做 renderer-side 占位。
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import path from "node:path";

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
  build: {
    target: "esnext",
    outDir: "dist",
  },
});
