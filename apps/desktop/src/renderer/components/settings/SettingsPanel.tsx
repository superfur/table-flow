import type { Component } from "solid-js";
import { settings, setSettings } from "../../store/settings";

export const SettingsPanel: Component = () => {
  return (
    <div class="p-6 space-y-6">
      <h2 class="text-lg font-semibold tracking-tight text-neutral-100">
        Settings
      </h2>

      <div class="space-y-4">
        <div>
          <label class="block text-xs font-medium text-neutral-400 mb-1.5">
            Theme
          </label>
          <select
            class="w-full bg-neutral-800 border border-neutral-700 rounded-md
              px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-indigo-500"
            value={settings.theme}
            onChange={(e) => setSettings("theme", e.currentTarget.value as "dark" | "light")}
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </div>

        <div>
          <label class="block text-xs font-medium text-neutral-400 mb-1.5">
            FPS per Table
          </label>
          <input
            type="range"
            min="15"
            max="60"
            step="5"
            value={settings.fpsPerTable}
            onInput={(e) =>
              setSettings("fpsPerTable", Number(e.currentTarget.value))
            }
            class="w-full accent-indigo-500"
          />
          <div class="text-xs text-neutral-500 mt-1 font-mono">
            {settings.fpsPerTable} FPS
          </div>
        </div>

        <div>
          <label class="block text-xs font-medium text-neutral-400 mb-1.5">
            Max Tables
          </label>
          <input
            type="number"
            min="1"
            max="8"
            value={settings.maxTables}
            onChange={(e) =>
              setSettings("maxTables", Math.min(8, Math.max(1, Number(e.currentTarget.value))))
            }
            class="w-full bg-neutral-800 border border-neutral-700 rounded-md
              px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-indigo-500"
          />
        </div>

        <div>
          <label class="block text-xs font-medium text-neutral-400 mb-1.5">
            Hero Seat Override
          </label>
          <input
            type="number"
            min="0"
            max="9"
            placeholder="Auto-detect"
            value={settings.heroSeatOverride ?? ""}
            onChange={(e) => {
              const v = e.currentTarget.value;
              setSettings(
                "heroSeatOverride",
                v === "" ? null : Math.min(9, Math.max(0, Number(v))),
              );
            }}
            class="w-full bg-neutral-800 border border-neutral-700 rounded-md
              px-3 py-2 text-sm text-neutral-200 placeholder-neutral-600
              focus:outline-none focus:border-indigo-500"
          />
          <p class="text-[10px] text-neutral-600 mt-1">
            Leave empty for automatic hero detection
          </p>
        </div>
      </div>
    </div>
  );
};
