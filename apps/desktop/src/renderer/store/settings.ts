import { createStore } from "solid-js/store";

export interface Settings {
  theme: "dark" | "light";
  fpsPerTable: number;
  maxTables: number;
  heroSeatOverride: number | null;
}

const [settings, setSettings] = createStore<Settings>({
  theme: "dark",
  fpsPerTable: 30,
  maxTables: 8,
  heroSeatOverride: null,
});

export { settings, setSettings };
