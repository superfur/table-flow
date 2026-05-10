import { recommendations } from "../../store/recommendation";
import { For, Show } from "solid-js";

export interface RecommendationProps {
  tableId: string;
}

export const Recommendation: Component<RecommendationProps> = (props) => {
  const rec = () => recommendations[props.tableId];

  return (
    <Show
      when={rec()}
      fallback={
        <div class="text-xs text-neutral-600 italic">
          Waiting for recommendation...
        </div>
      }
    >
      {(data) => {
        const r = () => data().recommendation;
        const actionColor = () => {
          const a = r().action;
          if (a === "fold") return "text-red-400";
          if (a === "call" || a === "check") return "text-sky-400";
          if (a === "raise") return "text-amber-400";
          if (a === "allIn") return "text-fuchsia-400";
          return "text-neutral-300";
        };

        return (
          <div class="space-y-2">
            <div class="flex items-baseline gap-2">
              <span class={`text-xl font-bold tracking-wide uppercase ${actionColor()}`}>
                {r().action.replace(":", " ")}
              </span>
              <Show when={r().amount > 0}>
                <span class="text-sm font-mono text-neutral-400">
                  {r().amount.toFixed(0)} BB
                </span>
              </Show>
            </div>

            <div class="flex items-center gap-1.5">
              <div class="flex-1 h-1 bg-neutral-700 rounded-full overflow-hidden">
                <div
                  class="h-full bg-indigo-500 rounded-full transition-all duration-300"
                  style={{ width: `${(r().confidence * 100).toFixed(0)}%` }}
                />
              </div>
              <span class="text-[10px] font-mono text-neutral-500">
                {(r().confidence * 100).toFixed(0)}%
              </span>
            </div>

            <Show when={Object.keys(r().distribution).length > 0}>
              <div class="space-y-1">
                <For each={Object.entries(r().distribution)}>
                  {(entry) => {
                    const [action, prob] = entry as [string, number];
                    return (
                      <div class="flex items-center gap-2 text-[11px]">
                        <span class="w-14 text-neutral-400 capitalize truncate">
                          {action}
                        </span>
                        <div class="flex-1 h-0.5 bg-neutral-800 rounded-full overflow-hidden">
                          <div
                            class="h-full bg-neutral-500 rounded-full"
                            style={{ width: `${(prob * 100).toFixed(0)}%` }}
                          />
                        </div>
                        <span class="font-mono text-neutral-500">
                          {(prob * 100).toFixed(1)}%
                        </span>
                      </div>
                    );
                  }}
                </For>
              </div>
            </Show>

            <div class="text-[10px] text-neutral-600 font-mono">
              EV: {r().ev.toFixed(2)}
            </div>
          </div>
        );
      }}
    </Show>
  );
};

import type { Component } from "solid-js";
