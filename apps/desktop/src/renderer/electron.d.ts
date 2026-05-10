export {};

declare global {
  interface Window {
    electronAPI: {
      onStateUpdate(cb: (event: unknown) => void): () => void;
      onRecommendationUpdate(cb: (event: unknown) => void): () => void;
      onError(cb: (event: unknown) => void): () => void;
      discoverTables(): Promise<string[]>;
      startCapture(config: {
        tableId: string;
        windowTitle: string;
      }): Promise<void>;
      stopCapture(tableId: string): Promise<void>;
      getTableState(tableId: string): Promise<unknown>;
      calibrateTable(tableId: string): Promise<unknown>;
      shutdown(): Promise<void>;
      getRecommendation(input: unknown): Promise<{
        action: string;
        amount: number;
        confidence: number;
        distribution: Record<string, number>;
        ev: number;
        processing_time_ms: number;
      }>;
      sidecarHealth(): Promise<{ ok: boolean; version: string }>;
      getSessionStats(): Promise<{
        totalHands: number;
        handsWithHero: number;
        heroWins: number;
        heroNet: number;
        vpip: number;
        pfr: number;
        winRate: number;
        totalPot: number;
        biggestPot: number;
      }>;
    };
  }
}
