import type { Component } from "solid-js";
import { Show } from "solid-js";
import { Recommendation } from "./Recommendation";
import { tables } from "../../store/table";

export interface HudOverlayProps {
  tableId: string;
}

export const HudOverlay: Component<HudOverlayProps> = (props) => {
  const table = () => tables.find((t) => t.id === props.tableId);

  return (
    <div class="absolute inset-0 pointer-events-none">
      <div class="pointer-events-auto absolute top-4 right-4 w-56">
        <div
          class="rounded-xl border border-neutral-800 bg-neutral-900/90 backdrop-blur-md
            shadow-2xl shadow-black/50 p-3.5 space-y-3"
        >
          <div class="flex items-center justify-between">
            <span class="text-[11px] font-medium uppercase tracking-wider text-neutral-500">
              TableFlow
            </span>
            <Show when={table()?.state}>
              <span class="text-[10px] font-mono text-neutral-600">
                {table()!.state!.street}
              </span>
            </Show>
          </div>

          <Recommendation tableId={props.tableId} />

          <Show when={table()?.state}>
            <div class="border-t border-neutral-800 pt-2 space-y-1">
              <div class="flex justify-between text-[11px]">
                <span class="text-neutral-500">Pot</span>
                <span class="text-amber-400 font-mono">
                  {table()!.state!.pot.toFixed(0)}
                </span>
              </div>
              <Show when={table()!.state!.holeCards}>
                <div class="flex justify-between text-[11px]">
                  <span class="text-neutral-500">Hand</span>
                  <span class="text-neutral-300 font-mono">
                    {table()!.state!.holeCards!.map((c) => `${c.rank}${c.suit}`).join(" ")}
                  </span>
                </div>
              </Show>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
};
