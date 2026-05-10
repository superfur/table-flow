import type { Component } from "solid-js";
import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { Dashboard } from "./components/dashboard/Dashboard";
import { HudOverlay } from "./components/overlay/HudOverlay";
import { SettingsPanel } from "./components/settings/SettingsPanel";
import { activeTableId, setActiveTableId, updateTableState } from "./store/table";
import { updateRecommendation } from "./store/recommendation";

type View = "dashboard" | "settings";

const App: Component = () => {
  const [view, setView] = createSignal<View>("dashboard");

  onMount(() => {
    const api = window.electronAPI;
    if (!api) return;

    const unsubState = api.onStateUpdate((event: any) => {
      if (event?.tableId && event?.state) {
        updateTableState(event.tableId, event.state);
      }
    });

    const unsubRec = api.onRecommendationUpdate((event: any) => {
      if (event?.tableId && event?.recommendation) {
        updateRecommendation(
          event.tableId,
          event.recommendation,
          event.timestampMs ?? Date.now(),
        );
      }
    });

    const unsubError = api.onError((event: any) => {
      console.error("[TableFlow]", event?.message ?? event);
    });

    onCleanup(() => {
      unsubState();
      unsubRec();
      unsubError();
    });
  });

  return (
    <div class="min-h-screen bg-neutral-900 text-neutral-100 flex flex-col">
      <nav class="flex items-center justify-between px-5 h-12 border-b border-neutral-800 bg-neutral-900/95 backdrop-blur-sm">
        <div class="flex items-center gap-4">
          <span class="text-sm font-bold tracking-wider uppercase text-neutral-300">
            TableFlow
          </span>
          <div class="flex gap-1">
            <button
              class={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                view() === "dashboard"
                  ? "bg-neutral-800 text-neutral-200"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
              onClick={() => setView("dashboard")}
            >
              Dashboard
            </button>
            <button
              class={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                view() === "settings"
                  ? "bg-neutral-800 text-neutral-200"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
              onClick={() => setView("settings")}
            >
              Settings
            </button>
          </div>
        </div>
        <Show when={activeTableId()}>
          <button
            class="text-xs text-neutral-500 hover:text-neutral-300 transition-colors"
            onClick={() => setActiveTableId(null)}
          >
            Back to tables
          </button>
        </Show>
      </nav>

      <main class="flex-1 overflow-auto">
        <Show when={activeTableId()}>
          <HudOverlay tableId={activeTableId()!} />
        </Show>

        <Show when={!activeTableId() && view() === "dashboard"}>
          <Dashboard />
        </Show>

        <Show when={!activeTableId() && view() === "settings"}>
          <SettingsPanel />
        </Show>
      </main>
    </div>
  );
};

export default App;
