import { createStore, produce } from "solid-js/store";
import { createSignal } from "solid-js";

export interface TableState {
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

export interface TableEntry {
  id: string;
  state: TableState | null;
}

const [tables, setTables] = createStore<TableEntry[]>([]);
const [activeTableId, setActiveTableId] = createSignal<string | null>(null);

export { tables, setTables, activeTableId, setActiveTableId };

export function updateTableState(tableId: string, state: TableState) {
  setTables(
    produce((list) => {
      const idx = list.findIndex((t) => t.id === tableId);
      if (idx >= 0) {
        list[idx].state = state;
      } else {
        list.push({ id: tableId, state });
      }
    }),
  );
}

export function removeTable(tableId: string) {
  setTables(
    produce((list) => {
      const idx = list.findIndex((t) => t.id === tableId);
      if (idx >= 0) list.splice(idx, 1);
    }),
  );
}
