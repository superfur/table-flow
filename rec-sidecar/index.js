#!/usr/bin/env node
// TableFlow Recommendation Sidecar
// JSON-RPC 2.0 over stdin/stdout (line-delimited)
//
// Methods:
//   rec.recommend(input) → RecOutput
//   rec.health()         → { ok, version }
//   rec.shutdown()       → { ok }

const readline = require("readline");

const VERSION = "0.1.0";

// ─── Hand Evaluation Helpers ────────────────────────────────────────────────

const RANK_VALUE = {
  2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 8: 8, 9: 9, T: 10,
  J: 11, Q: 12, K: 13, A: 14,
  Two: 2, Three: 3, Four: 4, Five: 5, Six: 6, Seven: 7, Eight: 8,
  Nine: 9, Ten: 10, Jack: 11, Queen: 12, King: 13, Ace: 14,
};

function handStrength(holeCards, communityCards) {
  const allCards = [...holeCards, ...communityCards];
  const values = allCards.map((c) => RANK_VALUE[c.rank] || 2);
  const suits = allCards.map((c) => c.suit);

  let isPair = false;
  let isSuited = holeCards[0].suit === holeCards[1].suit;
  let highCard = Math.max(values[0], values[1]);
  let lowCard = Math.min(values[0], values[1]);

  if (values[0] === values[1]) {
    isPair = true;
  }

  // Check flush potential
  const suitCounts = {};
  for (const s of suits) {
    suitCounts[s] = (suitCounts[s] || 0) + 1;
  }
  const maxFlush = Math.max(...Object.values(suitCounts));

  // Check straight potential
  const uniqueVals = [...new Set(values)].sort((a, b) => a - b);
  let maxStraightLen = 1;
  let curLen = 1;
  for (let i = 1; i < uniqueVals.length; i++) {
    if (uniqueVals[i] === uniqueVals[i - 1] + 1) {
      curLen++;
      maxStraightLen = Math.max(maxStraightLen, curLen);
    } else {
      curLen = 1;
    }
  }

  const isPreflop = communityCards.length === 0;
  let strength = 0;

  // Pair bonus
  if (isPair) {
    strength += 0.35 + (highCard / 14) * 0.35;
  }

  // High cards — much more important preflop
  if (isPreflop) {
    strength += (highCard / 14) * 0.25 + (lowCard / 14) * 0.1;
  } else {
    strength += (highCard / 14) * 0.1 + (lowCard / 14) * 0.03;
  }

  // Suited bonus
  if (isSuited) strength += isPreflop ? 0.06 : 0.04;
  if (maxFlush >= 5) strength += 0.3;
  else if (maxFlush >= 4) strength += 0.1;

  // Straight bonus
  if (maxStraightLen >= 5) strength += 0.3;
  else if (maxStraightLen >= 4) strength += 0.1;

  // Connected bonus
  if (!isPair && Math.abs(values[0] - values[1]) <= 2) strength += 0.04;

  return Math.min(strength, 1.0);
}

function recommend(input) {
  const t0 = Date.now();
  const street = (input.street || "preflop").toLowerCase();

  const holeCards = input.hole_cards || [];
  const communityCards = input.community_cards || [];
  const pot = input.pot || 0;
  const toCall = input.to_call || 0;
  const minRaise = input.min_raise || 0;
  const stack = input.stack || 0;
  const numOpponents = input.num_opponents || 1;

  const strength = handStrength(holeCards, communityCards);

  // Adjust for position and opponents
  let adjustedStrength = strength;
  if (numOpponents > 3) adjustedStrength *= 0.9;

  let action, amount = 0, foldProb, callProb, raiseProb;

  if (adjustedStrength >= 0.6) {
    // Strong hand
    const raiseMultiplier = adjustedStrength >= 0.85 ? 0.75 : 0.5;
    amount = Math.min(Math.round(pot * raiseMultiplier), stack);
    action = "raise";
    raiseProb = 0.55 + adjustedStrength * 0.25;
    callProb = 0.25;
    foldProb = 1 - raiseProb - callProb;
  } else if (adjustedStrength >= 0.3) {
    // Medium hand
    if (toCall > stack * 0.3) {
      action = "fold";
      foldProb = 0.55;
      callProb = 0.35;
      raiseProb = 0.1;
    } else if (toCall === 0) {
      action = "check";
      callProb = 0.7;
      raiseProb = 0.15;
      foldProb = 0.15;
    } else {
      action = "call";
      callProb = 0.6;
      raiseProb = 0.15;
      foldProb = 0.25;
    }
  } else {
    // Weak hand
    if (toCall === 0) {
      action = "check";
      callProb = 0.7;
      foldProb = 0.2;
      raiseProb = 0.1;
    } else if (toCall < stack * 0.05) {
      action = "call";
      callProb = 0.5;
      foldProb = 0.4;
      raiseProb = 0.1;
    } else {
      action = "fold";
      foldProb = 0.75;
      callProb = 0.15;
      raiseProb = 0.1;
    }
  }

  // Ensure probabilities sum to 1
  const total = foldProb + callProb + raiseProb;
  foldProb /= total;
  callProb /= total;
  raiseProb /= total;

  const ev = adjustedStrength * pot - (1 - adjustedStrength) * toCall;

  return {
    action,
    amount: Math.max(0, amount),
    confidence: Math.min(0.5 + adjustedStrength * 0.4, 0.98),
    distribution: {
      fold: Math.round(foldProb * 1000) / 1000,
      call: Math.round(callProb * 1000) / 1000,
      raise: Math.round(raiseProb * 1000) / 1000,
    },
    ev: Math.round(ev * 100) / 100,
    processing_time_ms: Date.now() - t0,
  };
}

// ─── JSON-RPC 2.0 Server ────────────────────────────────────────────────────

const handlers = {
  "rec.recommend": (params) => recommend(params),
  "rec.health": () => ({ ok: true, version: VERSION }),
  "rec.shutdown": () => {
    setTimeout(() => process.exit(0), 50);
    return { ok: true };
  },
};

const rl = readline.createInterface({ input: process.stdin });

rl.on("line", (line) => {
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    respond(request?.id, null, { code: -32700, message: "Parse error" });
    return;
  }

  const { id, method, params } = request;
  const handler = handlers[method];

  if (!handler) {
    respond(id, null, { code: -32601, message: "Method not found" });
    return;
  }

  try {
    const result = handler(params || {});
    respond(id, result, null);
  } catch (err) {
    respond(id, null, { code: -32603, message: err.message });
  }
});

function respond(id, result, error) {
  const response = { jsonrpc: "2.0" };
  if (id !== undefined) response.id = id;
  if (error) response.error = error;
  else response.result = result;
  process.stdout.write(JSON.stringify(response) + "\n");
}

// Signal ready
process.stderr.write("TableFlow rec-sidecar ready\n");
