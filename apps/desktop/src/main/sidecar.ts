import { ChildProcess, fork } from "node:child_process";
import * as path from "node:path";
import { app } from "electron";

export interface RecInput {
  hole_cards: { suit: string; rank: string; confidence: number }[];
  community_cards: { suit: string; rank: string; confidence: number }[];
  pot: number;
  to_call: number;
  min_raise: number;
  stack: number;
  street: string;
  num_opponents: number;
  action_history: Array<{
    seat_id: number;
    action: string;
    amount: number;
    street: string;
  }>;
}

export interface RecOutput {
  action: string;
  amount: number;
  confidence: number;
  distribution: Record<string, number>;
  ev: number;
  processing_time_ms: number;
}

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class RecSidecar {
  private child: ChildProcess | null = null;
  private nextId = 1;
  private pending = new Map<number, PendingRequest>();
  private restarting = false;
  private maxConsecutiveFailures: number;
  private consecutiveFailures = 0;
  private requestTimeout: number;

  constructor(
    private opts: {
      scriptPath?: string;
      requestTimeout?: number;
      maxConsecutiveFailures?: number;
    } = {},
  ) {
    this.requestTimeout = opts.requestTimeout ?? 2000;
    this.maxConsecutiveFailures = opts.maxConsecutiveFailures ?? 3;
  }

  async start(): Promise<void> {
    const scriptPath =
      this.opts.scriptPath ?? this.defaultScriptPath();

    this.child = fork(scriptPath, [], {
      stdio: ["pipe", "pipe", "pipe", "ipc"],
      env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
    });

    this.child.stdout!.setEncoding("utf8");
    let buffer = "";
    this.child.stdout!.on("data", (chunk: string) => {
      buffer += chunk;
      const lines = buffer.split("\n");
      buffer = lines.pop()!;
      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const resp = JSON.parse(line);
          this.handleResponse(resp);
        } catch {
          // ignore malformed lines
        }
      }
    });

    this.child.on("exit", () => {
      for (const [id, pending] of this.pending) {
        clearTimeout(pending.timer);
        pending.reject(new Error("Sidecar process exited"));
        this.pending.delete(id);
      }
    });

    await this.health();
  }

  private defaultScriptPath(): string {
    if (app.isPackaged) {
      return path.join(process.resourcesPath!, "rec-sidecar", "index.js");
    }
    return path.join(__dirname, "..", "..", "..", "..", "rec-sidecar", "index.js");
  }

  private handleResponse(resp: { id?: number; result?: unknown; error?: { code: number; message: string } }) {
    if (resp.id == null) return;
    const pending = this.pending.get(resp.id);
    if (!pending) return;

    clearTimeout(pending.timer);
    this.pending.delete(resp.id);

    if (resp.error) {
      pending.reject(new Error(`JSON-RPC ${resp.error.code}: ${resp.error.message}`));
    } else {
      this.consecutiveFailures = 0;
      pending.resolve(resp.result);
    }
  }

  private call<T>(method: string, params: unknown): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.child || this.child.killed) {
        reject(new Error("Sidecar not running"));
        return;
      }

      const id = this.nextId++;
      const request = { jsonrpc: "2.0", id, method, params };

      const timer = setTimeout(() => {
        this.pending.delete(id);
        this.consecutiveFailures++;
        if (this.consecutiveFailures >= this.maxConsecutiveFailures) {
          this.restart();
        }
        reject(new Error(`Sidecar request timeout after ${this.requestTimeout}ms`));
      }, this.requestTimeout);

      this.pending.set(id, { resolve: (v) => resolve(v as T), reject, timer });

      const line = JSON.stringify(request) + "\n";
      this.child.stdin!.write(line, (err) => {
        if (err) {
          clearTimeout(timer);
          this.pending.delete(id);
          reject(err);
        }
      });
    });
  }

  async health(): Promise<{ ok: boolean; version: string }> {
    return this.call("rec.health", {});
  }

  async recommend(input: RecInput): Promise<RecOutput> {
    return this.call("rec.recommend", input);
  }

  async shutdown(): Promise<void> {
    if (!this.child || this.child.killed) return;

    try {
      await this.call("rec.shutdown", {});
    } catch {
      // ignore — may have already exited
    }

    this.child.kill();
    this.child = null;
  }

  private async restart(): Promise<void> {
    if (this.restarting) return;
    this.restarting = true;

    try {
      if (this.child && !this.child.killed) {
        this.child.kill();
      }
      this.child = null;
      await this.start();
    } finally {
      this.restarting = false;
    }
  }
}
