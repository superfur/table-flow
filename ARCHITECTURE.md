# TableFlow 系统架构设计文档

> 专业级线上德州扑克实时辅助系统 — 完整技术架构

---

## 目录

- [1. 系统定位](#1-系统定位)
- [2. 核心架构原则](#2-核心架构原则)
- [3. 系统总体架构](#3-系统总体架构)
- [4. 技术栈与依赖](#4-技术栈与依赖)
- [5. 目录结构](#5-目录结构)
- [6. 线程模型与多进程设计](#6-线程模型与多进程设计)
- [7. Frame Processing Pipeline](#7-frame-processing-pipeline)
- [8. Vision Core 模块详细设计](#8-vision-core-模块详细设计)
- [9. ONNX 推理架构](#9-onnx-推理架构)
- [10. OCR 模块 (PaddleOCR)](#10-ocr-模块-paddleocr)
- [11. 状态机架构](#11-状态机架构)
- [12. 推荐引擎集成](#12-推荐引擎集成)
- [13. IPC 协议设计](#13-ipc-协议设计)
- [14. Overlay HUD 架构](#14-overlay-hud-架构)
- [15. Electron 架构](#15-electron-架构)
- [16. 多桌并发方案](#16-多桌并发方案)
- [17. 完整数据模型](#17-完整数据模型)
- [18. 性能优化策略](#18-性能优化策略)
- [19. 错误处理与健壮性](#19-错误处理与健壮性)
- [20. MVP 阶段拆分](#20-mvp-阶段拆分)
- [21. 推荐开发顺序](#21-推荐开发顺序)
- [21.A Hand History Replay](#21a-hand-history-replay行动历史回放)
- [21.B 多主题 / 多客户端适配](#21b-多主题--多客户端适配)
- [21.C 反作弊兼容性约束](#21c-反作弊兼容性约束)
- [21.D 测试与基准](#21d-测试与基准)
- [22. 未来扩展路线图](#22-未来扩展路线图)
- [23. 风险分析](#23-风险分析)

---

## 1. 系统定位

TableFlow 是一个**生产级实时德州扑克辅助系统**，核心定位：

```
实时桌面牌桌状态解析 + GTO/EV 推荐系统
```

### 1.1 核心能力

| 能力 | 描述 |
|------|------|
| 手牌识别 | 实时检测玩家手牌（2张） |
| 公共牌识别 | 检测 Flop / Turn / River 公共牌（0-5张） |
| 底池追踪 | 实时追踪主池与边池变化 |
| 筹码追踪 | 各座位筹码量变化检测 |
| 动作推导 | 通过状态变化推导玩家动作（非 OCR） |
| 回合检测 | Preflop / Flop / Turn / River / Showdown |
| 下注额追踪 | 跟踪各玩家当前下注额 |
| 座位映射 | 玩家座位状态、庄家位置、活跃状态 |
| GTO 推荐 | 基于完整状态输入推荐引擎，输出动作 / EV / 置信度 |
| HUD 叠加 | Overlay 实时显示推荐结果与统计数据 |

### 1.2 性能指标

| 指标 | 目标值 |
|------|--------|
| 端到端延迟 | < 100ms（Frame Capture → HUD Display） |
| 状态识别准确率 | > 98% |
| 卡牌识别准确率 | > 99.5% |
| 多桌支持 | 4-8 桌并发 |
| CPU 占用 | < 15%（8桌全开） |
| 内存占用 | < 500MB（8桌全开） |
| 稳定运行时间 | 7x24 小时 |
| 帧处理吞吐 | 30-60 FPS per table |

---

## 2. 核心架构原则

### 2.1 状态推导优先（State Machine-First）

**禁止方案（OCR-first）：**

```
截图 → OCR 识别文字 → 解析状态 → 推荐
```

- OCR 延迟高、准确率不稳定
- 文字识别受分辨率/主题/字体影响大
- 无法处理动态遮挡、动画过渡

**强制方案（State Derivation）：**

```
Frame Capture → Feature Extraction → Table State Diff → Event Reconstruction → State Machine Update → Recommendation
```

核心思想：**通过视觉特征变化推导游戏状态，而非文字识别**。

示例：

```
错误: OCR 识别 "CALL $30" 按钮 → 得出 call 动作
正确:
  Frame N:   Seat3 stack = 120, pot = 30
  Frame N+1: Seat3 stack = 90,  pot = 60
  推导: Seat3 call 30 → 动作 = Call, 金额 = 30
```

### 2.2 架构分层原则

1. **视觉层纯粹**：Vision Core 只输出特征，不做业务判断
2. **状态层确定**：State Machine 是唯一状态真相来源（Single Source of Truth）
3. **推荐层无状态**：Recommendation Engine 接受状态快照，输出推荐
4. **UI 层只读**：Overlay 从不修改状态，只展示

### 2.3 模块边界原则

```
模块间通过明确的 struct / enum / channel 通信
禁止跨模块共享可变状态
每个模块可独立测试
```

---

## 3. 系统总体架构

### 3.1 分层架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Presentation Layer                                │
│                                                                         │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
│  │  HUD Overlay   │  │  Config Panel  │  │  Stats Window  │            │
│  │  (SolidJS)     │  │  (SolidJS)     │  │  (SolidJS)     │            │
│  │  Transparent   │  │  Settings      │  │  History       │            │
│  │  Click-through │  │  Calibration   │  │  Analytics     │            │
│  └────────────────┘  └────────────────┘  └────────────────┘            │
│                                                                         │
│                    Electron Renderer Process                             │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ Electron IPC
┌──────────────────────────────┼──────────────────────────────────────────┐
│                              │     Application Layer (Electron Main)    │
│  ┌───────────────────────────┼──────────────────────────────────┐       │
│  │                           ▼                                  │       │
│  │  ┌─────────────────────────────────────────────────────────┐ │       │
│  │  │              Rust Native Module (napi-rs)               │ │       │
│  │  │                                                         │ │       │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │ │       │
│  │  │  │ TableManager │  │ EventRouter  │  │ IPCBridge    │ │ │       │
│  │  │  │ Multi-table  │  │ Pub/Sub      │  │ napi-exports │ │ │       │
│  │  │  │ Coordination │  │ Event filter │  │ Type marsh.  │ │ │       │
│  │  │  └──────────────┘  └──────────────┘  └──────────────┘ │ │       │
│  │  └─────────────────────────────────────────────────────────┘ │       │
│  └──────────────────────────────────────────────────────────────┘       │
│                                                                         │
│                    Electron Main Process                                 │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ Tokio mpsc / broadcast channels
┌──────────────────────────────┼──────────────────────────────────────────┐
│                              │     Core Logic Layer (Rust / Tokio)      │
│                              ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    State Machine (per table)                      │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────────────┐ │   │
│  │  │ TableState │  │ EventLog   │  │ ActionReconstructor       │ │   │
│  │  │ Snapshot   │  │ History    │  │ Stack Diff / Pot Diff      │ │   │
│  │  └────────────┘  └────────────┘  └────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Recommendation Engine                          │   │
│  │  TypeScript SDK (via Node.js embed or ported to Rust)            │   │
│  │  Input: holeCards, communityCards, pot, toCall, minRaise, ...   │   │
│  │  Output: action, amount, confidence, distribution                │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ crossbeam channels / Rayon scope
┌──────────────────────────────┼──────────────────────────────────────────┐
│                              │     Vision Layer (Rust / OpenCV / ONNX)  │
│                              ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Vision Pipeline (per table)                    │   │
│  │                                                                  │   │
│  │  Frame Capture ──→ Preprocess ──→ Feature Extract ──→ Classify │   │
│  │       │                │               │                 │       │   │
│  │  DXGI Desktop     Resize/Norm     ROI Extract       ONNX/Card   │   │
│  │  Duplication      Color Space     Template Match    Detection   │   │
│  │  Window Track     Frame Diff      Contour Analysis              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Shared Inference Pool                          │   │
│  │  ONNX Runtime Session Pool (GPU/CPU)                             │   │
│  │  PaddleOCR Instance Pool (auxiliary number recognition)          │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ Windows API
┌──────────────────────────────┼──────────────────────────────────────────┐
│                              │     OS Abstraction Layer                 │
│                              ▼                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │ DXGI Desktop │  │ Win32 Window │  │ DWM          │                  │
│  │ Duplication  │  │ Management   │  │ Composition  │                  │
│  └──────────────┘  └──────────────┘  └──────────────┘                  │
│                     Windows OS                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 数据流图（End-to-End）

```
[Poker Client Window]
       │
       │ DXGI Desktop Duplication
       ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ Frame Capture │───→│ Preprocessor │───→│ ROI Extractor│
│ 30-60 FPS    │    │ Resize/Norm  │    │ Region Split │
│ Per Table    │    │ Color Conv   │    │ Cache Check  │
└──────────────┘    └──────────────┘    └──────┬───────┘
                                               │
                      ┌────────────────────────┼────────────────┐
                      │                        │                │
                      ▼                        ▼                ▼
              ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
              │Card Detector │    │Stack Tracker │    │  Pot Tracker │
              │Template/ONNX │    │Digit OCR     │    │  Digit OCR   │
              │Suit + Rank   │    │+ Pixel FBack │    │+ Stack Diff  │
              └──────┬───────┘    └──────┬───────┘    └──────┬───────┘
                     │                   │                   │
                     └───────────┬───────┘───────────────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ Feature Aggregator   │
                      │ Cards + Stacks + Pot │
                      │ + Dealer + Seats     │
                      └──────────┬───────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ State Differ         │
                      │ prev_state vs curr   │
                      │ Detect all changes   │
                      └──────────┬───────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ Action Reconstructor │
                      │ Derive player acts   │
                      │ Fold/Call/Raise/etc  │
                      └──────────┬───────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ State Machine        │
                      │ Update TableState    │
                      │ Validate transitions │
                      │ Emit events          │
                      └──────────┬───────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ Recommendation Engine│
                      │ GTO/EV computation   │
                      │ Action distribution  │
                      └──────────┬───────────┘
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ Event Router         │
                      │ Publish to IPC       │
                      │ napi-rs callback     │
                      └──────────┬───────────┘
                                 │
                          IPC (napi-rs)
                                 │
                                 ▼
                      ┌──────────────────────┐
                      │ Overlay HUD          │
                      │ SolidJS Render       │
                      │ Transparent Window   │
                      │ Click-through        │
                      └──────────────────────┘
```

### 3.3 单帧处理时序

```
T+0ms    Frame Captured (DXGI)
T+2ms    Preprocessing Complete (resize, color convert)
T+3ms    ROI Extraction Complete
T+11ms   Feature Extraction Complete (card + OCR digit + stack)
T+13ms   State Diff + Action Reconstruction
T+14ms   State Machine Updated
T+19ms   Recommendation Engine Output (Sidecar JSON-RPC ~5ms)
T+22ms   IPC Transfer to Electron
T+24ms   HUD Rendered on Screen

Total: ~24ms end-to-end (target < 100ms)
说明：18.2 节列出每个阶段的预算上限（共 ~30ms），
此处是典型场景的实测目标。两者都低于 100ms 目标。
```

---

## 4. 技术栈与依赖

### 4.1 前端层技术栈

| 组件 | 技术 | 版本 | 选型原因 |
|------|------|------|----------|
| 桌面框架 | Electron | 33+ | 成熟桌面应用生态，Overlay 透明窗口支持完善 |
| UI 框架 | SolidJS | 1.8+ | 细粒度响应式，无 VDOM diff 开销，渲染性能优于 React |
| 样式 | TailwindCSS | 3.4+ | Utility-first，快速 UI 开发，构建时 tree-shake |
| 类型系统 | TypeScript | 5.3+ | 类型安全，IDE 支持完善 |
| 状态管理 | SolidJS Stores | - | 原生 fine-grained reactive，无需额外库 |
| 构建工具 | Vite | 5+ | 快速 HMR，ESBuild 预编译 |
| Overlay | Electron BrowserWindow | - | 透明窗口 + `setIgnoreMouseEvents` 点击穿透 |

### 4.2 Native Core 技术栈

| 组件 | 技术 | 选型原因 |
|------|------|----------|
| 核心语言 | Rust | 内存安全 + 零成本抽象 + 无 GC 暂停 |
| 异步运行时 | Tokio | 成熟的 async/await 生态，支持 multi-thread scheduler |
| 视觉处理 | opencv-rust | OpenCV 4.x Rust 绑定，工业级 CV 能力 |
| 深度学习推理 | ort (ONNX Runtime) | 跨平台推理，支持 CUDA / DirectML / CPU |
| IPC 桥接 | napi-rs | 零拷贝 Node.js native addon，自动 TS 类型生成 |
| 屏幕捕获 | windows-capture crate | DXGI Desktop Duplication API 封装 |
| 数据并行 | Rayon | Work-stealing 线程池，数据并行处理 |
| 序列化 | serde + serde_json | 零拷贝反序列化，IPC 数据传输 |
| 并发原语 | crossbeam | 高性能 channel / 无锁数据结构 |
| 日志 | tracing + tracing-subscriber | 结构化日志，tokio 集成 |
| 错误处理 | anyhow + thiserror | 应用级 + 库级错误处理 |
| 图像处理补充 | image crate | 基础图像编解码（PNG/JPEG） |

### 4.3 OCR 层技术栈

| 组件 | 技术 | 选型原因 |
|------|------|----------|
| OCR 引擎 | PaddleOCR (via ONNX) | 仅用于数字识别（筹码/底池数字），作为辅助验证 |
| 推理后端 | ONNX Runtime (shared) | 复用现有推理会话池，无需独立进程 |

### 4.4 Rust Crate 推荐清单

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
opencv = { version = "0.94", features = ["clang-runtime"] }
ort = { version = "2", features = ["cuda", "directml"] }
# napi-derive 与 napi 必须严格对齐到同一 minor 版本，否则会出现 ABI 不一致
napi = { version = "=2.16", features = ["napi4", "serde-json"] }
napi-derive = "=2.16"
rayon = "1.10"
crossbeam = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
image = "0.25"
windows = { version = "0.58", features = [
    "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Dxgi",
    "Win32_Graphics_Dxgi_Common",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
] }
windows-capture = "1"
parking_lot = "0.12"
dashmap = "6"
flume = "0.11"
bytemuck = "1"
crc32fast = "1"
num_cpus = "1"
```

### 4.5 Electron 前端依赖

```json
{
  "dependencies": {
    "electron": "^28.0.0",
    "solid-js": "^1.8.0",
    "@solidjs/router": "^0.13.0"
  },
  "devDependencies": {
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "vite-plugin-solid": "^2.8.0",
    "tailwindcss": "^3.4.0",
    "@napi-rs/cli": "^2.16.0"
  }
}
```

---

## 5. 目录结构

### 5.1 Monorepo 总体结构

```
table-flow/
├── ARCHITECTURE.md
├── prompt.md
├── Cargo.toml                    # Rust workspace root
├── package.json                  # Electron + Node workspace root
├── turbo.json                    # Turborepo config (optional)
├── .github/
│   └── workflows/
│       ├── ci-rust.yml
│       ├── ci-electron.yml
│       └── release.yml
│
├── crates/                       # Rust crates
│   ├── tf-core/                  # Core types, traits, shared abstractions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs          # Card, Suit, Rank, Street, ActionType, ...
│   │       ├── error.rs          # Error types
│   │       ├── events.rs         # TableEvent, StateTransition
│   │       └── config.rs         # Global configuration types
│   │
│   ├── tf-vision/                # Vision pipeline (capture, extract, detect)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capture/
│   │       │   ├── mod.rs
│   │       │   ├── dxgi.rs       # DXGI Desktop Duplication
│   │       │   ├── window.rs     # Window tracking & management
│   │       │   └── fps.rs        # FPS limiter
│   │       ├── pipeline/
│   │       │   ├── mod.rs
│   │       │   ├── preprocessor.rs   # Resize, normalize, color convert
│   │       │   ├── roi.rs            # ROI extraction & management
│   │       │   ├── diff.rs           # Frame differencing
│   │       │   └── aggregator.rs     # Feature aggregation
│   │       ├── detection/
│   │       │   ├── mod.rs
│   │       │   ├── card.rs           # Card detection (template + ONNX)
│   │       │   ├── stack.rs          # Stack detection (pixel analysis)
│   │       │   ├── pot.rs            # Pot detection
│   │       │   ├── dealer.rs         # Dealer button detection
│   │       │   ├── seat.rs           # Seat status detection
│   │       │   └── button.rs         # Action button detection
│   │       ├── matching/
│   │       │   ├── mod.rs
│   │       │   ├── template.rs       # Template matching engine
│   │       │   ├── contour.rs        # Contour analysis
│   │       │   └── feature.rs        # Feature vector extraction
│   │       └── calibration/
│   │           ├── mod.rs
│   │           └── auto.rs           # Auto-calibration
│   │
│   ├── tf-inference/             # ONNX inference runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── session.rs        # ONNX session pool management
│   │       ├── card_model.rs     # Card classification model
│   │       ├── digit_model.rs    # Digit OCR model (PaddleOCR export)
│   │       └── prepost.rs        # Pre/Post processing
│   │
│   ├── tf-state/                 # State machine & event reconstruction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── machine.rs        # TableStateMachine
│   │       ├── state.rs          # TableState, SeatState, PotInfo
│   │       ├── reconstructor.rs  # ActionReconstructor
│   │       ├── tracker.rs        # Pot tracker, Stack tracker
│   │       └── validator.rs      # State validation & sanity check
│   │
│   ├── tf-rec/                   # Recommendation engine integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs         # RecEngine wrapper
│   │       ├── input.rs          # RecInput builder (from TableState)
│   │       ├── output.rs         # RecOutput types
│   │       └── cache.rs          # Recommendation cache
│   │
│   ├── tf-table/                 # Multi-table manager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs        # TableManager (orchestrator)
│   │       ├── handle.rs         # TableHandle (per-table lifecycle)
│   │       └── discovery.rs      # Table window discovery
│   │
│   └── tf-napi/                  # napi-rs native addon (Electron bridge)
│       ├── Cargo.toml
│       ├── build.rs
│       └── src/
│           ├── lib.rs
│           ├── bridge.rs         # NapiBridge - main entry point
│           ├── commands.rs       # Exposed commands (start, stop, config)
│           ├── events.rs         # Event emission (state_change, rec_result)
│           └── types.rs          # JS↔Rust type marshalling
│
├── apps/                         # Electron applications
│   └── desktop/
│       ├── package.json
│       ├── electron-builder.yml
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── tailwind.config.js
│       ├── src/
│       │   ├── main/             # Electron main process
│       │   │   ├── index.ts
│       │   │   ├── window.ts         # Window management
│       │   │   ├── overlay.ts        # Overlay window creation
│       │   │   ├── tray.ts           # System tray
│       │   │   ├── ipc.ts            # IPC handler (calls native addon)
│       │   │   └── native.ts         # Native addon loader
│       │   ├── renderer/         # SolidJS renderer
│       │   │   ├── index.tsx
│       │   │   ├── App.tsx
│       │   │   ├── store/
│       │   │   │   ├── table.ts      # Table state store
│       │   │   │   ├── recommendation.ts  # Rec store
│       │   │   │   └── settings.ts    # Settings store
│       │   │   ├── components/
│       │   │   │   ├── overlay/
│       │   │   │   │   ├── HudOverlay.tsx    # Main overlay container
│       │   │   │   │   ├── Recommendation.tsx # Action recommendation
│       │   │   │   │   ├── ActionDistribution.tsx # Action bar chart
│       │   │   │   │   ├── PotDisplay.tsx    # Pot info
│       │   │   │   │   └── PlayerStats.tsx   # Per-player stats
│       │   │   │   ├── settings/
│       │   │   │   │   ├── SettingsPanel.tsx
│       │   │   │   │   ├── TableCalibration.tsx
│       │   │   │   │   └── ThemeSelector.tsx
│       │   │   │   └── dashboard/
│       │   │   │       ├── Dashboard.tsx
│       │   │   │       ├── TableGrid.tsx
│       │   │   │       └── SessionStats.tsx
│       │   │   └── styles/
│       │   │       └── index.css     # Tailwind base
│       │   └── preload/
│       │       └── index.ts          # Preload bridge
│       └── resources/
│           ├── models/               # ONNX model files
│           │   ├── card_classifier.onnx
│           │   ├── card_detector.onnx
│           │   └── digit_recognizer.onnx
│           └── templates/            # Card/element templates
│               ├── cards/
│               │   ├── 2s.png ... As.png   # 52 card templates
│               │   └── back.png
│               ├── buttons/
│               │   ├── fold.png
│               │   ├── call.png
│               │   ├── raise.png
│               │   └── allin.png
│               └── elements/
│                   ├── dealer_button.png
│                   └── chip_stack.png
│
└── tests/                        # Integration tests
    ├── rust/
    │   ├── vision_tests.rs
    │   ├── state_machine_tests.rs
    │   └── pipeline_tests.rs
    └── fixtures/
        ├── frames/                # Test frame images
        ├── templates/             # Test templates
        └── states/                # Expected state snapshots
```

### 5.2 Crate 依赖关系

```
tf-napi
  ├── tf-table
  │     ├── tf-state
  │     │     ├── tf-core
  │     │     └── tf-vision
  │     │           ├── tf-core
  │     │           ├── tf-inference
  │     │           │     └── tf-core
  │     │           └── opencv
  │     ├── tf-rec
  │     │     ├── tf-core
  │     │     └── tf-state
  │     └── tf-core
  └── tf-core
```

### 5.3 Rust Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/tf-core",
    "crates/tf-vision",
    "crates/tf-inference",
    "crates/tf-state",
    "crates/tf-rec",
    "crates/tf-table",
    "crates/tf-napi",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## 6. 线程模型与多进程设计

### 6.1 进程架构

```
┌─────────────────────────────────────────────────────────┐
│  Electron Main Process                                    │
│  - Window lifecycle management                            │
│  - System tray                                            │
│  - Native addon loading (tf-napi)                         │
│  - IPC routing                                            │
│                                                           │
│  Thread Pool: Node.js libuv thread pool                   │
│  Native Thread: Rust Tokio runtime (dedicated thread)    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  Electron Renderer Process (per window)                   │
│  - SolidJS UI rendering                                   │
│  - SolidJS reactive stores                                │
│  - Web Worker for heavy computation (optional)            │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  Rust Tokio Runtime (inside native addon)                 │
│  - Multi-threaded scheduler (core_count threads)          │
│  - Per-table capture tasks                                │
│  - Per-table state machine tasks                          │
│  - Shared inference thread pool                           │
│  - IPC event emission task                                │
└─────────────────────────────────────────────────────────┘
```

### 6.2 线程分配策略

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Rust Tokio Runtime                            │
│                                                                     │
│  Tokio Worker Threads (N = CPU core count)                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│  │Worker 0  │ │Worker 1  │ │Worker 2  │ │Worker N  │              │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘              │
│       │            │            │            │                       │
│  ┌────┴────────────┴────────────┴────────────┴──────┐              │
│  │               Tokio Task Arena                    │              │
│  │                                                  │              │
│  │  Table 1 Tasks:                                  │              │
│  │    [capture_task_1] → [state_task_1]             │              │
│  │                                                  │              │
│  │  Table 2 Tasks:                                  │              │
│  │    [capture_task_2] → [state_task_2]             │              │
│  │                                                  │              │
│  │  Table N Tasks:                                  │              │
│  │    [capture_task_N] → [state_task_N]             │              │
│  │                                                  │              │
│  │  Shared Tasks:                                   │              │
│  │    [ipc_bridge_task]                             │              │
│  │    [table_discovery_task]                        │              │
│  └──────────────────────────────────────────────────┘              │
│                                                                     │
│  Rayon Thread Pool (separate from Tokio)                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│  │Rayon W0  │ │Rayon W1  │ │Rayon W2  │ │Rayon WN  │              │
│  │  Feature  │ │  Card    │ │  Stack   │ │  Template │              │
│  │  Extract  │ │  Detect  │ │  Analyze │ │  Match   │              │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘              │
│                                                                     │
│  ONNX Session Threads (managed by ONNX Runtime)                    │
│  ┌──────────────────────────────────────────────────┐              │
│  │  Intra-op threads: parallel ops within model     │              │
│  │  Inter-op threads: parallel model executions     │              │
│  └──────────────────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.3 线程池配置

```rust
pub struct ThreadConfig {
    pub tokio_worker_threads: usize,
    pub rayon_worker_threads: usize,
    pub onnx_intra_threads: usize,
    pub onnx_inter_threads: usize,
    pub max_tables: usize,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        let cores = num_cpus::get();
        Self {
            tokio_worker_threads: cores,
            rayon_worker_threads: (cores * 3) / 4,
            onnx_intra_threads: 2,
            onnx_inter_threads: 2,
            max_tables: 8,
        }
    }
}
```

### 6.4 Channel 架构

```
Table 1:
  capture_task ──[flume::bounded(2)]──→ feature_worker(Rayon)
                                              │
                                    [crossbeam::bounded(4)]
                                              │
                                              ▼
  state_task ◄────────────────────── FeatureResult
      │
      │ (state change detected)
      ▼
  rec_task ──→ RecEngine ──→ RecResult
      │
      │ (result ready)
      ▼
  ipc_bridge_task ──[napi ThreadsafeFunction]──→ Electron Main Process
                                                      │
                                                Electron IPC
                                                      │
                                                      ▼
                                                Overlay Renderer
```

```rust
pub struct TableChannels {
    pub frame_tx: flume::Sender<CapturedFrame>,
    pub frame_rx: flume::Receiver<CapturedFrame>,
    pub feature_tx: crossbeam::channel::Sender<ExtractedFeatures>,
    pub feature_rx: crossbeam::channel::Receiver<ExtractedFeatures>,
    pub state_event_tx: tokio::sync::broadcast::Sender<TableEvent>,
}
```

---

## 7. Frame Processing Pipeline

### 7.1 Pipeline 总览

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐     ┌────────────────┐
│   Capture   │────▶│  Preprocess  │────▶│  ROI Extract    │────▶│  Frame Diff    │
│   Module    │     │  Module      │     │  Module         │     │  Module        │
└─────────────┘     └──────────────┘     └─────────────────┘     └───────┬────────┘
                                                                         │
                                              ┌──────────────────────────┤
                                              │  Has Changes?            │
                                              │  NO → Skip (return)     │
                                              │  YES ↓                   │
                                              └──────────────────────────┘
                                                                         │
                    ┌──────────────────────────┼──────────────────────────┼──────────────┐
                    │                          │                          │              │
                    ▼                          ▼                          ▼              ▼
          ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐  ┌──────────────┐
          │  Card Detection  │     │  Stack Tracker  │     │   Pot Tracker   │  │ Seat Tracker │
          │  (Template+ONNX) │     │  (Pixel Diff)   │     │  (Region Diff)  │  │ (Contour)    │
          └────────┬────────┘     └────────┬────────┘     └────────┬────────┘  └──────┬───────┘
                   │                       │                       │                  │
                   └───────────────────────┼───────────────────────┘──────────────────┘
                                           │
                                           ▼
                                 ┌───────────────────┐
                                 │  Feature Merge    │
                                 │  + Confidence     │
                                 └────────┬──────────┘
                                          │
                                          ▼
                                 ┌───────────────────┐
                                 │  State Diff       │
                                 │  Compare prev/cur │
                                 └────────┬──────────┘
                                          │
                                          ▼
                                 ┌───────────────────┐
                                 │  Event Emit       │
                                 │  TableEvent       │
                                 └───────────────────┘
```

### 7.2 Vision Pipeline 实现

```rust
pub struct VisionPipeline {
    table_id: TableId,
    capture: FrameCapture,
    preprocessor: Preprocessor,
    roi_manager: RoiManager,
    diff_detector: DiffDetector,
    card_detector: CardDetector,
    stack_tracker: StackTracker,
    pot_tracker: PotTracker,
    seat_tracker: SeatTracker,
    dealer_tracker: DealerTracker,
    hero_detector: HeroDetector,
    ocr_assistant: OcrAssistant,
    aggregator: FeatureAggregator,
    output_tx: flume::Sender<ExtractedFeatures>,
}

impl VisionPipeline {
    pub async fn run(mut self) {
        let mut prev_frame: Option<Mat> = None;

        loop {
            let raw_frame = match self.capture.capture_frame().await {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Capture error: {:?}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };

            let processed = self.preprocessor.process(&raw_frame);

            let changed = match &prev_frame {
                Some(prev) => self.diff_detector.has_significant_change(prev, &processed),
                None => true,
            };

            if !changed {
                prev_frame = Some(processed);
                continue;
            }

            let rois = self.roi_manager.extract(&processed);
            let features = self.extract_all_features(&rois, &processed);
            let merged = self.aggregator.merge(features);

            if let Err(e) = self.output_tx.send(merged) {
                tracing::error!("Feature channel closed: {:?}", e);
                break;
            }

            prev_frame = Some(processed);
        }
    }

    fn extract_all_features(&self, rois: &TableROI, frame: &Mat) -> RawFeatures {
        let ocr = Some(&self.ocr_assistant);
        let card_result = self.card_detector.detect(&rois.hole_cards, &rois.community_cards, frame);
        let stack_result = self.stack_tracker.track(&rois.player_seats, frame, ocr);
        let pot_result = self.pot_tracker.track(&rois.pot_area, frame, ocr);
        let seat_result = self.seat_tracker.track(&rois.player_seats, frame);
        let dealer_result = self.dealer_tracker.detect(&rois.dealer_button, frame);

        RawFeatures {
            cards: card_result,
            stacks: stack_result,
            pot: pot_result,
            seats: seat_result,
            dealer: dealer_result,
            timestamp: Instant::now(),
        }
    }
}
```

### 7.3 帧差分策略

```rust
pub struct DiffDetector {
    threshold: f64,
    roi_mask: Mat,
}

impl DiffDetector {
    pub fn has_significant_change(&self, prev: &Mat, curr: &Mat) -> bool {
        let diff = {
            let mut d = Mat::default();
            opencv::core::absdiff(prev, curr, &mut d).unwrap();
            d
        };

        let gray = {
            let mut g = Mat::default();
            opencv::imgproc::cvt_color(&diff, &mut g, opencv::imgproc::COLOR_BGR2GRAY, 0).unwrap();
            g
        };

        let thresh = {
            let mut t = Mat::default();
            opencv::imgproc::threshold(&gray, &mut t, 30.0, 255.0, opencv::imgproc::THRESH_BINARY).unwrap();
            t
        };

        let non_zero = opencv::core::count_non_zero(&thresh).unwrap();
        let total = thresh.rows() * thresh.cols();

        (non_zero as f64) / (total as f64) > self.threshold
    }
}
```

---

## 8. Vision Core 模块详细设计

### 8.1 Frame Capture Module

**职责：**
- DXGI Desktop Duplication API 捕获指定窗口
- 自动检测牌桌窗口位置变化
- 帧率控制（30-60 FPS per table）
- 零拷贝纹理传输

```rust
pub struct FrameCapture {
    table_id: TableId,
    window_handle: Option<HWND>,
    window_title_pattern: String,
    capture_region: Rect,
    fps_limiter: FpsLimiter,
    frame_counter: AtomicU64,
}

pub struct CapturedFrame {
    pub timestamp: Instant,
    pub table_id: TableId,
    pub image: Mat,
    pub frame_number: u64,
    pub latency: Duration,
}

pub struct FpsLimiter {
    target_fps: u32,
    last_capture: Instant,
    min_interval: Duration,
}

impl FrameCapture {
    pub async fn capture_frame(&mut self) -> Result<CapturedFrame> {
        self.fps_limiter.wait().await;

        let start = Instant::now();
        let image = self.capture_region_dxgi()?;
        let latency = start.elapsed();

        let frame = CapturedFrame {
            timestamp: Instant::now(),
            table_id: self.table_id,
            image,
            frame_number: self.frame_counter.fetch_add(1, Ordering::Relaxed),
            latency,
        };

        Ok(frame)
    }

    fn capture_region_dxgi(&self) -> Result<Mat> {
        // 1. Acquire DXGI frame from Desktop Duplication
        // 2. Map GPU texture to CPU memory
        // 3. Convert BGRA → BGR (OpenCV Mat)
        // 4. Crop to capture_region
        // 5. Return Mat
        todo!("DXGI implementation")
    }

    pub fn detect_window_position_change(&mut self) -> Option<Rect> {
        // Enumerate windows matching title pattern
        // Compare position to current capture_region
        // Return new Rect if changed
        todo!()
    }
}
```

### 8.2 Preprocessor Module

**职责：**
- 图像尺寸标准化
- 颜色空间转换
- 噪声过滤
- 对比度增强

```rust
pub struct Preprocessor {
    target_size: (u32, u32),
    denoise: bool,
}

impl Preprocessor {
    pub fn process(&self, frame: &Mat) -> Mat {
        let mut result = frame.clone();

        if self.denoise {
            let mut denoised = Mat::default();
            opencv::photo::fast_nl_means_denoising_colored(
                &result, &mut denoised, 10.0, 10.0, 7, 21
            ).unwrap();
            result = denoised;
        }

        result
    }
}
```

### 8.3 ROI Manager

**职责：**
- 定义各检测区域的位置
- 支持多分辨率模板（比例映射）
- 自动校准
- 区域缓存

```rust
pub struct RoiManager {
    table_id: TableId,
    table_resolution: (u32, u32),
    calibration: TableCalibration,
    cached_rois: Option<TableROI>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCalibration {
    pub resolution: (u32, u32),
    pub hole_card_positions: [(f64, f64, f64, f64); 2],
    pub community_card_positions: [(f64, f64, f64, f64); 5],
    pub pot_position: (f64, f64, f64, f64),
    pub seat_positions: [SeatCalibration; 10],
    pub dealer_button_region: (f64, f64, f64, f64),
    pub action_button_regions: [(f64, f64, f64, f64); 4],
    /// 用户手动指定的 Hero 座位（MVP 必填）
    pub hero_seat: Option<SeatId>,
    /// 该桌的盲注配置（启动时手动填，或从窗口标题/平台 API 解析）
    pub blinds: BlindsInfo,
    /// 数字 OCR 区域（pot / 各 seat stack / 各 seat current_bet）
    pub digit_ocr_regions: DigitOcrRegions,
    /// 客户端主题标识（同一平台多个皮肤需要不同 calibration profile）
    pub theme_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DigitOcrRegions {
    pub pot: Option<(f64, f64, f64, f64)>,
    pub seat_stacks: [Option<(f64, f64, f64, f64)>; 10],
    pub seat_bets: [Option<(f64, f64, f64, f64)>; 10],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatCalibration {
    pub seat_region: (f64, f64, f64, f64),
    pub stack_region: (f64, f64, f64, f64),
    pub bet_region: (f64, f64, f64, f64),
    pub avatar_region: (f64, f64, f64, f64),
    pub card_region: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct TableROI {
    pub hole_cards: [Rect; 2],
    pub community_cards: [Rect; 5],
    pub pot_area: Rect,
    pub player_seats: [SeatROI; 10],
    pub dealer_button: Rect,
    pub action_buttons: [Rect; 4],
}

#[derive(Debug, Clone)]
pub struct SeatROI {
    pub seat_area: Rect,
    pub stack_area: Rect,
    pub bet_area: Rect,
    pub avatar_area: Rect,
    pub card_area: Option<Rect>,
}
```

**自动校准流程：**

```
Input Frame
    │
    ▼
Detect Table Ellipse (Hough Circle / Green felt mask)
    │
    ▼
Locate Card Rectangles (contour detection, aspect ratio ~0.7)
    │
    ▼
Classify: 2 closest = hole cards, center row = community cards
    │
    ▼
Locate Chip Regions (color-based segmentation near seats)
    │
    ▼
Detect Dealer Button (small circular template match)
    │
    ▼
Map all positions → normalized (0.0-1.0) coordinates
    │
    ▼
Store as TableCalibration
```

### 8.4 Card Detection Module

**职责：**
- 手牌识别（2张）
- 公共牌识别（0-5张）
- 花色识别（4种）
- 点数识别（13种）
- 卡牌存在性检测（有/无牌）

```rust
pub struct CardDetector {
    template_matcher: TemplateMatcher,
    onnx_classifier: CardClassifier,
    suit_analyzer: SuitAnalyzer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten,
    Jack, Queen, King, Ace,
}

impl Rank {
    pub fn value(&self) -> u8 {
        match self {
            Rank::Two => 2, Rank::Three => 3, Rank::Four => 4, Rank::Five => 5,
            Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9,
            Rank::Ten => 10, Rank::Jack => 11, Rank::Queen => 12,
            Rank::King => 13, Rank::Ace => 14,
        }
    }

    pub fn short_str(&self) -> &'static str {
        match self {
            Rank::Two => "2", Rank::Three => "3", Rank::Four => "4", Rank::Five => "5",
            Rank::Six => "6", Rank::Seven => "7", Rank::Eight => "8", Rank::Nine => "9",
            Rank::Ten => "T", Rank::Jack => "J", Rank::Queen => "Q",
            Rank::King => "K", Rank::Ace => "A",
        }
    }
}

impl Suit {
    pub fn short_str(&self) -> &'static str {
        match self {
            Suit::Spades => "s", Suit::Hearts => "h",
            Suit::Diamonds => "d", Suit::Clubs => "c",
        }
    }
}

impl CardDetector {
    fn detect_single_card(&self, roi: &Rect, frame: &Mat) -> Option<Card> {
        let card_img = Mat::roi(frame, roi).ok()?;

        if !self.is_card_present(&card_img) {
            return None;
        }

        // Priority 1: Template matching (fast, high accuracy)
        if let Some(card) = self.template_matcher.match_card(&card_img) {
            if card.confidence > 0.95 {
                return Some(card);
            }
        }

        // Priority 2: ONNX classification (fallback)
        self.onnx_classifier.classify(&card_img)
    }

    fn is_card_present(&self, card_img: &Mat) -> bool {
        let mean = opencv::core::mean(card_img, &Mat::default()).unwrap();
        let brightness = (mean[0] + mean[1] + mean[2]) / 3.0;
        brightness > 80.0
    }
}
```

**卡牌识别算法优先级：**

```
1. 模板匹配 (Template Matching)
   - 预存 52 张标准牌面模板
   - 多尺度匹配 (0.9x - 1.1x)
   - 阈值: confidence > 0.95
   - 延迟: ~2ms per card
   - 适用: 清晰、正面、标准皮肤

2. ONNX 分类模型 (Fallback)
   - 输入: 64x90 RGB card crop
   - 输出: 52-class softmax + 4-class suit
   - 阈值: confidence > 0.8
   - 延迟: ~5ms per card (CPU) / ~1ms (GPU)
   - 适用: 模糊、部分遮挡、非标准皮肤

3. 花色色彩分析 (辅助验证)
   - HSV 色彩空间分割
   - 红色范围: H 0-10, 160-180, S > 100
   - 黑色范围: S < 50, V < 100
   - 用于验证模板/模型结果
```

### 8.5 Stack Detection Module

**职责：**
- 玩家筹码堆区域检测
- 筹码量变化追踪（核心：不依赖 OCR）
- 有效筹码估算
- Bet 区域金额追踪

```rust
pub struct StackTracker {
    baselines: HashMap<SeatId, StackBaseline>,
    history: HashMap<SeatId, VecDeque<StackSnapshot>>,
}

#[derive(Debug, Clone)]
pub struct StackBaseline {
    seat_id: SeatId,
    known_value: Option<f64>,
    pixel_count: f64,
    calibration_factor: f64,
}

#[derive(Debug, Clone)]
pub struct StackSnapshot {
    pub seat_id: SeatId,
    pub pixel_area: f64,
    pub estimated_value: f64,
    pub confidence: f32,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct StackChange {
    pub seat_id: SeatId,
    pub prev_estimated: f64,
    pub curr_estimated: f64,
    pub delta: f64,
    pub confidence: f32,
}

impl StackTracker {
    /// 主路径：digit OCR；fallback：像素面积。
    /// 注意 ocr 可以是 None（例如 OCR 不可用时），届时只走像素估算。
    pub fn track(
        &mut self,
        seat_rois: &[SeatROI],
        frame: &Mat,
        ocr: Option<&OcrAssistant>,
    ) -> Vec<StackChange> {
        let mut changes = Vec::new();

        for (idx, seat_roi) in seat_rois.iter().enumerate() {
            let seat_id = SeatId(idx as u8);
            let stack_img = match Mat::roi(frame, seat_roi.stack_area) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let current = self.analyze_stack_region(seat_id, &stack_img, ocr);

            if let Some(prev) = self.history.get(&seat_id).and_then(|h| h.back()) {
                let delta = current.estimated_value - prev.estimated_value;
                if delta.abs() > 0.5 {
                    changes.push(StackChange {
                        seat_id,
                        prev_estimated: prev.estimated_value,
                        curr_estimated: current.estimated_value,
                        delta,
                        confidence: current.confidence.min(prev.confidence),
                    });
                }
            }

            self.history.entry(seat_id).or_default().push_back(current);
            if self.history.get(&seat_id).map_or(false, |h| h.len() > 30) {
                self.history.get_mut(&seat_id).unwrap().pop_front();
            }
        }

        changes
    }

    fn analyze_stack_region(
        &self,
        seat_id: SeatId,
        stack_img: &Mat,
        ocr: Option<&OcrAssistant>,
    ) -> StackSnapshot {
        // 主路径：digit OCR
        if let Some(ocr) = ocr {
            if let Some(value) = ocr.recognize_digits(stack_img) {
                return StackSnapshot {
                    seat_id,
                    pixel_area: 0.0,
                    estimated_value: value,
                    confidence: 0.97,
                    timestamp: Instant::now(),
                };
            }
        }

        // Fallback：像素面积
        let pixel_count = self.count_chip_pixels(stack_img);
        let estimated_value = match self.baselines.get(&seat_id) {
            Some(baseline) => pixel_count / baseline.calibration_factor,
            None => pixel_count / 100.0,
        };

        StackSnapshot {
            seat_id,
            pixel_area: pixel_count,
            estimated_value,
            confidence: 0.70, // 像素估算置信度低于 OCR
            timestamp: Instant::now(),
        }
    }

    fn count_chip_pixels(&self, img: &Mat) -> f64 {
        let hsv = {
            let mut h = Mat::default();
            opencv::imgproc::cvt_color(img, &mut h, opencv::imgproc::COLOR_BGR2HSV, 0).unwrap();
            h
        };

        let mut mask = Mat::default();
        let lower = opencv::core::Scalar::new(0.0, 50.0, 50.0, 0.0);
        let upper = opencv::core::Scalar::new(180.0, 255.0, 255.0, 0.0);
        opencv::core::in_range(&hsv, &lower, &upper, &mut mask).unwrap();

        opencv::core::count_non_zero(&mask).unwrap() as f64
    }

    pub fn calibrate_from_known_value(
        &mut self,
        seat_id: SeatId,
        known_stack: f64,
        frame: &Mat,
        roi: &Rect,
    ) {
        let stack_img = Mat::roi(frame, *roi).unwrap();
        let pixel_count = self.count_chip_pixels(&stack_img);
        let calibration_factor = if known_stack > 0.0 {
            pixel_count / known_stack
        } else {
            100.0
        };

        self.baselines.insert(seat_id, StackBaseline {
            seat_id,
            known_value: Some(known_stack),
            pixel_count,
            calibration_factor,
        });
    }
}
```

**筹码识别策略（修正版）：**

> 关键澄清：`prompt.md` 的硬约束是 *"不允许 OCR-first"* —— 指的是
> **不允许用 OCR 识别按钮文字（FOLD/CALL/RAISE）来推导动作**，
> **不是禁止用 OCR 读数字本身**。
> 主流线上扑克客户端绝大多数情况下筹码以 **纯文本数字** 显示（没有物理筹码堆），
> 因此筹码主路径必须是 **数字 OCR + 差分推导动作**：用 OCR 拿到状态值（数字），
> 用差分（Stack Diff）推导动作语义，这才是 prompt 想要的"状态推导而非文字识别"。

```
方法 1（主路径）: PaddleOCR digit-only 模型
  - 输入：seat.stack_region 上的小矩形 ROI
  - 输出：浮点数 stack value
  - 触发：每次 ROI 像素 hash 变化时调用，未变化则直接复用上次结果
  - 性能：~3-8ms/region (CPU) / ~1ms (GPU)
  - 精度：>= 99%（数字清晰场景）

方法 2（fallback）: 像素面积法（物理筹码堆 UI）
  - 仅当 OCR 失败 / 客户端用物理筹码堆显示时启用
  - 与 calibration_factor 配合估算

方法 3（动作推导）: Stack Diff
  - 不论 stack 是 OCR 读出还是像素估算，
    最终的"动作 = Call/Bet/Raise/AllIn"判定都靠
    prev_stack → curr_stack 的差分 + Pot Diff 联合推导
  - 这一步是真正的"State Derivation"
```

### 8.6 Pot Tracking Module

**职责：**
- 底池区域变化检测
- 主池与边池追踪
- 底池金额估算

```rust
pub struct PotTracker {
    last_value: f64,
    last_pixel_hash: u64,
    change_history: VecDeque<PotChange>,
}

#[derive(Debug, Clone)]
pub struct PotChange {
    pub prev_value: f64,
    pub new_value: f64,
    pub delta: f64,
    pub timestamp: Instant,
}

impl PotTracker {
    pub fn track(&mut self, pot_roi: &Rect, frame: &Mat) -> Option<PotChange> {
        let pot_img = Mat::roi(frame, *pot_roi).ok()?;

        let current_hash = self.compute_hash(&pot_img);

        if current_hash == self.last_pixel_hash {
            return None;
        }

        self.last_pixel_hash = current_hash;

        let current_value = self.estimate_pot_value(&pot_img)?;

        let change = PotChange {
            prev_value: self.last_value,
            new_value: current_value,
            delta: current_value - self.last_value,
            timestamp: Instant::now(),
        };

        self.last_value = current_value;
        self.change_history.push_back(change.clone());

        Some(change)
    }

    fn compute_hash(&self, img: &Mat) -> u64 {
        let mut hasher = crc32fast::Hasher::new();
        let data = img.data_bytes().unwrap_or(&[]);
        hasher.update(data);
        hasher.finalize() as u64
    }

    /// Pot 估算策略（State-Derivation-First）：
    /// 1. 主路径：基于上一帧 pot + Stack Diff 推导 ΔPot（不依赖 OCR）
    /// 2. 辅助：每 N 次变化或检测到推导漂移时，调用 digit OCR 校验/修正
    /// 3. 兜底：纯像素面积估算（最不准，仅在前两条都失败时使用）
    ///
    /// 注意：此函数只负责"读出当前底池数值"，**主推导仍然在 Action Reconstructor**。
    fn estimate_pot_value(&self, pot_img: &Mat, ocr: Option<&OcrAssistant>) -> Option<f64> {
        // Step 1: 像素 hash 相同则直接返回上次值（在外层已过滤）
        // Step 2: 用 digit OCR 在 pot ROI 上读数字（PaddleOCR digit-only 模型）
        if let Some(ocr) = ocr {
            if let Some(v) = ocr.recognize_digits(pot_img) {
                return Some(v);
            }
        }
        // Step 3: 兜底用像素面积估算（基于 calibration_factor）
        None
    }
}
```

### 8.7 Seat Mapping Module

**职责：**
- 玩家座位状态检测（空座/有人/活跃/弃牌/全押）
- 座位位置映射
- 头像/名字区域追踪

```rust
pub struct SeatTracker {
    seat_states: HashMap<SeatId, TrackedSeat>,
}

#[derive(Debug, Clone)]
pub struct TrackedSeat {
    pub seat_id: SeatId,
    pub status: SeatStatus,
    pub last_seen_active: Instant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SeatStatus {
    Empty,
    SittingOut,
    Active,
    Folded,
    AllIn,
}

#[derive(Debug, Clone)]
pub struct SeatChange {
    pub seat_id: SeatId,
    pub prev_status: Option<SeatStatus>,
    pub new_status: SeatStatus,
}

impl SeatTracker {
    pub fn track(&mut self, seat_rois: &[SeatROI], frame: &Mat) -> Vec<SeatChange> {
        let mut changes = Vec::new();

        for (idx, seat_roi) in seat_rois.iter().enumerate() {
            let seat_id = SeatId(idx as u8);
            let seat_img = Mat::roi(frame, seat_roi.seat_area).unwrap();

            let current_status = self.classify_seat_status(&seat_img);
            let prev = self.seat_states.get(&seat_id);

            if prev.map_or(true, |p| p.status != current_status) {
                changes.push(SeatChange {
                    seat_id,
                    prev_status: prev.map(|p| p.status.clone()),
                    new_status: current_status.clone(),
                });
            }

            self.seat_states.insert(seat_id, TrackedSeat {
                seat_id,
                status: current_status,
                last_seen_active: Instant::now(),
            });
        }

        changes
    }

    fn classify_seat_status(&self, seat_img: &Mat) -> SeatStatus {
        let mean = opencv::core::mean(seat_img, &Mat::default()).unwrap();
        let brightness = (mean[0] + mean[1] + mean[2]) / 3.0;

        if brightness < 40.0 {
            SeatStatus::Empty
        } else if brightness < 80.0 {
            SeatStatus::SittingOut
        } else if brightness < 120.0 {
            SeatStatus::Folded
        } else {
            SeatStatus::Active
        }
    }
}
```

### 8.8 Dealer Button Detection Module

**职责：**
- 庄家按钮位置检测
- 庄家位置变化追踪

```rust
pub struct DealerTracker {
    template: Mat,
    last_position: Option<SeatId>,
}

impl DealerTracker {
    pub fn detect(&mut self, dealer_roi: &Rect, frame: &Mat) -> Option<SeatId> {
        let region = Mat::roi(frame, *dealer_roi).ok()?;

        let mut result = Mat::default();
        opencv::imgproc::match_template(
            &region, &self.template, &mut result,
            opencv::imgproc::TM_CCOEFF_NORMED, &opencv::core::no_array()
        ).ok()?;

        // opencv-rust 的 min_max_loc 是 C 风格输出参数，不会以 tuple 返回
        let mut min_val = 0.0f64;
        let mut max_val = 0.0f64;
        let mut min_loc = opencv::core::Point::default();
        let mut max_loc = opencv::core::Point::default();
        opencv::core::min_max_loc(
            &result,
            Some(&mut min_val),
            Some(&mut max_val),
            Some(&mut min_loc),
            Some(&mut max_loc),
            &opencv::core::no_array(),
        ).ok()?;

        if max_val > 0.8 {
            let dealer_seat = self.position_to_seat(max_loc);
            self.last_position = Some(dealer_seat);
            Some(dealer_seat)
        } else {
            None
        }
    }

    fn position_to_seat(&self, loc: opencv::core::Point) -> SeatId {
        todo!("Map pixel position to seat ID based on table layout")
    }
}
```

### 8.9 Action Reconstruction Module

**职责：**
- 基于状态变化推导玩家动作（核心模块）
- 消除误检和噪声
- 生成高置信度事件序列
- 处理复杂场景（All-in、Side pot 等）

```rust
pub struct ActionReconstructor {
    config: ReconConfig,
}

#[derive(Debug, Clone)]
pub struct ReconConfig {
    pub min_stack_change: f64,
    pub min_pot_change: f64,
    pub debounce_ms: u64,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedAction {
    pub seat_id: SeatId,
    pub action_type: ActionType,
    pub amount: Option<f64>,
    pub street: Street,
    pub timestamp: Instant,
    pub confidence: f32,
    pub source: ActionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Fold,
    Check,
    Call,
    Bet(f64),
    Raise(f64),
    AllIn(f64),
    /// 强制盲注 / Ante，非自愿动作（不进入 GTO 推荐的 action_history）
    PostBlind(BlindKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlindKind {
    SmallBlind,
    BigBlind,
    Straddle,
    Ante,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionSource {
    StackDiff,
    PotDiff,
    SeatStatusChange,
    Combined,
}

impl ActionReconstructor {
    pub fn reconstruct(
        &self,
        prev_state: &TableState,
        current_features: &ExtractedFeatures,
    ) -> Vec<ReconstructedAction> {
        let mut actions = Vec::new();

        // 1. Stack-based action detection
        for change in &current_features.stack_changes {
            if let Some(action) = self.derive_from_stack_change(prev_state, change) {
                actions.push(action);
            }
        }

        // 2. Pot-based action verification
        if let Some(pot_change) = &current_features.pot_change {
            self.cross_validate_with_pot(&mut actions, pot_change);
        }

        // 3. Seat status-based detection
        for seat_change in &current_features.seat_changes {
            if seat_change.new_status == SeatStatus::Folded {
                actions.push(ReconstructedAction {
                    seat_id: seat_change.seat_id,
                    action_type: ActionType::Fold,
                    amount: None,
                    street: prev_state.street,
                    timestamp: Instant::now(),
                    confidence: 0.95,
                    source: ActionSource::SeatStatusChange,
                });
            }
            if seat_change.new_status == SeatStatus::AllIn {
                actions.push(ReconstructedAction {
                    seat_id: seat_change.seat_id,
                    action_type: ActionType::AllIn(0.0),
                    amount: None,
                    street: prev_state.street,
                    timestamp: Instant::now(),
                    confidence: 0.90,
                    source: ActionSource::SeatStatusChange,
                });
            }
        }

        self.deduplicate(&mut actions);
        actions
    }

    fn derive_from_stack_change(
        &self,
        prev_state: &TableState,
        change: &StackChange,
    ) -> Option<ReconstructedAction> {
        if change.delta >= -self.config.min_stack_change {
            return None;
        }

        let amount = change.delta.abs();
        let seat = prev_state.seats.get(change.seat_id.0 as usize)?;
        let to_call = self.compute_to_call(prev_state);
        let prev_bet = seat.current_bet;
        let new_bet = prev_bet + amount;
        let bb = prev_state.blinds.big_blind;
        const EPS: f64 = 0.01;

        // ① All-in 必须最先判定（stack 归零优先级最高）
        let action_type = if (seat.stack - amount).abs() < EPS || seat.stack - amount <= 0.0 {
            ActionType::AllIn(seat.stack)
        }
        // ② 盲注 / Ante：preflop 且金额恰好等于 SB/BB/Ante 且 hand 刚开始
        else if prev_state.street == Street::Preflop
            && prev_state.action_history.is_empty()
            && (amount - prev_state.blinds.small_blind).abs() < EPS
        {
            // 由上层 StateMachine 单独处理 PostBlind，这里直接跳过避免误判成 Bet
            return None;
        }
        else if prev_state.street == Street::Preflop
            && prev_state.action_history.iter().filter(|a| matches!(a.action, ActionType::PostBlind(_))).count() < 2
            && (amount - bb).abs() < EPS
        {
            return None;
        }
        // ③ Call: new_bet 与 to_call 在 EPS 内相等
        else if to_call > EPS && (new_bet - to_call).abs() < EPS {
            ActionType::Call
        }
        // ④ Bet: 当前轮没人下注（to_call ≈ 0），自己开下
        else if to_call < EPS && new_bet > EPS {
            ActionType::Bet(amount)
        }
        // ⑤ Raise: new_bet 超过现有 to_call
        else if to_call > EPS && new_bet > to_call + EPS {
            ActionType::Raise(new_bet)
        }
        // ⑥ 兜底：返回 None 让上游 fallback，不要伪造 Call
        else {
            return None;
        };

        Some(ReconstructedAction {
            seat_id: change.seat_id,
            action_type,
            amount: Some(amount),
            street: prev_state.street,
            timestamp: Instant::now(),
            confidence: change.confidence * 0.9,
            source: ActionSource::StackDiff,
        })
    }

    fn cross_validate_with_pot(
        &self,
        actions: &mut Vec<ReconstructedAction>,
        pot_change: &PotChange,
    ) {
        let total_action_amount: f64 = actions.iter()
            .filter_map(|a| a.amount)
            .sum();

        if (total_action_amount - pot_change.delta).abs() < 2.0 {
            for action in actions.iter_mut() {
                action.confidence = (action.confidence + 0.1).min(1.0);
                action.source = ActionSource::Combined;
            }
        }
    }

    fn compute_to_call(&self, state: &TableState) -> f64 {
        state.seats.iter().map(|s| s.current_bet).fold(0.0, f64::max)
    }

    /// 去重需要按 (seat_id, street, 物理动作语义) 联合判定，
    /// 不能只用 seat_id（同一桌的同一座位在不同 street 上会有合法的多次动作）。
    fn deduplicate(&self, actions: &mut Vec<ReconstructedAction>) {
        let mut seen: std::collections::HashSet<(SeatId, Street, std::mem::Discriminant<ActionType>)>
            = std::collections::HashSet::new();
        actions.retain(|a| {
            seen.insert((a.seat_id, a.street, std::mem::discriminant(&a.action_type)))
        });
    }
}
```

**动作推导核心逻辑：**

```
场景示例：

State Frame N:
  Seat3 stack=120, current_bet=0
  Seat5 stack=80,  current_bet=30
  Pot=30

State Frame N+1:
  Seat3 stack=90,  current_bet=30  (stack -30, bet +30)
  Seat5 stack=80,  current_bet=30  (unchanged)
  Pot=60  (+30)

推导:
  Stack Diff: Seat3 lost 30
  Pot Diff: +30
  Match: Seat3's lost 30 matches pot increase
  Conclusion: Seat3 called 30 (action=Call, amount=30)
  Confidence: 0.9 (stack match) * 1.1 (pot verified) = 0.95
```

### 8.10 Street Detection Module

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

pub struct StreetDetector;

impl StreetDetector {
    pub fn detect(community_cards: &[Card]) -> Street {
        match community_cards.len() {
            0 => Street::Preflop,
            3 => Street::Flop,
            4 => Street::Turn,
            5 => Street::River,
            _ => Street::Preflop,
        }
    }

    pub fn detect_transition(prev: Street, curr: Street) -> Option<StreetTransition> {
        match (prev, curr) {
            (Street::Preflop, Street::Flop) => Some(StreetTransition::Flop),
            (Street::Flop, Street::Turn) => Some(StreetTransition::Turn),
            (Street::Turn, Street::River) => Some(StreetTransition::River),
            (Street::River, Street::Preflop) => Some(StreetTransition::NewHand),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StreetTransition {
    Flop,
    Turn,
    River,
    Showdown,
    NewHand,
}
```

### 8.11 Hero Detection Module

**职责**：判定哪一个 Seat 是当前用户（Hero）。这是推荐引擎能否工作的前提。

判定策略（按优先级）：

1. **手动校准（MVP 必备）**：用户在校准阶段点击自己所在的 Seat，持久化到 `TableCalibration.hero_seat`。
2. **正面手牌检测（自动）**：扑克客户端中只有 hero 的两张手牌是正面（可识别 Suit/Rank），其余玩家显示牌背。一旦在某个 seat 的 `card_region` 上识别出有效卡牌，该 seat 即为 hero。
3. **下注控件位置**：FOLD/CALL/RAISE 按钮一般贴在 hero 座位附近，可作为弱信号验证。

```rust
pub struct HeroDetector {
    manual_hero: Option<SeatId>,
    last_detected: Option<(SeatId, Instant)>,
    confidence_window: Duration,
}

impl HeroDetector {
    pub fn detect(
        &mut self,
        seat_rois: &[SeatROI],
        frame: &Mat,
        card_detector: &CardDetector,
    ) -> Option<SeatId> {
        // 优先级 1：手动校准
        if let Some(seat) = self.manual_hero {
            return Some(seat);
        }

        // 优先级 2：哪个座位的 card_region 能识别出正面牌
        for (idx, seat_roi) in seat_rois.iter().enumerate() {
            if let Some(card_region) = seat_roi.card_region {
                let img = match Mat::roi(frame, card_region) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if card_detector.is_face_up_card(&img) {
                    let seat = SeatId(idx as u8);
                    self.last_detected = Some((seat, Instant::now()));
                    return Some(seat);
                }
            }
        }

        // 优先级 3：confidence_window 内还信任上次结果
        if let Some((seat, t)) = self.last_detected {
            if t.elapsed() < self.confidence_window {
                return Some(seat);
            }
        }
        None
    }
}
```

> **注意**：MVP 阶段 `manual_hero` 必须填写，**不能依赖自动检测**。
> 自动检测是 v1.1 能力，在校准 UI 中应当显式提示用户选择自己座位。

### 8.12 Feature Aggregator

```rust
#[derive(Debug, Clone)]
pub struct RawFeatures {
    pub cards: CardDetectionResult,
    pub stacks: Vec<StackChange>,
    pub pot: Option<PotChange>,
    pub seats: Vec<SeatChange>,
    pub dealer: Option<SeatId>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct ExtractedFeatures {
    pub table_id: TableId,
    pub timestamp: Instant,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub street: Street,
    pub stack_changes: Vec<StackChange>,
    pub pot_change: Option<PotChange>,
    pub seat_changes: Vec<SeatChange>,
    pub dealer_seat: Option<SeatId>,
}

pub struct FeatureAggregator;

impl FeatureAggregator {
    pub fn merge(&self, raw: RawFeatures) -> ExtractedFeatures {
        ExtractedFeatures {
            table_id: TableId::default(),
            timestamp: raw.timestamp,
            hole_cards: raw.cards.hole_cards,
            community_cards: raw.cards.community_cards,
            street: StreetDetector::detect(&raw.cards.community_cards),
            stack_changes: raw.stacks,
            pot_change: raw.pot,
            seat_changes: raw.seats,
            dealer_seat: raw.dealer,
        }
    }
}
```

---

## 9. ONNX 推理架构

### 9.1 模型清单

| 模型 | 输入 | 输出 | 用途 | 推理频率 |
|------|------|------|------|----------|
| card_classifier.onnx | 1x3x90x64 (RGB) | 52-class softmax | 卡牌分类 | 每帧每牌区 (<=7次) |
| card_detector.onnx | 1x3xHxW (ROI) | Bounding boxes | 卡牌定位（备选） | 校准时 |
| digit_recognizer.onnx | 1x1x32x100 (Gray) | 10-class + blank | 数字 OCR（筹码/底池） | 校准/低频 |

### 9.2 Session Pool 设计

```rust
pub struct InferencePool {
    card_sessions: Vec<Arc<ort::Session>>,
    digit_sessions: Vec<Arc<ort::Session>>,
    card_queue: Arc<crossbeam::queue::ArrayQueue<Arc<ort::Session>>>,
    digit_queue: Arc<crossbeam::queue::ArrayQueue<Arc<ort::Session>>>,
    card_semaphore: Arc<Semaphore>,
    digit_semaphore: Arc<Semaphore>,
}

#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub card_model_path: PathBuf,
    pub digit_model_path: PathBuf,
    pub onnx_intra_threads: usize,
    pub onnx_inter_threads: usize,
    pub card_session_count: usize,
    pub digit_session_count: usize,
    pub use_gpu: bool,
}

// 注意：ort 2.x 的 Session / SessionBuilder 不实现 Clone，
// pool 的每个 Session 必须独立 commit_from_file 创建。
impl InferencePool {
    pub fn new(config: &InferenceConfig) -> Result<Self> {
        let card_pool_count = config.card_session_count.max(1);
        let digit_pool_count = config.digit_session_count.max(1);

        let card_sessions: Vec<Arc<ort::Session>> = (0..card_pool_count)
            .map(|_| {
                let session = ort::Session::builder()?
                    .with_intra_threads(config.onnx_intra_threads)?
                    .with_inter_threads(config.onnx_inter_threads)?
                    .commit_from_file(&config.card_model_path)?;
                Ok(Arc::new(session))
            })
            .collect::<Result<Vec<_>>>()?;

        let digit_sessions: Vec<Arc<ort::Session>> = (0..digit_pool_count)
            .map(|_| {
                let session = ort::Session::builder()?
                    .with_intra_threads(1)?
                    .commit_from_file(&config.digit_model_path)?;
                Ok(Arc::new(session))
            })
            .collect::<Result<Vec<_>>>()?;

        // 用无锁队列做 round-robin 借/还，避免 index 0 永远被选中
        let card_queue = Arc::new(crossbeam::queue::ArrayQueue::new(card_pool_count));
        for s in &card_sessions {
            let _ = card_queue.push(s.clone());
        }
        let digit_queue = Arc::new(crossbeam::queue::ArrayQueue::new(digit_pool_count));
        for s in &digit_sessions {
            let _ = digit_queue.push(s.clone());
        }

        Ok(Self {
            card_sessions,
            digit_sessions,
            card_queue,
            digit_queue,
            card_semaphore: Arc::new(Semaphore::new(card_pool_count)),
            digit_semaphore: Arc::new(Semaphore::new(digit_pool_count)),
        })
    }

    pub async fn classify_card(&self, input: TensorData) -> Result<CardClassification> {
        // permit 必须覆盖整个推理过程，否则并发数限制失效
        let _permit = self.card_semaphore.acquire().await?;
        let session = loop {
            if let Some(s) = self.card_queue.pop() {
                break s;
            }
            tokio::task::yield_now().await;
        };

        let queue = self.card_queue.clone();
        let session_for_task = session.clone();
        let output = tokio::task::spawn_blocking(move || {
            session_for_task.run(ort::inputs![input]?)
        })
        .await??;

        // 归还
        let _ = queue.push(session);

        Ok(self.parse_card_output(&output))
    }
}
```

### 9.3 推理执行后端优先级

```
1. CUDA (NVIDIA GPU)     → 最高性能
2. DirectML (任意 GPU)    → Windows 通用 GPU 加速
3. CPU (fallback)        → 无 GPU 时可用
```

---

## 10. OCR 模块 (PaddleOCR)

### 10.1 定位与约束

**OCR 在本系统中的定位：纯辅助，非主路径。**

```
禁止:
  ✗ OCR 识别 "FOLD" / "CALL" / "RAISE" 按钮文字
  ✗ OCR 识别玩家名字
  ✗ OCR 作为主要筹码识别手段
  ✗ OCR-first 架构

允许:
  ✓ 底池数字识别（辅助验证）
  ✓ 筹码数字校准（初始建立 pixel↔value 映射）
  ✓ 特定场景下的数字辅助（如新牌桌首次校准）
```

### 10.2 PaddleOCR 集成方式

```rust
pub struct OcrAssistant {
    digit_session: Arc<ort::Session>,
    enabled: bool,
}

impl OcrAssistant {
    pub fn recognize_digits(&self, roi: &Mat) -> Option<f64> {
        if !self.enabled {
            return None;
        }

        let preprocessed = self.preprocess_for_digits(roi);
        let result = self.run_digit_model(&preprocessed)?;
        self.parse_number(result)
    }

    fn preprocess_for_digits(&self, img: &Mat) -> Mat {
        let mut gray = Mat::default();
        opencv::imgproc::cvt_color(img, &mut gray, opencv::imgproc::COLOR_BGR2GRAY, 0).unwrap();

        let mut binary = Mat::default();
        opencv::imgproc::threshold(
            &gray, &mut binary, 0.0, 255.0,
            opencv::imgproc::THRESH_BINARY | opencv::imgproc::THRESH_OTSU
        ).unwrap();

        let mut resized = Mat::default();
        opencv::imgproc::resize(
            &binary, &mut resized,
            opencv::core::Size::new(100, 32), 0.0, 0.0,
            opencv::imgproc::INTER_LINEAR
        ).unwrap();

        resized
    }

    fn parse_number(&self, digits: Vec<(char, f32)>) -> Option<f64> {
        let num_str: String = digits.iter().map(|(c, _)| *c).collect();
        num_str.parse().ok()
    }
}
```

### 10.3 OCR 使用场景清单

| 场景 | 触发条件 | 频率 | 作用 |
|------|----------|------|------|
| 初始筹码校准 | 新牌桌检测到 | 1次/桌 | 建立 pixel↔value 映射 |
| 底池数字验证 | 底池区域变化 | 每次变化 | 验证 pot diff 推导 |
| 筹码漂移修正 | 定时校验 | 每 30s | 修正像素估算累积误差 |
| 新手牌筹码快照 | 新手牌开始 | 每手牌 | 确认初始筹码量 |

---

## 11. 状态机架构

### 11.1 状态机总览

```
┌─────────────────────────────────────────────────────────────┐
│                     TableStateMachine                        │
│                                                             │
│  ┌──────────┐  new hand    ┌───────────┐  3 cards    ┌─────┤
│  │  WAITING │─────────────▶│  PREFLOP  │────────────▶│FLOP │
│  └──────────┘              └───────────┘             └─────┤
│       ▲                          │                     │    │
│       │                          │                     │    │
│       │                    showdown/timeout            │    │
│       │                          │                     │    │
│       │                          ▼                     ▼    │
│       │                    ┌───────────┐  1 card    ┌─────┤
│       │                    │ SHOWDOWN  │◀──────────│TURN ││
│       │                    └───────────┘             └─────┤
│       │                          │                     │    │
│       │                    reset/cleanup               │    │
│       │                          │               1 card│    │
│       │                          ▼                     ▼    │
│       └──────────────────── ┌───────────┐         ┌───────┐│
│                              │  CLEANUP  │◀────────│RIVER  ││
│                              └───────────┘         └───────┘│
│                                                             │
│  Internal State:                                            │
│    - TableState (complete game snapshot)                    │
│    - ActionHistory (all actions this hand)                  │
│    - StreetHistory (street transitions)                     │
│    - ConfidenceScore (state confidence)                     │
└─────────────────────────────────────────────────────────────┘
```

### 11.2 TableState 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub table_id: TableId,
    pub phase: TablePhase,
    pub street: Street,
    pub hand_number: u64,
    pub dealer_seat: Option<SeatId>,
    pub hero_seat: Option<SeatId>,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub pot: PotInfo,
    pub seats: Vec<SeatState>,
    pub action_history: Vec<ActionRecord>,
    pub current_player_turn: Option<SeatId>,
    pub blinds: BlindsInfo,
    pub last_update: f64,
    pub state_confidence: f32,
}

/// 盲注 / Ante / Straddle 配置（每桌一份）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlindsInfo {
    pub small_blind: f64,
    pub big_blind: f64,
    pub ante: f64,
    pub straddle: f64,
    /// 当前轮中的最大下注额（计算 to_call、min_raise 用）
    pub current_max_bet: f64,
    /// 当前 street 上一次合法加注的"加注幅度"（min_raise 计算）
    pub last_raise_size: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TablePhase {
    Waiting,
    Playing,
    Showdown,
    Cleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotInfo {
    pub main_pot: f64,
    pub side_pots: Vec<SidePot>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidePot {
    pub amount: f64,
    pub eligible_seats: Vec<SeatId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatState {
    pub seat_id: SeatId,
    pub status: SeatStatus,
    pub stack: f64,
    pub current_bet: f64,
    pub total_bet_this_hand: f64,
    pub last_action: Option<ActionRecord>,
    pub is_hero: bool,
    pub has_cards: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub seat_id: SeatId,
    pub action: ActionType,
    pub amount: f64,
    pub street: Street,
    pub seq: u32,
    pub confidence: f32,
}
```

### 11.3 State Machine 实现

```rust
use std::collections::VecDeque;

pub struct TableStateMachine {
    table_id: TableId,
    state: TableState,
    event_log: VecDeque<TableEvent>,
    action_seq: u32,
    hand_seq: u64,
}

#[derive(Debug, Clone)]
pub enum TableEvent {
    NewHandDetected { dealer_seat: SeatId },
    HoleCardsDetected { cards: [Card; 2] },
    CommunityCardsChanged { cards: Vec<Card>, street: Street },
    ActionReconstructed { action: ReconstructedAction },
    PotChanged { new_total: f64, delta: f64 },
    SeatStatusChanged { seat_id: SeatId, new_status: SeatStatus },
    DealerButtonMoved { new_seat: SeatId },
    Timeout,
}

#[derive(Debug, Clone)]
pub enum StateTransition {
    HandStarted { hand_number: u64 },
    StreetChanged { from: Street, to: Street },
    ActionRecorded(ActionRecord),
    HandCompleted { result: HandResult },
    StateConfidenceUpdated { new_confidence: f32 },
}

impl TableStateMachine {
    pub fn new(table_id: TableId) -> Self {
        Self {
            table_id,
            state: TableState::initial(table_id),
            // 一手牌可达 30-40 events × 多个玩家，按 1000 容纳近 5-10 手牌
            event_log: VecDeque::with_capacity(1000),
            action_seq: 0,
            hand_seq: 0,
        }
    }

    pub fn process_event(&mut self, event: TableEvent) -> Vec<StateTransition> {
        const EVENT_LOG_MAX: usize = 1000;
        let transitions = match &event {
            TableEvent::NewHandDetected { dealer_seat } => {
                self.handle_new_hand(*dealer_seat)
            }
            TableEvent::HoleCardsDetected { cards } => {
                self.state.hole_cards = Some(*cards);
                self.state.state_confidence = (self.state.state_confidence + 0.05).min(1.0);
                vec![]
            }
            TableEvent::CommunityCardsChanged { cards, street } => {
                self.handle_community_change(cards.clone(), *street)
            }
            TableEvent::ActionReconstructed { action } => {
                self.handle_action(action.clone())
            }
            TableEvent::PotChanged { new_total, .. } => {
                self.state.pot.total = *new_total;
                self.state.pot.main_pot = *new_total;
                vec![]
            }
            TableEvent::SeatStatusChanged { seat_id, new_status } => {
                if let Some(seat) = self.state.seats.iter_mut().find(|s| s.seat_id == *seat_id) {
                    seat.status = new_status.clone();
                }
                vec![]
            }
            TableEvent::DealerButtonMoved { new_seat } => {
                self.state.dealer_seat = Some(*new_seat);
                vec![]
            }
            TableEvent::Timeout => {
                self.state.state_confidence = (self.state.state_confidence - 0.05).max(0.5);
                vec![StateTransition::StateConfidenceUpdated {
                    new_confidence: self.state.state_confidence,
                }]
            }
        };

        self.event_log.push_back(event);
        if self.event_log.len() > EVENT_LOG_MAX {
            self.event_log.pop_front();
        }

        transitions
    }

    fn handle_new_hand(&mut self, dealer_seat: SeatId) -> Vec<StateTransition> {
        self.hand_seq += 1;
        self.action_seq = 0;

        self.state.phase = TablePhase::Playing;
        self.state.street = Street::Preflop;
        self.state.hand_number = self.hand_seq;
        self.state.dealer_seat = Some(dealer_seat);
        self.state.hole_cards = None;
        self.state.community_cards.clear();
        self.state.pot = PotInfo::default();
        self.state.action_history.clear();

        for seat in &mut self.state.seats {
            seat.current_bet = 0.0;
            seat.total_bet_this_hand = 0.0;
            seat.last_action = None;
            seat.has_cards = true;
            if seat.status == SeatStatus::Folded {
                seat.status = SeatStatus::Active;
            }
        }

        vec![StateTransition::HandStarted { hand_number: self.hand_seq }]
    }

    fn handle_community_change(&mut self, cards: Vec<Card>, new_street: Street) -> Vec<StateTransition> {
        let old_street = self.state.street;
        self.state.community_cards = cards;
        self.state.street = new_street;

        for seat in &mut self.state.seats {
            seat.current_bet = 0.0;
        }

        vec![StateTransition::StreetChanged { from: old_street, to: new_street }]
    }

    fn handle_action(&mut self, action: ReconstructedAction) -> Vec<StateTransition> {
        self.action_seq += 1;

        let record = ActionRecord {
            seat_id: action.seat_id,
            action: action.action_type.clone(),
            amount: action.amount.unwrap_or(0.0),
            street: action.street,
            seq: self.action_seq,
            confidence: action.confidence,
        };

        if let Some(seat) = self.state.seats.iter_mut().find(|s| s.seat_id == action.seat_id) {
            match &action.action_type {
                ActionType::Fold => {
                    seat.status = SeatStatus::Folded;
                    seat.has_cards = false;
                }
                ActionType::Check => {}
                ActionType::Call => {
                    seat.stack -= record.amount;
                    seat.current_bet += record.amount;
                    seat.total_bet_this_hand += record.amount;
                }
                ActionType::Bet(amount) => {
                    seat.stack -= amount;
                    seat.current_bet += amount;
                    seat.total_bet_this_hand += amount;
                    self.state.blinds.last_raise_size = *amount;
                    self.state.blinds.current_max_bet = seat.current_bet;
                }
                ActionType::Raise(total) => {
                    let raise_amount = total - seat.current_bet;
                    let raise_increment = total - self.state.blinds.current_max_bet;
                    seat.stack -= raise_amount;
                    seat.current_bet = *total;
                    seat.total_bet_this_hand += raise_amount;
                    self.state.blinds.last_raise_size = raise_increment.max(self.state.blinds.big_blind);
                    self.state.blinds.current_max_bet = *total;
                }
                ActionType::AllIn(_total) => {
                    let allin_amount = seat.stack;
                    seat.stack = 0.0;
                    seat.current_bet += allin_amount;
                    seat.total_bet_this_hand += allin_amount;
                    seat.status = SeatStatus::AllIn;
                    if seat.current_bet > self.state.blinds.current_max_bet {
                        self.state.blinds.current_max_bet = seat.current_bet;
                    }
                }
                ActionType::PostBlind(kind) => {
                    seat.stack -= record.amount;
                    seat.current_bet += record.amount;
                    seat.total_bet_this_hand += record.amount;
                    match kind {
                        BlindKind::SmallBlind => {
                            self.state.blinds.small_blind = record.amount;
                        }
                        BlindKind::BigBlind => {
                            self.state.blinds.big_blind = record.amount;
                            self.state.blinds.current_max_bet = record.amount;
                            self.state.blinds.last_raise_size = record.amount;
                        }
                        BlindKind::Straddle => {
                            self.state.blinds.straddle = record.amount;
                            self.state.blinds.current_max_bet = record.amount;
                        }
                        BlindKind::Ante => {
                            self.state.blinds.ante = record.amount;
                            // Ante 不计入 current_max_bet
                            seat.current_bet -= record.amount;
                        }
                    }
                }
            }
            seat.last_action = Some(record.clone());
        }

        // 推进当前行动玩家
        self.advance_turn();

        self.state.action_history.push(record.clone());
        vec![StateTransition::ActionRecorded(record)]
    }

    /// 推进 current_player_turn 到下一个 Active 玩家
    fn advance_turn(&mut self) {
        let n = self.state.seats.len();
        if n == 0 { return; }
        let start = self.state.current_player_turn
            .map(|s| s.0 as usize)
            .unwrap_or(0);
        for i in 1..=n {
            let idx = (start + i) % n;
            let seat = &self.state.seats[idx];
            if matches!(seat.status, SeatStatus::Active) {
                self.state.current_player_turn = Some(seat.seat_id);
                return;
            }
        }
        self.state.current_player_turn = None;
    }

    pub fn get_state(&self) -> &TableState {
        &self.state
    }

    pub fn get_snapshot(&self) -> TableState {
        self.state.clone()
    }
}
```

### 11.4 BettingRoundEngine（行动顺序 / 回合关闭）

德州扑克的 street 切换不是简单的 "公共牌从 0→3 张就进 Flop"，
而是 **"所有未弃牌玩家在当前 street 上至少 act 一次，且所有 active 玩家的 current_bet 相等"**
才允许进入下一条 street。`BettingRoundEngine` 专门维护这个语义。

```rust
pub struct BettingRoundEngine;

impl BettingRoundEngine {
    /// 当前 street 是否可以关闭（所有人 act 完且下注相等）
    pub fn is_round_complete(state: &TableState) -> bool {
        let active: Vec<&SeatState> = state.seats.iter()
            .filter(|s| matches!(s.status, SeatStatus::Active))
            .collect();
        if active.len() < 2 { return true; } // 只剩一人，直接结束

        let max_bet = state.blinds.current_max_bet;
        let all_matched = active.iter().all(|s| (s.current_bet - max_bet).abs() < 0.01);

        // 所有 active 玩家本 street 都已 act
        let all_acted = active.iter().all(|s| {
            state.action_history.iter().any(|a| {
                a.seat_id == s.seat_id && a.street == state.street
                    && !matches!(a.action, ActionType::PostBlind(_))
            })
        });

        all_matched && all_acted
    }

    /// 计算 Hero 视角的 to_call
    pub fn to_call_for(state: &TableState, seat_id: SeatId) -> f64 {
        let seat = match state.seats.iter().find(|s| s.seat_id == seat_id) {
            Some(s) => s,
            None => return 0.0,
        };
        (state.blinds.current_max_bet - seat.current_bet).max(0.0)
    }

    /// min_raise = max(last_raise_size, big_blind)
    pub fn min_raise(state: &TableState) -> f64 {
        state.blinds.last_raise_size.max(state.blinds.big_blind)
    }
}
```

### 11.5 State Validation

```rust
pub struct StateValidator;

impl StateValidator {
    pub fn validate(state: &TableState) -> ValidationResult {
        let mut issues = Vec::new();

        let total_bets: f64 = state.seats.iter().map(|s| s.total_bet_this_hand).sum();
        // 阈值改为相对 BB 的倍数（NL2 与 NL500 上 5.0 含义完全不同）
        let bb = state.blinds.big_blind.max(0.01);
        let tolerance = (bb * 0.5).max(state.pot.total * 0.02);
        if (state.pot.total - total_bets).abs() > tolerance {
            issues.push(ValidationIssue::PotBetMismatch {
                pot: state.pot.total,
                total_bets,
            });
        }

        for seat in &state.seats {
            if seat.stack < 0.0 {
                issues.push(ValidationIssue::NegativeStack {
                    seat_id: seat.seat_id,
                    stack: seat.stack,
                });
            }
        }

        let expected = match state.street {
            Street::Preflop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River | Street::Showdown => 5,
        };
        if state.community_cards.len() != expected {
            issues.push(ValidationIssue::CardStreetMismatch {
                cards: state.community_cards.len(),
                street: state.street,
            });
        }

        let mut all_cards = state.community_cards.clone();
        if let Some(hole) = &state.hole_cards {
            all_cards.extend_from_slice(hole);
        }
        // Card 包含 confidence: f32，不能直接 Hash；按 (Suit, Rank) 判定唯一性
        let unique: std::collections::HashSet<(Suit, Rank)> =
            all_cards.iter().map(|c| (c.suit, c.rank)).collect();
        if all_cards.len() != unique.len() {
            issues.push(ValidationIssue::DuplicateCards);
        }

        if issues.is_empty() { ValidationResult::Valid } else { ValidationResult::Issues(issues) }
    }
}
```

---

## 12. 推荐引擎集成

### 12.0 集成方案选型（决策）

`prompt.md` 已明确 RecEngine 是 **TypeScript SDK**，跨语言桥接有两条路径：

| 方案 | 优点 | 缺点 | 选择 |
|------|------|------|------|
| **MVP：Sidecar Node.js 子进程 + JSON-RPC over stdio** | 不重写 SDK；崩溃隔离；快速跑起来 | 一次进程跳转 1-3ms | ✅ MVP |
| v1.1：将 SDK port 到 Rust（或编译为 WASM 嵌入） | 端到端无跨进程；最低延迟 | 工作量大、需要保持与 TS 主线同步 | 后续 |

**Sidecar 协议**（MVP）：

```
进程：node ./rec-sidecar/index.js  (由 tf-rec crate 在启动时 spawn)
传输：stdio（stdin/stdout，按行 JSON）
协议：JSON-RPC 2.0
方法：
  - rec.recommend(input) -> RecOutput
  - rec.health()         -> { ok: true, version: "..." }
  - rec.shutdown()       -> ok
超时：500ms（默认），超时后单帧降级为"上次推荐 + 置信度衰减"
重启：连续 3 次超时或非零退出码自动重启 sidecar
```

```rust
pub struct RecSidecar {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
}

impl RecSidecar {
    pub async fn recommend(&self, input: RecInput) -> Result<RecOutput> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "rec.recommend",
            "params": input,
        });
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let line = serde_json::to_string(&req)? + "\n";
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        let resp = tokio::time::timeout(Duration::from_millis(500), rx).await??;
        let output: RecOutput = serde_json::from_value(resp)?;
        Ok(output)
    }
}
```

### 12.1 Engine Interface

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecInput {
    pub hole_cards: [Card; 2],
    pub community_cards: Vec<Card>,
    pub pot: f64,
    pub to_call: f64,
    pub min_raise: f64,
    pub stack: f64,
    pub street: Street,
    pub num_opponents: usize,
    pub action_history: Vec<ActionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecOutput {
    pub action: String,
    pub amount: f64,
    pub confidence: f64,
    pub distribution: HashMap<String, f64>,
    pub ev: f64,
    pub processing_time_ms: f64,
}

pub struct RecEngine {
    cache: HashMap<String, RecOutput>,
}

impl RecEngine {
    pub async fn recommend(&mut self, input: RecInput) -> Result<RecOutput> {
        let cache_key = self.compute_cache_key(&input);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let start = Instant::now();
        let result = self.call_rec_sdk(&input).await?;

        let output = RecOutput {
            action: result.action,
            amount: result.amount,
            confidence: result.confidence,
            distribution: result.distribution,
            ev: result.ev,
            processing_time_ms: start.elapsed().as_secs_f64() * 1000.0,
        };

        self.cache.insert(cache_key, output.clone());
        Ok(output)
    }

    /// Cache key 必须包含 action_history 的关键摘要，否则同样的 pot/stack/cards
    /// 但不同 betting line（例如 3bet pot vs 单挑 pot）会被错误命中同一缓存。
    /// GTO 推荐对 line 是极度敏感的。
    fn compute_cache_key(&self, input: &RecInput) -> String {
        // 把 action_history 摘要成 (seat, action_kind, amount_bb) 序列
        let history_digest: String = input.action_history.iter()
            .map(|a| format!("{}:{:?}:{:.1}", a.seat_id.0, std::mem::discriminant(&a.action), a.amount))
            .collect::<Vec<_>>()
            .join("|");

        format!(
            "{:?}|{:?}|p{:.2}|c{:.2}|s{:.2}|m{:.2}|{:?}|n{}|h[{}]",
            input.hole_cards, input.community_cards, input.pot,
            input.to_call, input.stack, input.min_raise,
            input.street, input.num_opponents, history_digest,
        )
    }
}
```

### 12.2 SDK 输入映射

> 前提：`state.hero_seat` 已被 `HeroDetector` 填充（手动校准或自动检测）。
> 没有 Hero 时不应调用推荐引擎。

```
TableState → RecInput mapping:

  hole_cards       ← state.hole_cards (Option<[Card; 2]>)
  community_cards  ← state.community_cards (Vec<Card>)
  pot              ← state.pot.total
  to_call          ← BettingRoundEngine::to_call_for(state, hero_seat)
  min_raise        ← BettingRoundEngine::min_raise(state)   // = max(last_raise_size, BB)
  stack            ← seats[hero_seat].stack
  street           ← state.street (Preflop/Flop/Turn/River)
  num_opponents    ← count(seats where status == Active && seat_id != hero_seat)
  action_history   ← state.action_history.iter().filter(|a| !is_postblind(a))
                       // 盲注/Ante 不进 GTO 推荐的 history
```

```rust
pub fn build_rec_input(state: &TableState) -> Option<RecInput> {
    let hero = state.hero_seat?;
    let hole_cards = state.hole_cards?;
    let hero_state = state.seats.iter().find(|s| s.seat_id == hero)?;

    Some(RecInput {
        hole_cards,
        community_cards: state.community_cards.clone(),
        pot: state.pot.total,
        to_call: BettingRoundEngine::to_call_for(state, hero),
        min_raise: BettingRoundEngine::min_raise(state),
        stack: hero_state.stack,
        street: state.street,
        num_opponents: state.seats.iter()
            .filter(|s| matches!(s.status, SeatStatus::Active) && s.seat_id != hero)
            .count(),
        action_history: state.action_history.iter()
            .filter(|a| !matches!(a.action, ActionType::PostBlind(_)))
            .cloned()
            .collect(),
    })
}
```

---

## 13. IPC 协议设计

### 13.1 IPC 架构

```
┌──────────────────────┐         ┌──────────────────────┐
│   Electron Main      │         │   Rust Native Addon  │
│   Process            │         │   (napi-rs)          │
│                      │         │                      │
│   ipc.ts ◄──────────►│ napi-rs │◄────────► bridge.rs  │
│   (call/resolve)     │  calls  │  (dispatch)          │
│                      │         │                      │
│   event handler ◄────│ napi    │◄──────── events.rs   │
│   (on callback)      │  tsfn   │  (emit)              │
└──────────────────────┘         └──────────────────────┘
```

### 13.2 命令协议（Electron → Rust）

```rust
use napi_derive::napi;
use napi::*;

#[napi(ts_return_type = "Promise<void>")]
pub async fn start_capture(config: JsTableConfig) -> Result<()> {
    let bridge = get_bridge()?;
    bridge.start_capture(config.into()).await
}

#[napi(ts_return_type = "Promise<void>")]
pub async fn stop_capture(table_id: String) -> Result<()> {
    let bridge = get_bridge()?;
    bridge.stop_capture(table_id).await
}

#[napi(ts_return_type = "Promise<TableStateSnapshot>")]
pub async fn get_table_state(table_id: String) -> Result<JsTableState> {
    let bridge = get_bridge()?;
    let state = bridge.get_state(table_id).await?;
    Ok(state.into())
}

#[napi(ts_return_type = "Promise<CalibrationResult>")]
pub async fn calibrate_table(table_id: String) -> Result<JsCalibrationResult> {
    let bridge = get_bridge()?;
    bridge.calibrate(table_id).await
}

#[napi(ts_return_type = "Promise<string[]>")]
pub async fn discover_tables() -> Result<Vec<String>> {
    let bridge = get_bridge()?;
    bridge.discover_tables().await
}

// 后台线程要回调 JS 必须用 ThreadsafeFunction，
// JsFunction 只能在 JS 主线程上调用。
type StateTsfn = ThreadsafeFunction<StateUpdateEvent, ErrorStrategy::Fatal>;
type RecTsfn = ThreadsafeFunction<RecommendationEvent, ErrorStrategy::Fatal>;
type ErrorTsfn = ThreadsafeFunction<ErrorEvent, ErrorStrategy::Fatal>;

#[napi]
pub fn on_state_update(callback: JsFunction) -> Result<()> {
    let bridge = get_bridge()?;
    let tsfn: StateTsfn = callback.create_threadsafe_function(0, |ctx| {
        Ok(vec![ctx.value])
    })?;
    bridge.register_state_callback(tsfn)
}

#[napi]
pub fn on_recommendation(callback: JsFunction) -> Result<()> {
    let bridge = get_bridge()?;
    let tsfn: RecTsfn = callback.create_threadsafe_function(0, |ctx| {
        Ok(vec![ctx.value])
    })?;
    bridge.register_rec_callback(tsfn)
}

#[napi]
pub fn on_error(callback: JsFunction) -> Result<()> {
    let bridge = get_bridge()?;
    let tsfn: ErrorTsfn = callback.create_threadsafe_function(0, |ctx| {
        Ok(vec![ctx.value])
    })?;
    bridge.register_error_callback(tsfn)
}
```

### 13.3 事件协议（Rust → Electron）

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateUpdateEvent {
    pub table_id: String,
    pub state: JsTableState,
    pub transition: Option<JsTransition>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationEvent {
    pub table_id: String,
    pub recommendation: JsRecOutput,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsTableState {
    pub table_id: String,
    pub phase: String,
    pub street: String,
    pub hand_number: f64,
    pub dealer_seat: Option<f64>,
    pub hole_cards: Option<Vec<JsCard>>,
    pub community_cards: Vec<JsCard>,
    pub pot: f64,
    pub seats: Vec<JsSeat>,
    pub last_action: Option<JsAction>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsCard {
    pub suit: String,
    pub rank: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsSeat {
    pub seat_id: f64,
    pub status: String,
    pub stack: f64,
    pub current_bet: f64,
    pub last_action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsRecOutput {
    pub action: String,
    pub amount: f64,
    pub confidence: f64,
    pub distribution: HashMap<String, f64>,
}
```

### 13.4 IPC Bridge

```rust
pub struct NapiBridge {
    table_manager: Arc<tokio::sync::Mutex<TableManager>>,
    state_callback: Arc<Mutex<Option<ThreadsafeFunction<JsObject>>>>,
    rec_callback: Arc<Mutex<Option<ThreadsafeFunction<JsObject>>>>,
    error_callback: Arc<Mutex<Option<ThreadsafeFunction<JsObject>>>>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl NapiBridge {
    pub fn emit_state_update(&self, event: StateUpdateEvent) -> Result<()> {
        let callback = self.state_callback.lock().unwrap();
        if let Some(tsfn) = callback.as_ref() {
            let json = serde_json::to_string(&event).unwrap();
            tsfn.call(Ok(json), ThreadsafeFunctionCallMode::NonBlocking);
        }
        Ok(())
    }

    pub fn emit_recommendation(&self, event: RecommendationEvent) -> Result<()> {
        let callback = self.rec_callback.lock().unwrap();
        if let Some(tsfn) = callback.as_ref() {
            let json = serde_json::to_string(&event).unwrap();
            tsfn.call(Ok(json), ThreadsafeFunctionCallMode::NonBlocking);
        }
        Ok(())
    }
}
```

---

## 14. Overlay HUD 架构

### 14.1 Overlay 窗口设计

```
┌─────────────────────────────────────────────────────────────┐
│  Poker Client Window                                         │
│                                                             │
│    ┌──────────────────────────────────────────────────┐     │
│    │  Transparent Overlay BrowserWindow                │     │
│    │  (same size & position as poker window)          │     │
│    │                                                    │     │
│    │  ┌─────────────────────────────────┐              │     │
│    │  │  Recommendation Panel           │              │     │
│    │  │  ┌───────────────────────────┐  │              │     │
│    │  │  │  RAISE to 6.0            │  │              │     │
│    │  │  │  Confidence: 87%         │  │              │     │
│    │  │  │  EV: +2.3 BB            │  │              │     │
│    │  │  └───────────────────────────┘  │              │     │
│    │  │  ┌───────────────────────────┐  │              │     │
│    │  │  │  Fold 3% | Call 15%      │  │              │     │
│    │  │  │  Raise 82%               │  │              │     │
│    │  │  └───────────────────────────┘  │              │     │
│    │  └─────────────────────────────────┘              │     │
│    │                                                    │     │
│    │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐             │     │
│    │  │Seat1 │ │Seat3 │ │Seat5 │ │Seat7 │             │     │
│    │  │VPIP  │ │PFR   │ │AF    │ │Hands │             │     │
│    │  └──────┘ └──────┘ └──────┘ └──────┘             │     │
│    └──────────────────────────────────────────────────┘     │
│                                                             │
│   [ Poker Table Graphics underneath ]                       │
└─────────────────────────────────────────────────────────────┘

  Overlay Properties:
  - Transparent background
  - Click-through (setIgnoreMouseEvents)
  - Always on top
  - No frame / no title bar
  - Position synced with poker client window
  - Per-table overlay (one BrowserWindow per poker table)
```

### 14.2 Overlay Window 创建

```typescript
import { BrowserWindow } from 'electron';

interface OverlayConfig {
  targetWindowTitle: string;
  tableId: string;
}

export async function createOverlayWindow(config: OverlayConfig): Promise<BrowserWindow> {
  const targetBounds = await findWindowBounds(config.targetWindowTitle);

  const overlay = new BrowserWindow({
    x: targetBounds.x,
    y: targetBounds.y,
    width: targetBounds.width,
    height: targetBounds.height,
    transparent: true,
    frame: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    hasShadow: false,
    resizable: false,
    focusable: false,
    show: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, '../preload/index.js'),
    },
  });

  overlay.setIgnoreMouseEvents(true, { forward: true });
  overlay.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true });

  await overlay.loadFile(path.join(__dirname, '../renderer/overlay.html'));
  overlay.show();

  startPositionSync(overlay, config.targetWindowTitle);

  return overlay;
}
```

### 14.3 Overlay UI 组件（SolidJS）

```tsx
import { createSignal, onCleanup, For, Show } from 'solid-js';

interface OverlayProps {
  tableId: string;
}

export function HudOverlay(props: OverlayProps) {
  const [state, setState] = createSignal<TableState | null>(null);
  const [recommendation, setRecommendation] = createSignal<RecOutput | null>(null);

  const unsubscribe = window.electronAPI.onStateUpdate((event: StateUpdateEvent) => {
    if (event.tableId === props.tableId) {
      setState(event.state);
    }
  });

  const unsubRec = window.electronAPI.onRecommendationUpdate((event: RecEvent) => {
    if (event.tableId === props.tableId) {
      setRecommendation(event.recommendation);
    }
  });

  onCleanup(() => {
    unsubscribe();
    unsubRec();
  });

  return (
    <div class="overlay-container">
      <Show when={recommendation()}>
        {(rec) => (
          <div class="recommendation-panel">
            <div class="rec-action" data-action={rec().action}>
              {rec().action} {rec().amount > 0 ? rec().amount : ''}
            </div>
            <div class="rec-confidence">
              Confidence: {(rec().confidence * 100).toFixed(0)}%
            </div>
            <div class="rec-distribution">
              <For each={Object.entries(rec().distribution)}>
                {([action, prob]) => (
                  <div class="dist-bar">
                    <span class="dist-label">{action}</span>
                    <div class="dist-fill" style={{ width: `${prob * 100}%` }} />
                    <span class="dist-value">{(prob * 100).toFixed(1)}%</span>
                  </div>
                )}
              </For>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}
```

---

## 15. Electron 架构

### 15.1 进程架构

```
┌─────────────────────────────────────────────────────────┐
│                    Main Process                           │
│                                                           │
│  index.ts ──┬── window.ts (window management)            │
│             ├── overlay.ts (overlay windows)              │
│             ├── tray.ts (system tray)                     │
│             ├── ipc.ts (IPC handlers)                     │
│             └── native.ts (native addon loader)           │
│                                                           │
│  Lifecycle:                                               │
│  1. app.whenReady() → create main window                  │
│  2. Load native addon (tf-napi.node)                      │
│  3. Start table discovery                                 │
│  4. For each table: create overlay window                 │
│  5. Wire native events → renderer IPC                     │
│  6. app.on('window-all-closed') → cleanup                 │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              Renderer Process (Main Window)               │
│                                                           │
│  SolidJS App                                              │
│  ├── Dashboard (table overview, session stats)            │
│  ├── Settings (calibration, preferences)                  │
│  └── History (action replay, hand history)                │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│           Renderer Process (Overlay Window) x N           │
│                                                           │
│  SolidJS Overlay                                          │
│  ├── HudOverlay (recommendation display)                  │
│  ├── PlayerStats (per-seat VPIP/PFR/AF)                  │
│  └── PotDisplay (pot/side-pot info)                       │
│                                                           │
│  Properties: transparent, click-through, always-on-top    │
└─────────────────────────────────────────────────────────┘
```

### 15.2 Main Process 入口

```typescript
import { app, BrowserWindow, ipcMain } from 'electron';
import { loadNative } from './native';
import { createMainWindow } from './window';
import { createOverlayWindow } from './overlay';

let native: NativeAddon | null = null;
const overlays = new Map<string, BrowserWindow>();

app.whenReady().then(async () => {
  native = loadNative();

  const mainWindow = createMainWindow();

  native.onStateUpdate((event) => {
    mainWindow.webContents.send('state:update', event);
    const overlay = overlays.get(event.tableId);
    if (overlay) overlay.webContents.send('state:update', event);
  });

  native.onRecommendation((event) => {
    mainWindow.webContents.send('recommendation:update', event);
    const overlay = overlays.get(event.tableId);
    if (overlay) overlay.webContents.send('recommendation:update', event);
  });

  ipcMain.handle('discover-tables', async () => {
    return native!.discoverTables();
  });

  ipcMain.handle('start-capture', async (_, config) => {
    await native!.startCapture(config);
    const overlay = await createOverlayWindow({
      targetWindowTitle: config.windowTitle,
      tableId: config.tableId,
    });
    overlays.set(config.tableId, overlay);
  });

  ipcMain.handle('stop-capture', async (_, tableId) => {
    await native!.stopCapture(tableId);
    const overlay = overlays.get(tableId);
    if (overlay) {
      overlay.close();
      overlays.delete(tableId);
    }
  });
});
```

### 15.3 Preload Bridge

```typescript
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
  onStateUpdate: (callback: (event: any) => void) => {
    const handler = (_: any, event: any) => callback(event);
    ipcRenderer.on('state:update', handler);
    return () => ipcRenderer.removeListener('state:update', handler);
  },
  onRecommendationUpdate: (callback: (event: any) => void) => {
    const handler = (_: any, event: any) => callback(event);
    ipcRenderer.on('recommendation:update', handler);
    return () => ipcRenderer.removeListener('recommendation:update', handler);
  },
  discoverTables: () => ipcRenderer.invoke('discover-tables'),
  startCapture: (config: any) => ipcRenderer.invoke('start-capture', config),
  stopCapture: (tableId: string) => ipcRenderer.invoke('stop-capture', tableId),
  getTableState: (tableId: string) => ipcRenderer.invoke('get-table-state', tableId),
  calibrateTable: (tableId: string) => ipcRenderer.invoke('calibrate-table', tableId),
});
```

---

## 16. 多桌并发方案

### 16.1 TableManager 架构

```rust
pub struct TableManager {
    tables: HashMap<String, TableHandle>,
    inference_pool: Arc<InferencePool>,
    config: ManagerConfig,
    event_tx: tokio::sync::broadcast::Sender<ManagerEvent>,
}

#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub max_tables: usize,
    pub fps_per_table: u32,
    pub capture_backend: CaptureBackend,
}

#[derive(Debug, Clone)]
pub enum CaptureBackend {
    Dxgi,
    WindowsGraphicsCapture,
}

#[derive(Debug, Clone)]
pub enum ManagerEvent {
    TableDiscovered { table_id: String, window_title: String },
    TableLost { table_id: String },
    StateUpdated { table_id: String, state: Box<TableState> },
    RecommendationReady { table_id: String, output: Box<RecOutput> },
    Error { table_id: String, error: String },
}

impl TableManager {
    pub fn new(config: ManagerConfig) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(1000);
        Self {
            tables: HashMap::new(),
            inference_pool: Arc::new(InferencePool::new(&config.inference_config).unwrap()),
            config,
            event_tx,
        }
    }

    pub async fn start_table(
        &mut self,
        table_id: String,
        window_handle: HWND,
        calibration: Option<TableCalibration>,
    ) -> Result<()> {
        if self.tables.len() >= self.config.max_tables {
            return Err(anyhow::anyhow!("Max tables reached"));
        }

        let handle = TableHandle::new(
            table_id.clone(),
            window_handle,
            calibration,
            self.inference_pool.clone(),
            self.config.fps_per_table,
        );

        self.tables.insert(table_id, handle);
        Ok(())
    }

    pub async fn stop_table(&mut self, table_id: &str) -> Result<()> {
        if let Some(mut handle) = self.tables.remove(table_id) {
            handle.shutdown().await;
        }
        Ok(())
    }

    pub fn get_state(&self, table_id: &str) -> Option<&TableState> {
        self.tables.get(table_id).map(|h| h.state_machine.get_state())
    }

    pub async fn shutdown_all(&mut self) {
        for (_, mut handle) in self.tables.drain() {
            handle.shutdown().await;
        }
    }
}
```

### 16.2 多桌资源分配

```
8桌场景资源分配:

CPU Cores: 8
├── Tokio Workers: 8 (async tasks)
│   ├── Capture Task x 8 (lightweight, mostly waiting)
│   ├── State Machine Task x 8 (sequential per table)
│   ├── IPC Bridge Task x 1
│   └── Discovery Task x 1
│
├── Rayon Pool: 6 threads (heavy computation)
│   ├── Feature Extraction (shared across all tables)
│   └── Template Matching (shared)
│
└── ONNX Runtime: 2 intra-op threads
    └── Card + Digit inference (shared session pool)

Memory per table: ~30MB (frame buffer + state history)
Total Memory: ~250MB + 100MB overhead = ~350MB
```

---

## 17. 完整数据模型

### 17.1 Core Types

```rust
pub type TableId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeatId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten,
    Jack, Queen, King, Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionType {
    Fold,
    Check,
    Call,
    Bet(f64),
    Raise(f64),
    AllIn(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SeatStatus {
    Empty,
    SittingOut,
    Active,
    Folded,
    AllIn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TablePhase {
    Waiting,
    Playing,
    Showdown,
    Cleanup,
}
```

### 17.2 Frame & Feature Types

```rust
pub struct CapturedFrame {
    pub timestamp: Instant,
    pub table_id: TableId,
    pub image: Mat,
    pub frame_number: u64,
    pub latency: Duration,
}

pub struct ExtractedFeatures {
    pub table_id: TableId,
    pub timestamp: Instant,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub street: Street,
    pub stack_changes: Vec<StackChange>,
    pub pot_change: Option<PotChange>,
    pub seat_changes: Vec<SeatChange>,
    pub dealer_seat: Option<SeatId>,
}

pub struct StackChange {
    pub seat_id: SeatId,
    pub prev_estimated: f64,
    pub curr_estimated: f64,
    pub delta: f64,
    pub confidence: f32,
}

pub struct PotChange {
    pub prev_value: f64,
    pub new_value: f64,
    pub delta: f64,
    pub timestamp: Instant,
}

pub struct SeatChange {
    pub seat_id: SeatId,
    pub prev_status: Option<SeatStatus>,
    pub new_status: SeatStatus,
}
```

---

## 18. 性能优化策略

### 18.1 Frame Processing Optimization

| 优化点 | 策略 | 预期收益 |
|--------|------|----------|
| 帧跳过 | Frame Diff 检测无变化则跳过 | 减少 60-80% 无效处理 |
| ROI 裁剪 | 只处理感兴趣区域 | 减少 90% 像素处理量 |
| 模板缓存 | 缓存模板匹配结果 | 避免重复计算 |
| SIMD 加速 | OpenCV 内置 SIMD 优化 | 2-4x 速度提升 |
| GPU 推理 | ONNX DirectML/CUDA | 10x 推理速度 |

### 18.2 Latency Budget

```
关键路径延迟预算:

Frame Capture:     5ms   (DXGI, zero-copy)
Preprocessing:     2ms   (resize, color convert)
ROI Extraction:    1ms   (crop)
Feature Extract:   8ms   (template match + ONNX)
State Diff:        2ms   (compare states)
Action Recon:      1ms   (derive actions)
State Update:      1ms   (state machine)
Recommendation:    5ms   (engine compute)
IPC Transfer:      3ms   (napi → Electron)
HUD Render:        2ms   (SolidJS reactive)
────────────────────────
Total:            ~30ms  (well under 100ms target)
```

### 18.3 CPU Optimization

```
策略:
1. Tokio + Rayon 分离: IO 任务 (Tokio) 与 CPU 任务 (Rayon) 分离
2. 共享推理池: 多桌共享 ONNX Session Pool
3. 避免锁竞争: per-table channel, 无全局锁
4. 零拷贝: DXGI → Mat → ROI 尽量引用
5. 批量推理: 同一帧的多个 card crop 合并 batch
```

### 18.4 Memory Optimization

```
策略:
1. 双缓冲帧: 仅保留 prev + current
2. BGR 格式: 比 BGRA 减少 25% 内存
3. 状态快照: 仅在变化时克隆
4. 模板预加载: 启动时全部加载
5. 历史限制: ActionHistory 最多 100 条
```

---

## 19. 错误处理与健壮性

### 19.1 错误分类

```rust
#[derive(Debug, thiserror::Error)]
pub enum TfError {
    #[error("Capture error: {0}")]
    Capture(String),

    #[error("Vision pipeline error: {0}")]
    Vision(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("State machine error: {0}")]
    StateMachine(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Calibration error: {0}")]
    Calibration(String),

    #[error("Window not found: {0}")]
    WindowNotFound(String),
}
```

### 19.2 健壮性策略

| 场景 | 处理策略 |
|------|----------|
| 窗口关闭/最小化 | 暂停 capture，等待窗口恢复 |
| 帧捕获失败 | 重试 3 次，间隔 100ms，失败则暂停该桌 |
| ONNX 推理失败 | 降级到模板匹配，标记 confidence 降低 |
| 状态机不一致 | 触发重新校准，重置为 Waiting 状态 |
| OCR 识别失败 | 忽略，不影响主路径 |
| 内存不足 | 减少帧缓冲，降低 FPS |
| CPU 过载 | 降低 FPS，减少推理频率 |

### 19.3 恢复机制

```rust
impl TableHandle {
    pub async fn run_with_recovery(mut self) {
        let mut consecutive_errors = 0u32;
        let max_errors = 10;

        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            match self.process_next_frame().await {
                Ok(_) => {
                    consecutive_errors = 0;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::warn!(
                        "Table {} error ({}/{}): {:?}",
                        self.table_id, consecutive_errors, max_errors, e
                    );

                    if consecutive_errors >= max_errors {
                        tracing::error!("Table {} entering recovery mode", self.table_id);
                        // Recovery：重新探测窗口位置 + 触发自动校准 + 重置状态机
                        let _ = self.rediscover_window().await;
                        let _ = self.trigger_recalibration().await;
                        self.state_machine.reset_to_waiting();
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        consecutive_errors = 0;
                    } else {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
            }
        }
    }
}
```

---

## 20. MVP 阶段拆分

### Phase 0: 项目基础设施（1 周）

```
目标: 搭建完整开发环境

- [ ] Rust workspace 初始化 (7 crates)
- [ ] Electron 项目初始化 (Vite + SolidJS + TailwindCSS)
- [ ] napi-rs 构建配置
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] 核心类型定义 (tf-core)
- [ ] 错误类型定义
- [ ] 日志系统 (tracing)
```

### Phase 1: 单桌视觉基础（3 周）

```
目标: 单桌帧捕获 + 基础卡牌识别

Week 1:
- [ ] Frame Capture (DXGI Desktop Duplication)
- [ ] Preprocessor (resize, color convert)
- [ ] ROI Manager (硬编码位置)

Week 2:
- [ ] Card Detection - Template Matching
- [ ] 52 张卡牌模板准备
- [ ] 基础测试 (准确率 > 95%)

Week 3:
- [ ] ONNX 卡牌分类模型 (fallback)
- [ ] Frame Diff (帧差分)
- [ ] Vision Pipeline 整合
```

### Phase 2: 状态机 + 动作推导（2 周）

```
目标: 完整的状态机 + 动作推导链路

Week 4:
- [ ] Stack Detection (pixel analysis)
- [ ] Pot Detection (region diff)
- [ ] Seat Status Detection
- [ ] Dealer Button Detection

Week 5:
- [ ] Action Reconstructor
- [ ] State Machine (完整状态转换)
- [ ] State Validator
- [ ] 集成测试 (feature → state → action)
```

### Phase 3: 推荐引擎集成（1 周）

```
目标: State → Recommendation 完整链路

Week 6:
- [ ] RecEngine wrapper
- [ ] TableState → RecInput 映射
- [ ] RecOutput 处理
- [ ] Recommendation Cache
- [ ] SDK bridge (Node.js embedded / Rust port)
```

### Phase 4: IPC + Overlay（2 周）

```
目标: Rust ↔ Electron 通信 + HUD 显示

Week 7:
- [ ] napi-rs bridge (commands + events)
- [ ] Electron Main Process (window, IPC)
- [ ] Overlay Window (transparent, click-through)

Week 8:
- [ ] SolidJS Overlay UI
- [ ] Recommendation Panel
- [ ] Action Distribution Chart
- [ ] Settings Panel
```

### Phase 5: 多桌 + 生产化（2 周）

```
目标: 多桌并发 + 稳定性

Week 9:
- [ ] TableManager (multi-table orchestration)
- [ ] Table Discovery (window enumeration)
- [ ] Auto-calibration
- [ ] 错误恢复机制

Week 10:
- [ ] 性能优化 (帧跳过, 缓存, 批量推理)
- [ ] OCR 辅助 (数字校准)
- [ ] 端到端集成测试
- [ ] 内存/CPU profiling
```

### Phase 6: 打磨 + 发布（1 周）

```
目标: 产品化 + 打包发布

Week 11:
- [ ] UI 打磨
- [ ] Dashboard (session stats)
- [ ] Hand History Replay
- [ ] Electron Builder 打包
- [ ] Installer (NSIS)
- [ ] 文档
```

**MVP 总工期: ~11 周**

---

## 21. 推荐开发顺序

```
开发依赖图:

tf-core (无依赖)
  │
  ├──→ tf-vision (依赖 tf-core, opencv, onnx)
  │       │
  │       └──→ tf-inference (依赖 tf-core)
  │
  ├──→ tf-state (依赖 tf-core, tf-vision)
  │       │
  │       └──→ tf-rec (依赖 tf-core, tf-state)
  │
  ├──→ tf-table (依赖 tf-core, tf-vision, tf-state, tf-rec)
  │
  └──→ tf-napi (依赖 tf-core, tf-table)
          │
          └──→ Electron app (依赖 tf-napi)

推荐实现顺序:

1. tf-core         → 所有类型定义
2. tf-vision       → 帧 capture + 卡牌识别
3. tf-state        → 状态机 + 动作推导
4. tf-inference    → ONNX 集成
5. tf-rec          → 推荐引擎桥接
6. tf-table        → 多桌管理
7. tf-napi         → IPC 桥接
8. Electron app    → UI + Overlay
```

---

## 21.A Hand History Replay（行动历史回放）

`prompt.md` 明确要求 Action History Replay。这是 v1.0 必备能力。

### 21.A.1 数据模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandRecord {
    pub hand_id: u64,
    pub table_id: TableId,
    pub started_at: i64,        // Unix ms
    pub ended_at: Option<i64>,
    pub blinds: BlindsInfo,
    pub dealer_seat: SeatId,
    pub hero_seat: Option<SeatId>,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub pot_total: f64,
    /// 完整 action 序列（含 PostBlind）
    pub actions: Vec<ActionRecord>,
    /// 每个 street 完成时的 TableState 快照
    pub street_snapshots: Vec<(Street, TableStateSnapshot)>,
    /// 每条 action 对应的推荐结果（若 hero 视角触发了推荐）
    pub recommendations: Vec<(u32 /*action_seq*/, RecOutput)>,
    pub result: Option<HandResult>,
}
```

### 21.A.2 持久化

| 介质 | 用途 |
|------|------|
| **SQLite (per user)** | hand 元数据 + action 序列（结构化查询） |
| **JSONL append-only** | 完整事件流（崩溃恢复 / 调试） |
| **可选：稀疏 frame snapshot (PNG)** | 只在用户触发"标记此手"时存储原始帧（隐私 + 体积考虑） |

```
~/AppData/Roaming/TableFlow/
├── hands.db                    # SQLite
├── events/
│   └── 2026-05-08.jsonl        # 当日事件流
└── frames/
    └── hand_<id>/              # 用户标记的帧
```

### 21.A.3 Replay UI

- Timeline scrubber（可拖动到任意 action）
- 每帧显示当时的 TableState + 推荐结果
- 支持"如果当时这样打"的 what-if 模拟（重新调用 RecEngine）

### 21.A.4 隐私

- 默认不记录其他玩家姓名 / 头像
- Hero 手牌可选加密
- 用户可一键清空全部 history

---

## 21.B 多主题 / 多客户端适配

`prompt.md` 要求"不同主题皮肤"。设计：

```rust
/// 一个 Calibration Profile = 客户端 × 主题 × 分辨率
pub struct CalibrationProfile {
    pub profile_id: String,            // e.g. "pokerstars-classic-1920x1080"
    pub client_signature: ClientSig,   // 用于自动识别客户端
    pub theme_id: String,
    pub calibration: TableCalibration,
    pub card_template_set: PathBuf,    // 该主题专属的卡牌模板目录
    pub button_template_set: PathBuf,
}

pub struct ClientSig {
    pub window_title_pattern: String,  // 正则
    pub window_class: Option<String>,
    pub felt_color_hint: (u8, u8, u8), // 桌面主色
}
```

启动时按窗口标题 + 桌面主色匹配 profile，匹配不到则提示用户手动校准并保存为新 profile。

模板目录结构：

```
resources/
├── profiles/
│   ├── pokerstars-classic.json
│   ├── pokerstars-modern.json
│   └── ggpoker-default.json
└── templates/
    ├── pokerstars-classic/
    │   ├── cards/
    │   └── buttons/
    └── pokerstars-modern/
        └── ...
```

---

## 21.C 反作弊兼容性约束

为了避免被扑克客户端 / 游戏 Anti-cheat 标记：

```
✓ 允许:
  - DXGI Desktop Duplication（公开 API，与 OBS 同款）
  - Windows Graphics Capture API
  - 透明 Overlay（与目标进程零交互）

✗ 禁止:
  - ReadProcessMemory / WriteProcessMemory
  - DLL injection / API hook
  - SendInput / PostMessage / SendMessage 模拟点击
  - 修改目标窗口属性
  - 任何形式的 process attach / debug API
```

发布时附带反作弊白名单说明文档，说明 TableFlow 仅做"屏幕读取 + UI 叠加"，不接触任何游戏进程内存。

---

## 21.D 测试与基准

```
tests/
├── rust/
│   ├── vision_fixtures/        # 录制的游戏视频 → 帧切片
│   │   ├── pokerstars_4max_30fps.mp4
│   │   └── ggpoker_6max_30fps.mp4
│   ├── vision_tests.rs         # 帧 → 期望 features
│   ├── state_machine_tests.rs  # property-based via proptest
│   └── pipeline_tests.rs       # 端到端 latency benchmark
└── fixtures/
    ├── frames/                 # 关键场景的静态帧
    └── states/                 # 期望状态快照（JSON）
```

基准目标：
- 单桌 latency p50 < 30ms, p99 < 80ms
- 4 桌并发 CPU < 10%
- 8 桌并发 CPU < 15%, 内存 < 500MB
- 状态识别准确率 > 98%（基于 fixture 集）

---

## 22. 未来扩展路线图

### v1.0 (MVP)

- 单桌/多桌视觉识别
- 状态推导 + 动作重建
- GTO 推荐
- Overlay HUD
- 手牌历史

### v1.1

- 多平台支持（不同扑克客户端）
- 自动主题/皮肤适配
- 高级统计（VPIP, PFR, AF, 3bet%）
- 对手建模 (Opponent Modeling)
- Range 估算可视化

### v1.2

- MTT 锦标赛支持
- ICM 计算
- Bubble factor 分析
- Final table 策略

### v2.0

- 神经网络端到端检测（YOLO-based table parsing）
- 自适应校准 (self-supervised calibration)
- 云端同步（手牌历史、统计数据）
- 对手数据库 (HUD stats across sessions)
- 视频 Replay + 标注

### v2.0+

- 多语言 UI
- 插件系统
- API 开放（第三方集成）
- 移动端远程监控

---

## 23. 风险分析

### 23.1 技术风险

| 风险 | 严重性 | 可能性 | 缓解策略 |
|------|--------|--------|----------|
| DXGI 捕获在某些 GPU 上不兼容 | 高 | 中 | 备选方案: Windows Graphics Capture API, GDI BitBlt |
| 不同扑克客户端 UI 差异大 | 高 | 高 | Calibration 系统 + 多套 ROI 模板 |
| ONNX 模型精度不足 | 中 | 中 | Template matching 作为 fallback，模型 fine-tune |
| 状态推导误差累积 | 高 | 中 | 定期 OCR 校准 + State Validator + 置信度衰减 |
| 多桌 CPU 过载 | 中 | 中 | 动态 FPS 调节 + Rayon 负载均衡 |
| Overlay 窗口在某些游戏上闪烁 | 中 | 低 | DWM composition + 双缓冲渲染 |
| Electron 内存泄漏 | 中 | 中 | 定期重启 Renderer Process + 内存监控 |

### 23.2 工程风险

| 风险 | 严重性 | 可能性 | 缓解策略 |
|------|--------|--------|----------|
| napi-rs 跨语言调试困难 | 中 | 高 | 充分的 Rust 单元测试 + 集成测试 |
| OpenCV Rust 绑定 API 不稳定 | 中 | 中 | 固定 opencv-rust 版本，封装抽象层 |
| 推荐引擎 SDK 集成复杂度 | 中 | 中 | MVP 先用 Node.js sidecar，后续 port 到 Rust |
| Windows 兼容性（Win10/11 版本差异） | 中 | 中 | CI 多版本测试 + feature detection |

### 23.3 产品风险

| 风险 | 严重性 | 可能性 | 缓解策略 |
|------|--------|--------|----------|
| 扑克客户端反作弊检测 | 高 | 高 | 纯读取模式（不注入、不模拟输入），仅 Overlay |
| 用户法律/合规顾虑 | 高 | 低 | 免责声明，定位为学习/分析工具 |
| 竞品成熟度高 | 中 | 高 | 差异化: 状态推导 vs OCR-first 架构优势 |
