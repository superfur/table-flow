# TableFlow 使用细则

> 版本：0.1.0 (MVP) · 最后更新：2026-05-10

---

## 1. 环境要求

| 依赖 | 最低版本 | 说明 |
|------|---------|------|
| Node.js | ≥ 20 | 推荐 v22 |
| Yarn | 1.22.x | 包管理器 |
| Rust | 1.86 | 见 `rust-toolchain.toml` |
| Electron | 33+ | 自动安装 |
| macOS | 13+ | 开发环境 |
| Windows | 10+ | DXGI 截图（CI 验证） |

---

## 2. 安装与构建

### 2.1 克隆与依赖安装

```bash
git clone <repo-url> && cd table-flow
yarn install
```

### 2.2 Rust 编译

```bash
# 编译全部 Rust crates（使用 mock 后端，无需 ONNX Runtime）
cargo build --workspace

# 如需启用真实 ONNX 推理
ORT_LIB_LOCATION=./deps/onnxruntime-osx-arm64-1.22.0 \
  cargo build -p tf-inference --features real-onnx
```

### 2.3 Electron 构建

```bash
yarn workspace @table-flow/desktop build
```

---

## 3. 运行

### 3.1 开发模式

```bash
# 启动 Electron 开发服务器（HMR）
yarn workspace @table-flow/desktop dev
```

开发模式下：
- Renderer 加载 `http://localhost:5173`（Vite HMR）
- Main Process 自动重启
- DevTools 自动打开

### 3.2 生产模式

```bash
yarn workspace @table-flow/desktop build
yarn workspace @table-flow/desktop start
```

---

## 4. 测试

### 4.1 Rust 单元/集成测试

```bash
# 全部 256 个测试
cargo test --workspace

# 仅集成测试
cargo test -p tf-integration-tests

# 仅 sidecar e2e 测试
cargo test -p tf-rec -- sidecar_e2e

# 性能基准测试
cargo test -p tf-integration-tests -- bench_
```

### 4.2 Electron 集成测试

```bash
yarn workspace @table-flow/desktop test:integration
```

5 个测试：app ready、BrowserWindow 创建、IPC 通信、sidecar 模块、preload API。

### 4.3 TypeScript 类型检查

```bash
yarn workspace @table-flow/desktop typecheck
```

---

## 5. 架构概览

```
┌─────────────────────────────────────────────────┐
│  Electron Renderer (SolidJS)                     │
│  Dashboard / HUD Overlay / Settings              │
└───────────────┬─────────────────────────────────┘
                │ IPC (contextBridge)
┌───────────────┴─────────────────────────────────┐
│  Electron Main Process                           │
│  sidecar.ts (Node.js 子进程) → rec-sidecar/      │
│  ipc.ts → native.ts (tf-napi mock/real)          │
└───────────────┬─────────────────────────────────┘
                │
┌───────────────┴─────────────────────────────────┐
│  Rust Crates (8 个)                               │
│  tf-core → tf-vision → tf-inference              │
│  tf-state → tf-rec → tf-table → tf-napi          │
│  tf-integration-tests                             │
└──────────────────────────────────────────────────┘
```

---

## 6. 推荐系统 (Sidecar)

### 6.1 工作原理

推荐引擎以 Node.js sidecar 子进程运行，通过 JSON-RPC 2.0 over stdin/stdout 通信：

```
Electron Main → fork() → node rec-sidecar/index.js
                    ↕ JSON-RPC (line-delimited)
               rec.recommend(input) → RecOutput
               rec.health() → { ok, version }
```

### 6.2 自动管理

- Electron 启动时自动 spawn sidecar
- 连续 3 次请求失败后自动重启
- App 退出时自动 shutdown
- 开发模式：sidecar 脚本路径自动解析到 `rec-sidecar/index.js`
- 生产模式：从 `resources/rec-sidecar/` 加载

### 6.3 手动测试 Sidecar

```bash
# 直接运行 sidecar
node rec-sidecar/index.js

# 在另一个终端发送 JSON-RPC 请求
echo '{"jsonrpc":"2.0","id":1,"method":"rec.health","params":{}}' | node rec-sidecar/index.js
```

---

## 7. 手牌历史

### 7.1 功能

- 每手牌自动记录：hand_id、hole_cards、community_cards、pot、action_history
- JSONL 格式持久化（append-only）
- 自动统计 VPIP / PFR / 胜率 / 盈亏

