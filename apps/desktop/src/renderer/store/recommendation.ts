export interface RecommendationData {
  action: string;
  amount: number;
  confidence: number;
  distribution: Record<string, number>;
  ev: number;
}

export interface RecommendationEntry {
  tableId: string;
  recommendation: RecommendationData;
  timestampMs: number;
}

import { createStore } from "solid-js/store";

const [recommendations, setRecommendations] = createStore<Record<string, RecommendationEntry>>({});

export { recommendations, setRecommendations };

export function updateRecommendation(
  tableId: string,
  recommendation: RecommendationData,
  timestampMs: number,
) {
  setRecommendations(tableId, {
    tableId,
    recommendation,
    timestampMs,
  });
}
