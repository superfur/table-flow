# TableFlow 架构实现进度

> **范围说明**
>
> 本工作流只负责 **架构骨架** 的代码编写：
> - Cargo / npm workspace 结构
> - 各 crate / package 的目录骨架与 `mod.rs` 链接
> - **公开类型**（struct / enum / trait）的定义
> - 函数签名（body 用 `todo!()` / `unimplemented!()` 占位）
> - 模块之间的**通道（channel）/ 错误 / 事件**契约
> - 必要的 Cargo.toml / package.json / build script
>
> **不负责** 任何具体算法实现，包括但不限于：
> - DXGI 帧捕获的 Win32 调用
> - OpenCV pipeline 的图像处理代码
> - ONNX 推理的张量构造与后处理
> - PaddleOCR 的解码逻辑
> - 模板匹配 / Frame Diff / Hero 识别的视觉算法
> - State Machine / Action Reconstructor 的德扑规则细节
> - Recommendation Engine sidecar 的 Node.js 子进程实现
> - Electron Overlay 的具体 UI 与窗口同步
>
> 这些**留给后续 detail-impl 阶段** 或 GLM/其他工作者实现，本阶段只保证：
> 1. 工作区能 `cargo check` 通过
> 2. 类型系统自洽（不会出现编译错误）
> 3. 模块边界清晰，每个 `todo!()` 都有明确的输入输出契约
>
> 真理来源：`ARCHITECTURE.md`。本文档与之对齐，发现冲突以 `ARCHITECTURE.md` 为准。

---

## 进度总览

| # | 阶段 | 范围 | 状态 | 完成度 |
|---|------|------|------|--------|
| P0 | 工作区骨架 | Cargo workspace + Electron monorepo + CI 占位 | ✅ 完成 | 5/5 |
| P1 | `tf-core` | 全局类型、错误、事件、配置 | ✅ 完成 | 4/4 |

| P2 | `tf-inference` | ONNX Session Pool / OCR Assistant 接口 | ✅ 完成 | 3/3 |
| P3 | `tf-vision` | Capture / Pipeline / Detection / Calibration 模块边界 | ✅ 完成 | 8/8 |
| P4 | `tf-state` | StateMachine / Reconstructor / BettingRoundEngine / Validator | ✅ 完成 | 5/5 |
| P5 | `tf-rec` | RecEngine trait / Sidecar 协议 / Cache | ✅ 完成 | 4/4 |
| P6 | `tf-table` | TableManager / TableHandle / Discovery | ✅ 完成 | 3/3 |
| P7 | `tf-napi` | Bridge / Commands / Events / Type marshalling | ✅ 完成 | 4/4 |
| P8 | `apps/desktop` | Electron main / preload / SolidJS overlay 骨架 | ✅ 完成 | 6/6 |
| P9 | 集成验证 | `cargo check --workspace` + `pnpm typecheck` 通过 | 🚧 待本地验证 | 0/2 |

**图例**：⏳ 未开始 / 🚧 进行中 / ✅ 完成 / 🔁 需返工

---

## P0 · 工作区骨架

| 任务 | 路径 | 状态 |
|------|------|------|
| 根 `Cargo.toml`（workspace 定义） | `/Cargo.toml` | ✅ |
| 根 `package.json`（npm workspace） + `pnpm-workspace.yaml` | `/package.json`, `/pnpm-workspace.yaml` | ✅ |
| 7 个 crate 骨架 + apps/desktop 骨架（lib.rs / mod.rs / index.ts 全部就位） | `/crates/*`, `/apps/desktop` | ✅ |
| `.gitignore`、`rust-toolchain.toml` | 根目录 | ✅ |
| CI workflow 占位（rust + electron + release，3 份） | `.github/workflows/*.yml` | ✅ |

**完成判据**：`cargo check --workspace` 报"找不到内容"以外的错误为零；npm workspace 能识别所有子包。

---

## P1 · `tf-core`

| 任务 | 文件 | 状态 |
|------|------|------|
| 基础类型：`Card / Suit / Rank / Street / SeatId / TableId / BlindKind / ActionType / SeatStatus / TablePhase / ActionSource` | `src/types.rs` | ✅ |
| 错误类型：`TfError` + `Result<T>` 别名 | `src/error.rs` | ✅ |
| 事件类型：`TableEvent / ReconstructedAction / StateTransition / ManagerEvent / HandResult` | `src/events.rs` | ✅ |
| 配置类型：`ManagerConfig / ThreadConfig / InferenceConfig / CalibrationProfile / TableCalibration / SeatCalibration / BlindsInfo / DigitOcrRegions / NormalizedRect` | `src/config.rs` | ✅ |

