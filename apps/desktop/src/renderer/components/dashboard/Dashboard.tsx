import type { Component } from "solid-js";
import { For, Show } from "solid-js";
import { tables, activeTableId, setActiveTableId } from "../../store/table";
import { SessionStats } from "./SessionStats";

export const Dashboard: Component = () => {
  return (
    <div class="p-6 space-y-6">
      <div class="space-y-3">
        <h3 class="text-sm font-medium uppercase tracking-wider text-neutral-500">
          Session Stats
        </h3>
        <SessionStats />
      </div>

      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold tracking-tight text-neutral-100">
          Active Tables
        </h2>
        <button
          class="px-3 py-1.5 text-xs font-medium rounded-md
            bg-indigo-600 hover:bg-indigo-500 text-white
            transition-colors duration-150"
          onClick={() => window.electronAPI?.discoverTables()}
        >
          Scan Tables
        </button>
      </div>

      <Show
        when={tables.length > 0}
        fallback={
          <div class="text-sm text-neutral-500 py-12 text-center">
            No tables detected. Click "Scan Tables" to discover poker clients.
          </div>
        }
      >
        <div class="grid grid-cols-2 gap-3">
          <For each={tables}>
            {(table) => (
              <button
                class={`p-4 rounded-lg border text-left transition-all duration-150 ${
                  activeTableId() === table.id
                    ? "border-indigo-500 bg-indigo-500/10 shadow-lg shadow-indigo-500/20"
                    : "border-neutral-700 bg-neutral-800/50 hover:border-neutral-600"
                }`}
                onClick={() => setActiveTableId(table.id)}
              >
                <div class="flex items-center justify-between mb-2">
                  <span class="text-sm font-medium text-neutral-200 truncate">
                    {table.id}
                  </span>
                  <Show when={table.state}>
                    <span
                      class={`inline-block w-2 h-2 rounded-full ${
                        table.state!.phase === "playing"
                          ? "bg-emerald-400"
                          : "bg-neutral-500"
                      }`}
                    />
                  </Show>
                </div>
                <Show when={table.state}>
                  <div class="text-xs text-neutral-400 space-y-0.5">
                    <div>
                      Street:{" "}
                      <span class="text-neutral-300 capitalize">
                        {table.state!.street}
                      </span>
                    </div>
                    <div>
                      Pot:{" "}
                      <span class="text-amber-400 font-mono">
                        {table.state!.pot.toFixed(0)}
                      </span>
                    </div>
                  </div>
                </Show>
              </button>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};