### 7.2 数据格式

每行一个 JSON 对象：

```json
{"hand_id":1,"table_id":"table-1","started_at_ms":1715300000000,"hole_cards":[...],"community_cards":[...],"pot_total":150.0,"actions":[...]}
```

### 7.3 SessionStats

Dashboard 页面顶部展示 8 格统计面板：
- Hands / Win Rate / Profit / VPIP / PFR / Biggest Pot / Total Pot / Hero Wins

---

## 8. 配置

### 8.1 Settings 面板

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| Theme | select | dark | UI 主题 |
| FPS per Table | range | 30 | 每桌帧率 (15-60) |
| Max Tables | number | 8 | 最大同时桌数 (1-8) |
| Hero Seat Override | number | auto | Hero 座位覆盖 (0-9) |

### 8.2 Rust 配置

- `InferenceConfig`: ONNX 模型路径、线程数、GPU 开关
- `SidecarConfig`: sidecar 脚本路径、请求超时、最大连续失败数
- `ManagerConfig`: 最大桌数、FPS、capture 后端

---

## 9. CI/CD

### 9.1 GitHub Actions

| Workflow | 触发条件 | 内容 |
|----------|---------|------|
| `ci-rust.yml` | push/PR to main | cargo check + clippy + test |
| `ci-electron.yml` | push/PR to main | yarn build + typecheck + integration tests |

### 9.2 本地验证

```bash
# 模拟 CI
cargo fmt --all -- --check
cargo clippy --workspace
cargo test --workspace
yarn workspace @table-flow/desktop typecheck
yarn workspace @table-flow/desktop test:integration
```

---

## 10. 目录结构

```
table-flow/
├── crates/                     # Rust workspace
│   ├── tf-core/               # 核心类型（Card, Street, ActionType）
│   ├── tf-vision/             # 视觉管线（capture, detection, matching）
│   ├── tf-inference/          # ONNX 推理（mock + real-onnx）
│   ├── tf-state/              # 状态机 + 动作推导 + 手牌历史
│   ├── tf-rec/                # 推荐引擎（sidecar + cache）
│   ├── tf-table/              # 多桌管理
│   ├── tf-napi/               # napi-rs 桥接
│   └── tf-integration-tests/  # 集成测试 + 基准
├── apps/desktop/              # Electron 应用
│   ├── src/main/              # 主进程（sidecar, ipc, window）
│   ├── src/renderer/          # SolidJS UI（Dashboard, HUD, Settings）
│   ├── src/preload/           # contextBridge
│   └── tests/                 # Electron 集成测试
├── rec-sidecar/               # Node.js 推荐引擎 sidecar
├── deps/                      # ONNX Runtime 二进制（gitignored）
├── .github/workflows/         # CI 配置
├── ARCHITECTURE.md            # 系统架构文档
├── ROADMAP.md                 # 开发路线图 + 进度
└── USAGE.md                   # 本文件
```

---

## 11. 已知限制

1. **ONNX 模型缺失**：当前使用 mock 推理后端，真实 ONNX 模型需手动放置到 `resources/models/`
2. **napi-rs 未启用**：`napi`/`napi-derive` 依赖仍为注释状态，Electron 使用 mock native addon
3. **Windows DXGI**：仅 CI 验证，开发以 macOS mock capture 为主
4. **准确率基准**（8.2.4/8.2.5）：需要真实扑克客户端帧 fixture，暂未覆盖
5. **Electron 打包**：未配置 electron-builder，需手动安装和运行

---

## 12. 故障排除

| 问题 | 解决方案 |
|------|---------|
| `cargo build` 失败 | 确认 `rust-toolchain.toml` 为 `1.86`：`rustup show` |
| ONNX 编译失败 | mock 模式无需 ONNX。真实模式需设置 `ORT_LIB_LOCATION` |
| Electron 启动白屏 | 开发模式需先启动 Vite：`yarn workspace @table-flow/desktop dev` |
| Sidecar 启动失败 | 检查 `rec-sidecar/index.js` 是否存在 |
| `yarn install` 失败 | 删除 `node_modules` 和 `yarn.lock` 后重试 |
| 测试超时 | Sidecar e2e 测试在无 Node.js 环境时自动跳过 |
