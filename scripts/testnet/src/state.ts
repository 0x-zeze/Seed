import * as fs from "node:fs";
import * as path from "node:path";
import { SCRIPT_ROOT } from "./config.js";

export interface DeploymentState {
  codeId?: number;
  codeHash?: string;
  storeTxHash?: string;
  contractAddress?: string;
  instantiateTxHash?: string;
  seedDenom?: string;
  treasuryDenom?: string;
  allowedMarkets?: string[];
  updatedAt?: string;
}

export const STATE_FILE_PATH = path.resolve(SCRIPT_ROOT, "state.json");

export function loadState(): DeploymentState {
  if (!fs.existsSync(STATE_FILE_PATH)) {
    return {};
  }
  try {
    const raw = fs.readFileSync(STATE_FILE_PATH, "utf-8");
    return JSON.parse(raw) as DeploymentState;
  } catch (err) {
    console.warn(`[WARN] Failed to parse state file at ${STATE_FILE_PATH}, starting fresh.`);
    return {};
  }
}

export function saveState(patch: Partial<DeploymentState>): DeploymentState {
  const current = loadState();
  const next: DeploymentState = {
    ...current,
    ...patch,
    updatedAt: new Date().toISOString(),
  };
  fs.writeFileSync(STATE_FILE_PATH, JSON.stringify(next, null, 2) + "\n", "utf-8");
  return next;
}
