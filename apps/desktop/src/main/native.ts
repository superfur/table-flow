import * as path from "node:path";

export interface TableConfig {
  tableId: string;
  windowTitle: string;
}

export interface JsTableState {
  tableId: string;
  phase: string;
  street: string;
  handNumber: number;
  dealerSeat: number | null;
  heroSeat: number | null;
  holeCards: { suit: string; rank: string }[] | null;
  communityCards: { suit: string; rank: string }[];
  pot: number;
  seats: {
    seatId: number;
    status: string;
    stack: number;
    currentBet: number;
    lastAction: string | null;
    isHero: boolean;
    hasCards: boolean;
  }[];
  stateConfidence: number;
}

export interface JsRecOutput {
  action: string;
  amount: number;
  confidence: number;
  distribution: Record<string, number>;
  ev: number;
}

export interface NativeAddon {
  startCapture(config: TableConfig): Promise<void>;
  stopCapture(tableId: string): Promise<void>;
  discoverTables(): Promise<string[]>;
  getTableState(tableId: string): Promise<JsTableState>;
  calibrateTable(tableId: string): Promise<unknown>;
  shutdown(): Promise<void>;
  onStateUpdate(
    cb: (event: {
      tableId: string;
      state: JsTableState;
      timestampMs: number;
    }) => void,
  ): () => void;
  onRecommendation(
    cb: (event: {
      tableId: string;
      recommendation: JsRecOutput;
      timestampMs: number;
    }) => void,
  ): () => void;
  onError(
    cb: (event: { tableId: string | null; message: string }) => void,
  ): () => void;
}

type StateListener = (event: {
  tableId: string;
  state: JsTableState;
  timestampMs: number;
}) => void;
type RecListener = (event: {
  tableId: string;
  recommendation: JsRecOutput;
  timestampMs: number;
}) => void;
type ErrorListener = (event: {
  tableId: string | null;
  message: string;
}) => void;

export async function loadNative(): Promise<NativeAddon> {
  try {
    const modulePath = path.join(
      process.resourcesPath ?? path.join(__dirname, ".."),
      "tf_napi.node",
    );
    const native = await import(/* @vite-ignore */ modulePath);
    return native as NativeAddon;
  } catch {
    return createMockNative();
  }
}

function createMockNative(): NativeAddon {
  const stateListeners: Array<StateListener> = [];
  const recListeners: Array<RecListener> = [];
  const errorListeners: Array<ErrorListener> = [];

  return {
    async startCapture() {},
    async stopCapture() {},
    async discoverTables() {
      return ["mock-table-1"];
    },
    async getTableState(tableId: string) {
      return {
        tableId,
        phase: "playing",
        street: "preflop",
        handNumber: 1,
        dealerSeat: 0,
        heroSeat: 2,
        holeCards: [
          { suit: "s", rank: "A" },
          { suit: "h", rank: "K" },
        ],
        communityCards: [],
        pot: 100,
        seats: [
          {
            seatId: 0,
            status: "active",
            stack: 500,
            currentBet: 0,
            lastAction: null,
            isHero: false,
            hasCards: true,
          },
          {
            seatId: 1,
            status: "active",
            stack: 450,
            currentBet: 0,
            lastAction: null,
            isHero: false,
            hasCards: true,
          },
          {
            seatId: 2,
            status: "active",
            stack: 400,
            currentBet: 0,
            lastAction: null,
            isHero: true,
            hasCards: true,
          },
        ],
        stateConfidence: 0.95,
      } as JsTableState;
    },
    async calibrateTable() {
      return { status: "not_available" };
    },
    async shutdown() {},

    onStateUpdate(cb: StateListener) {
      stateListeners.push(cb);
      return () => {
        const idx = stateListeners.indexOf(cb);
        if (idx >= 0) stateListeners.splice(idx, 1);
      };
    },
    onRecommendation(cb: RecListener) {
      recListeners.push(cb);
      return () => {
        const idx = recListeners.indexOf(cb);
        if (idx >= 0) recListeners.splice(idx, 1);
      };
    },
    onError(cb: ErrorListener) {
      errorListeners.push(cb);
      return () => {
        const idx = errorListeners.indexOf(cb);
        if (idx >= 0) errorListeners.splice(idx, 1);
      };
    },
  };
}
