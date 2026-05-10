import type { Component } from "solid-js";
import { createSignal, onMount } from "solid-js";

interface Stats {
  totalHands: number;
  handsWithHero: number;
  heroWins: number;
  heroNet: number;
  vpip: number;
  pfr: number;
  winRate: number;
  totalPot: number;
  biggestPot: number;
}

export const SessionStats: Component = () => {
  const [stats, setStats] = createSignal<Stats | null>(null);

  onMount(async () => {
    try {
      const s = await window.electronAPI?.getSessionStats();
      if (s) setStats(s);
    } catch {}
  });

  const fmt = (n: number, decimals = 0) =>
    n.toFixed(decimals);

  const fmtCurrency = (n: number) => {
    const sign = n >= 0 ? "+" : "";
    return `${sign}${fmt(n, 2)}`;
  };

  return (
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
      <StatCard
        label="Hands"
        value={stats()?.totalHands?.toString() ?? "—"}
      />
      <StatCard
        label="Win Rate"
        value={stats() ? `${fmt(stats()!.winRate, 1)}%` : "—"}
        highlight={stats() && stats()!.winRate > 50}
      />
      <StatCard
        label="Profit"
        value={stats() ? fmtCurrency(stats()!.heroNet) : "—"}
        highlight={stats() && stats()!.heroNet > 0}
        negative={stats() && stats()!.heroNet < 0}
      />
      <StatCard
        label="VPIP"
        value={stats() ? `${fmt(stats()!.vpip, 1)}%` : "—"}
      />
      <StatCard
        label="PFR"
        value={stats() ? `${fmt(stats()!.pfr, 1)}%` : "—"}
      />
      <StatCard
        label="Biggest Pot"
        value={stats() ? fmt(stats()!.biggestPot) : "—"}
        highlight={stats() && stats()!.biggestPot > 0}
      />
      <StatCard
        label="Total Pot"
        value={stats() ? fmt(stats()!.totalPot) : "—"}
      />
      <StatCard
        label="Hero Wins"
        value={stats()?.heroWins?.toString() ?? "—"}
      />
    </div>
  );
};

const StatCard: Component<{
  label: string;
  value: string;
  highlight?: boolean;
  negative?: boolean;
}> = (props) => {
  const colorClass = () => {
    if (props.negative) return "text-red-400";
    if (props.highlight) return "text-emerald-400";
    return "text-neutral-200";
  };

  return (
    <div class="rounded-lg border border-neutral-800 bg-neutral-800/50 p-3">
      <div class="text-[10px] font-medium uppercase tracking-wider text-neutral-500 mb-1">
        {props.label}
      </div>
      <div class={`text-lg font-mono font-semibold ${colorClass()}`}>
        {props.value}
      </div>
    </div>
  );
};