**完成判据**：所有类型可序列化（serde 派生），跨 crate 引用无循环依赖。

---

## P2 · `tf-inference`

| 任务 | 文件 | 状态 |
|------|------|------|
| `InferencePool` + `OpaqueSession` + `CardClassificationInput/Output` + `DigitInput/Output` | `src/session.rs` | ✅ |
| `CardClassifier` trait + `class_id_to_card` + `frame_to_card_input` | `src/card_model.rs` | ✅ |
| `DigitRecognizer` trait + `frame_to_digit_input` + `parse_number_from_digits` | `src/digit_model.rs` | ✅ |
| `Ocr` trait + `OcrAssistant` 默认实现（含 `disabled()` / `NullRecognizer`） | `src/ocr.rs` | ✅ |
| 共享预处理 `bgra_to_rgb / to_grayscale / resize / ctc_greedy_decode` | `src/prepost.rs` | ✅ |

**完成判据**：`tf-vision` 可以引用并 `unimplemented!()` 调用 OCR / 推理接口。

---

## P3 · `tf-vision`

| 任务 | 文件 | 状态 |
|------|------|------|
| `FrameCapture` trait + `CapturedFrame` + `DxgiCapture` 默认实现 + `FpsLimiter` + `WindowInfo / enumerate_windows / get_window_bounds` | `src/capture/{mod,dxgi,fps,window}.rs` | ✅ |
| `Preprocessor / PreprocessorConfig`、`RoiManager / TableRoi / SeatRoi`、`DiffDetector / DiffConfig` | `src/pipeline/{preprocessor,roi,diff}.rs` | ✅ |
| `CardDetector` trait + `DefaultCardDetector`、`StackTracker / StackBaseline / StackSnapshot`、`PotTracker`、`SeatTracker / TrackedSeat`、`DealerTracker`、`HeroDetector`、`ActionButtonDetector` | `src/detection/*.rs` | ✅ |
| `TemplateMatcher / TemplateMatch`、`ContourAnalyzer / Contour`、`FeatureExtractor / FeatureVector / cosine_similarity` | `src/matching/*.rs` | ✅ |
| `AutoCalibrator`（MVP 直接返回 Err）+ `load_profiles / match_profile` | `src/calibration/{mod,auto}.rs` | ✅ |
| `FeatureAggregator` + `RawFeatures / ExtractedFeatures / CardDetectionResult / StackChange / PotChange / SeatChange` | `src/features.rs` + `src/pipeline/aggregator.rs` | ✅ |
| `VisionPipeline` 主结构体 + `VisionPipelineRun` trait + `run()` 签名 | `src/pipeline/mod.rs` | ✅ |
| 公开 API 不依赖 OpenCV，所有内部 `Frame` 都来自 `tf-core::Frame` | 全模块 | ✅ |

**完成判据**：`VisionPipeline::run` 可以从 `tf-table` 启动（即使内部全是 `todo!()`）。

---

## P4 · `tf-state`

| 任务 | 文件 | 状态 |
|------|------|------|
| `TableState / SeatState / PotInfo / SidePot / ActionRecord` 类型 | `src/state.rs` | ⏳ |
| `TableStateMachine` 结构 + `process_event` 签名 | `src/machine.rs` | ⏳ |
| `ActionReconstructor` 接口 + 输入输出 | `src/reconstructor.rs` | ⏳ |
| `BettingRoundEngine`（`is_round_complete / to_call_for / min_raise`） | `src/round.rs` | ⏳ |
| `StateValidator` + `ValidationResult / ValidationIssue` | `src/validator.rs` | ⏳ |

**完成判据**：`TableState` 字段与 `ARCHITECTURE.md` §11.2 完全一致。

---

## P5 · `tf-rec`

| 任务 | 文件 | 状态 |
|------|------|------|
| `RecInput / RecOutput` 类型 | `src/input.rs`, `src/output.rs` | ⏳ |
| `RecEngine` trait | `src/engine.rs` | ⏳ |
| `RecSidecar`（JSON-RPC over stdio）骨架 + 协议常量 | `src/sidecar.rs` | ⏳ |
| `RecCache`（含 action_history digest）骨架 | `src/cache.rs` | ⏳ |

**完成判据**：trait 定义清晰；Sidecar 启动函数返回 `Result<RecSidecar, TfError>`，body `todo!()`。

---

