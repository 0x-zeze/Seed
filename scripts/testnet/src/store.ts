import * as fs from "node:fs";
import * as path from "node:path";
import * as crypto from "node:crypto";
import { MsgStoreCode } from "@injectivelabs/sdk-ts";
import {
  WORKSPACE_ROOT,
  OFFICIAL_CHAIN_CONSTANTS,
  initDeployerContext,
  handleRawChainError,
  extractAttributeFromTx,
} from "./config.js";
import { saveState } from "./state.js";

const EXPECTED_HASH = "10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60";

async function main(): Promise<void> {
  console.log("================================================================================");
  console.log(">>> Injective Testnet Deploy: Store CosmWasm Bytecode");
  console.log(`Target Network : testnet (${OFFICIAL_CHAIN_CONSTANTS.chainId})`);
  console.log(`Explorer Base  : ${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}`);
  console.log("================================================================================\n");

  const wasmPath = path.resolve(WORKSPACE_ROOT, "contracts/seed-vault/artifacts/seed_vault.wasm");

  // 1. Check if artifacts/seed_vault.wasm exists
  if (!fs.existsSync(wasmPath)) {
    console.error("================================================================================");
    console.error("[ERROR] Missing compiled CosmWasm binary!");
    console.error(`Expected file: ${wasmPath}`);
    console.error("Please download the 'seed-vault-wasm' artifact from GitHub Actions,");
    console.error("place it at 'contracts/seed-vault/artifacts/seed_vault.wasm', and re-run:");
    console.error("  npm run store");
    console.error("================================================================================\n");
    process.exit(1);
  }

  // 2. Compute SHA256 and compare against expected hash
  const wasmBytes = fs.readFileSync(wasmPath);
  const computedHash = crypto.createHash("sha256").update(wasmBytes).digest("hex");

  console.log(`WASM File Path : ${wasmPath} (${(wasmBytes.length / 1024).toFixed(2)} KB)`);
  console.log(`Computed SHA256: ${computedHash}`);
  console.log(`Expected SHA256: ${EXPECTED_HASH}`);

  if (computedHash.toLowerCase() !== EXPECTED_HASH.toLowerCase()) {
    console.error("\n[CRITICAL ERROR] WASM SHA256 Hash Mismatch!");
    console.error(`Expected: ${EXPECTED_HASH}`);
    console.error(`Computed: ${computedHash}`);
    console.error("Bytecode does not match the verified CI build. Aborting.\n");
    process.exit(1);
  }
  console.log("[PASS] SHA256 checksum verified.\n");

  // 3. Initialize testnet context (aborts if not testnet or if mnemonic missing)
  const { address, broadcaster, wasmApi } = initDeployerContext();
  console.log(`Deployer Address: ${address}`);

  const msgStoreCode = MsgStoreCode.fromJSON({
    sender: address,
    wasmBytes: new Uint8Array(wasmBytes),
  });

  // 4. Simulate gas
  console.log("Simulating gas for MsgStoreCode upload...");
  try {
    const simRes = await broadcaster.simulate({ msgs: [msgStoreCode] });
    const gasUsed = simRes.gasInfo?.gasUsed ? simRes.gasInfo.gasUsed.toString() : "estimated";
    console.log(`[PASS] Gas simulation completed. Estimated gas units: ${gasUsed}\n`);
  } catch (simErr) {
    handleRawChainError("StoreCode Gas Simulation", simErr);
  }

  // 5. Broadcast upload transaction
  console.log("Broadcasting MsgStoreCode transaction to Injective testnet...");
  let txResponse: any;
  try {
    txResponse = await broadcaster.broadcast({ msgs: [msgStoreCode] });
  } catch (broadcastErr) {
    handleRawChainError("StoreCode Broadcast", broadcastErr);
  }

  const txHash: string = txResponse.txHash || txResponse.txhash;
  const explorerLink = `${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}/transaction/${txHash}`;

  const codeIdStr = extractAttributeFromTx(txResponse, "store_code", "code_id");
  if (!codeIdStr) {
    console.error("[ERROR] Failed to extract code_id from transaction response events.");
    console.error("Raw response:\n", JSON.stringify(txResponse, null, 2));
    process.exit(1);
  }

  const codeId = parseInt(codeIdStr, 10);
  console.log("\n=================== STORE CODE CONFIRMED ===================");
  console.log(`Code ID      : ${codeId}`);
  console.log(`Tx Hash      : ${txHash}`);
  console.log(`Explorer Link: ${explorerLink}`);
  console.log("============================================================\n");

  // 6. Query on-chain code info and verify data hash
  console.log(`Querying on-chain code info for Code ID ${codeId}...`);
  try {
    const codeInfoRes = await wasmApi.fetchContractCode(codeId);
    const dataHashField = codeInfoRes.codeInfo?.dataHash;
    const onChainHash = Buffer.isBuffer(dataHashField) || dataHashField instanceof Uint8Array
      ? Buffer.from(dataHashField).toString("hex").toLowerCase()
      : String(dataHashField || "").toLowerCase();

    console.log(`On-Chain Data Hash: ${onChainHash}`);
    if (onChainHash !== EXPECTED_HASH.toLowerCase()) {
      console.error(`[CRITICAL ERROR] On-chain code hash (${onChainHash}) does not match local SHA256 (${EXPECTED_HASH})!`);
      process.exit(1);
    }
    console.log("[PASS] On-chain bytecode verified against local SHA256.\n");
  } catch (queryErr) {
    handleRawChainError("FetchContractCode On-Chain Query", queryErr);
  }

  // 7. Save to state.json
  saveState({
    codeId,
    codeHash: computedHash,
    storeTxHash: txHash,
  });
  console.log("Deployment state successfully saved to scripts/testnet/state.json.\n");
}

main().catch((err) => handleRawChainError("Unhandled Store Main Error", err));