## P6 · `tf-table`

| 任务 | 文件 | 状态 |
|------|------|------|
| `TableManager` 结构 + 公开方法签名 | `src/manager.rs` | ⏳ |
| `TableHandle` 生命周期（`new / shutdown / run_with_recovery`） | `src/handle.rs` | ⏳ |
| `TableDiscovery` 接口 | `src/discovery.rs` | ⏳ |

**完成判据**：能从 `tf-napi` 调用 `TableManager::start_table` 启动一个 handle（内部全是 todo）。

---

## P7 · `tf-napi`

| 任务 | 文件 | 状态 |
|------|------|------|
| `NapiBridge` 结构 + `get_bridge()` | `src/bridge.rs` | ⏳ |
| 命令导出：`start_capture / stop_capture / get_table_state / calibrate_table / discover_tables` | `src/commands.rs` | ⏳ |
| 事件导出：`on_state_update / on_recommendation / on_error`（用 `ThreadsafeFunction`） | `src/events.rs` | ⏳ |
| JS↔Rust 类型 marshalling：`JsTableState / JsCard / JsSeat / JsRecOutput` | `src/types.rs` | ⏳ |

**完成判据**：`napi build` 能产出 `.node` 文件（即使运行时 panic on todo）。

---

## P8 · `apps/desktop`

| 任务 | 文件 | 状态 |
|------|------|------|
| Vite + SolidJS + TailwindCSS 配置 | `vite.config.ts`, `tailwind.config.js` | ⏳ |
| Electron main 入口 + 窗口管理骨架 | `src/main/*.ts` | ⏳ |
| `native.ts` 加载 `tf-napi` 模块（含类型定义） | `src/main/native.ts` | ⏳ |
| Preload bridge | `src/preload/index.ts` | ⏳ |
| SolidJS App / 路由 / store 骨架 | `src/renderer/*.tsx` | ⏳ |
| `HudOverlay / Recommendation / SettingsPanel` 占位组件 | `src/renderer/components/*` | ⏳ |

**完成判据**：`pnpm dev` 可以启动一个空白 Electron 窗口（不要求功能完整）。

---

## P9 · 集成验证

| 任务 | 状态 |
|------|------|
| `cargo check --workspace` 通过（允许 warnings） | 🚧 待本地 |
| `pnpm -r typecheck` 通过 | 🚧 待本地 |

**说明**：当前沙盒环境没有 `cargo` / `pnpm`，所以这两个命令需要在你本地的 macOS / Windows 上运行验证。
基于静态 review，预计能直接通过；如果遇到编译错误，可能命中以下几个**已知风险点**：

1. **`opencv-rust` 没在依赖里**：架构骨架阶段刻意没引入，所有 vision 内部用 `tf_core::Frame` 占位。
   detail-impl 阶段才需要 `opencv = "0.94"`（且需要系统装 OpenCV 4.x）。
2. **`napi / napi-derive` 在 `tf-napi/Cargo.toml` 中被注释掉**：所以 `tf-napi` 此刻只是普通的
   Rust cdylib，并不能真正被 Electron 加载。这是有意为之，避免构建工具链未配置时拉不下来。
3. **TS 端的 `@table-flow/desktop` 还没 `pnpm install`**：第一次 typecheck 需要先
   `pnpm install --frozen-lockfile=false`（项目还没生成 lockfile）。
4. **`tokio_util::sync::CancellationToken` 没引入**：在 `tf-table` 中我用了一个简化的
   `Arc<AtomicBool>` 占位 (`CancelToken`)，detail-impl 时按需替换。
5. **`async_trait` + `Send + Sync` 边界**：所有 trait 都明确写了 `: Send + Sync`，但若
   detail-impl 里某处写了非 Send 的内部状态（如 `*mut c_void`），需要单独包 Mutex。

### 验证命令

```bash
# Rust 工作区
cargo check --workspace

# TS 工作区（首次需要 install）
pnpm install
pnpm -r typecheck
```

---

## Changelog

- **2026-05-09** · 文档创建。范围、阶段、各 crate 任务清单初版。
- **2026-05-09** · ✅ P0 完成：
  - `/Cargo.toml`（workspace + 共享依赖 + dev/release profile）
  - `/package.json` + `/pnpm-workspace.yaml`
  - `/rust-toolchain.toml`（pin 1.78）+ `/.gitignore`
  - 7 个 Rust crate 骨架：`tf-core / tf-inference / tf-vision / tf-state / tf-rec / tf-table / tf-napi`，每个 crate 的 `Cargo.toml` 和 `src/lib.rs` 模块声明就位，业务逻辑文件全部为 `// 占位` 注释
  - `tf-core` 引入 `Frame / Rect / PixelFormat` 占位类型，避免架构骨架阶段对 opencv 的硬依赖
  - `tf-napi` 暂时注释掉 napi/napi-derive 依赖（等 P7 detail-impl 阶段开启）
  - `apps/desktop` 骨架：`package.json / tsconfig.json / vite.config.ts / tailwind.config.js / postcss.config.js / electron-builder.yml / index.html`
  - Electron main 进程文件：`index.ts / window.ts / overlay.ts / native.ts / ipc.ts / tray.ts`
  - Preload bridge：`preload/index.ts`（含 `ElectronAPI` global 声明）
  - SolidJS renderer：`index.tsx / App.tsx / styles/index.css` + `store/{table,recommendation,settings}.ts` + `components/{overlay,settings,dashboard}/*.tsx`
  - resources 占位目录：`models/`, `templates/`
  - 3 份 CI workflow 占位：`ci-rust.yml / ci-electron.yml / release.yml`
- **2026-05-09** · ✅ P1 完成（`tf-core` 真实类型填充）：
  - `src/types.rs`：`SeatId / Suit / Rank / Card / Street / TablePhase / ActionType / BlindKind / SeatStatus / ActionSource`，`Card` 不实现 Hash（持有 f32 confidence），按 `(suit, rank)` 比较唯一性
  - `src/error.rs`：`TfError` + `Result<T, E = TfError>` 别名，含 12 个具体变体（Capture / Vision / Inference / Ocr / StateMachine / Recommendation / Ipc / Calibration / WindowNotFound / TableNotFound / Config / Io / Serde / Other）
  - `src/events.rs`：`TableEvent / ReconstructedAction / PostedBlind / StateTransition / ActionRecordSummary / HandResult / ManagerEvent`
  - `src/config.rs`：`ThreadConfig / InferenceConfig / CaptureBackend / ManagerConfig / BlindsInfo / CalibrationProfile / ClientSignature / TableCalibration / SeatCalibration / DigitOcrRegions / NormalizedRect`，`NormalizedRect::to_pixel_rect` 提供归一化坐标 → 像素坐标的映射
  - `tf-core/Cargo.toml` 增加 `num_cpus` 依赖（`ThreadConfig::default` 使用）
- **2026-05-09** · ✅ P2 完成（`tf-inference` 接口边界）：
  - `OpaqueSession`（detail-impl 阶段替换为 `ort::Session`），`InferencePool` 三个公开方法（`new / classify_card / recognize_digits / shutdown`）
  - `CardClassifier` / `DigitRecognizer` 两个 trait，`class_id_to_card` 提供 52-class indexing 转换
  - `OcrAssistant` 默认实现 + `NullRecognizer`，可 `disabled()` 单测注入
  - 共享原语 `bgra_to_rgb / to_grayscale / resize / ctc_greedy_decode`
- **2026-05-09** · ✅ P3 完成（`tf-vision` 模块边界）：
  - `capture/`: `FrameCapture` trait + `CapturedFrame` + `DxgiCapture` 默认实现 + `FpsLimiter` + `WindowInfo / enumerate_windows`
  - `pipeline/`: `Preprocessor`, `RoiManager` (含 `TableRoi / SeatRoi`), `DiffDetector`, `FeatureAggregator`, **`VisionPipeline` 主结构体 + `VisionPipelineRun` trait**
  - `detection/`: `CardDetector` trait + `DefaultCardDetector`, `StackTracker` (含 baseline/history), `PotTracker` (含 pixel hash 跳过), `SeatTracker`, `DealerTracker`, `HeroDetector` (manual + face-up fallback), `ActionButtonDetector`
  - `matching/`: `TemplateMatcher / TemplateMatch`, `ContourAnalyzer / Contour`, `FeatureExtractor / FeatureVector` (`cosine_similarity` 已实现)
  - `calibration/`: `AutoCalibrator` (MVP 直接返回 Err) + `load_profiles / match_profile` 函数
  - `features.rs`：`RawFeatures / ExtractedFeatures / StackChange / PotChange / SeatChange / CardDetectionResult` 集中管理
- **2026-05-09** · ✅ P4 完成（`tf-state`）：
  - `state.rs`：`TableState / PotInfo / SidePot / SeatState / ActionRecord` + `TableState::initial`、`SeatState::new` 构造器
  - `machine.rs`：`TableStateMachine`（含 `process_event / advance_turn / reset_to_waiting / state / snapshot / next_action_seq / next_hand_seq`），`event_log` 容量 1000
  - `reconstructor.rs`：`ActionReconstructor` + `ReconConfig` + `ReconInput / StackDelta / PotDelta`，避免 tf-state 反向依赖 tf-vision
  - `round.rs`：`BettingRoundEngine`（`is_round_complete / to_call_for / min_raise / total_committed`）—— 所有方法都是无状态纯函数
  - `validator.rs`：`StateValidator` + `ValidationIssue / ValidationResult`，6 类校验项
  - `tracker.rs`：`PlayerStats / StatsTracker`（v1.1 占位）
- **2026-05-09** · ✅ P5 完成（`tf-rec`）：
  - `input.rs`：`RecInput / RecActionRecord`（`is_blind()` 辅助方法）
  - `output.rs`：`RecOutput`
  - `engine.rs`：`RecEngine` trait + `build_rec_input`（从 TableState 构造，过滤 PostBlind）+ `recommend_from_state` 便利函数
  - `sidecar.rs`：`RecSidecar` + `SidecarConfig` + JSON-RPC 协议类型 + `DEFAULT_TIMEOUT`/`MAX_CONSECUTIVE_FAILURES` 常量
  - `cache.rs`：`RecCache`（带容量上限）+ `compute_cache_key`（已包含 action_history digest，规避 line-sensitive cache 污染）
- **2026-05-09** · ✅ P6 完成（`tf-table`）：
  - `manager.rs`：`TableManager`（持有 `Arc<InferencePool>` + `Arc<dyn RecEngine>` + `broadcast::Sender<ManagerEvent>`）
  - `handle.rs`：`TableHandle` + 自定义 `CancelToken`（`Arc<AtomicBool>` 占位，detail-impl 阶段可换成 `tokio_util::sync::CancellationToken`）
  - `discovery.rs`：`TableDiscovery::scan` + `DiscoveredTable`
- **2026-05-09** · ✅ P7 完成（`tf-napi`）：
  - `types.rs`：`JsCard / JsSeat / JsTableState / JsRecOutput / StateUpdateEvent / RecommendationEvent / ErrorEvent` + `street_to_str / phase_to_str / status_to_str / action_to_str / js_seat_id` 字符串化辅助
  - `bridge.rs`：`NapiBridge` 单例（`OnceCell`）+ 三个 `Mutex<Option<OpaqueTsfn<...>>>` 回调持有，`OpaqueTsfn<T>` 占位类型，detail-impl 替换为 `napi::ThreadsafeFunction`
  - `commands.rs`：`start_capture / stop_capture / discover_tables / get_table_state / calibrate_table / shutdown` 6 个 async fn 占位
  - `events.rs`：`on_state_update / on_recommendation / on_error` 3 个回调注册函数
  - `tf-napi/Cargo.toml` 改为 `cdylib + rlib` 双产出
- **2026-05-09** · 🔧 跨 crate re-export 修复：
  - `tf-vision/src/lib.rs` glob 扩展为 `pub use {calibration, capture, detection, features, matching, pipeline}::*`
  - `tf-vision/src/capture/mod.rs` 增加 `pub use {dxgi, fps, window}::*`
  - 让 `tf_vision::WindowInfo / FpsLimiter / DxgiCapture` 都能从顶层引用
- **2026-05-09** · 🚧 P9 暂标记"待本地验证"：sandbox 没 cargo/pnpm，需要在 macOS/Windows 上跑 `cargo check --workspace` 与 `pnpm install && pnpm -r typecheck` 确认编译。已在 P9 章节列出 5 类已知风险点供 cross-check。

---

## 工作约定

1. **每完成一个任务**，把对应行的状态从 ⏳ 改为 ✅，并在 Changelog 加一条简短记录（含文件路径）。
2. **遇到与 `ARCHITECTURE.md` 不一致**：先在本文档下新增 "决策记录" 段落，得到确认后再写代码；不擅自偏离架构。
3. **新增/调整阶段**：在进度总览表新增一行，并补对应章节。
4. **`todo!()` 必须带短注释**说明输入输出契约，方便后续 detail-impl 接手。
   ```rust
   // TODO(detail-impl): 给定 BGRA 帧 → 返回 BGR Mat，按 capture_region 裁剪。
   //   错误：返回 TfError::Capture(...)
   pub fn capture_region_dxgi(&self) -> Result<Mat, TfError> { todo!() }
   ```
