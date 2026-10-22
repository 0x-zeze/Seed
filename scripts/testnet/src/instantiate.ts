import {
  MsgCreateDenom,
  MsgMint,
  MsgInstantiateContract,
} from "@injectivelabs/sdk-ts";
import {
  OFFICIAL_CHAIN_CONSTANTS,
  initDeployerContext,
  handleRawChainError,
  extractAttributeFromTx,
} from "./config.js";
import { loadState, saveState } from "./state.js";

async function main(): Promise<void> {
  console.log("================================================================================");
  console.log(">>> Injective Testnet Deploy: Instantiate Seed Vault");
  console.log(`Target Network : testnet (${OFFICIAL_CHAIN_CONSTANTS.chainId})`);
  console.log(`Explorer Base  : ${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}`);
  console.log("================================================================================\n");

  // 1. Validate state
  const state = loadState();
  if (!state.codeId) {
    console.error("[ERROR] No codeId found in scripts/testnet/state.json.");
    console.error("Please run the store script first: npm run store\n");
    process.exit(1);
  }

  // 2. Read MARKET_IDS from env (abort with clear message if empty)
  const rawMarketIds = process.env.MARKET_IDS;
  if (!rawMarketIds || !rawMarketIds.trim()) {
    console.error("================================================================================");
    console.error("[CRITICAL CONFIGURATION ERROR] MARKET_IDS is empty or undefined!");
    console.error("The Seed Vault requires explicit allowed_markets at instantiation.");
    console.error("Please set MARKET_IDS in scripts/testnet/.env with comma-separated market IDs.");
    console.error("Example: MARKET_IDS=\"0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23\"");
    console.error("================================================================================\n");
    process.exit(1);
  }

  const allowedMarkets = rawMarketIds
    .split(",")
    .map((m) => m.trim())
    .filter((m) => m.length > 0);

  if (allowedMarkets.length === 0) {
    console.error("[ERROR] Parsed MARKET_IDS list is empty. Aborting.\n");
    process.exit(1);
  }

  console.log(`Code ID to instantiate : ${state.codeId}`);
  console.log(`Allowed Market IDs     : ${JSON.stringify(allowedMarkets)}\n`);

  // 3. Initialize testnet context
  const { address, broadcaster } = initDeployerContext();
  console.log(`Deployer / Admin Address: ${address}`);

  // 4. Create TokenFactory test SEED token
  const subdenom = (process.env.SEED_SUBDENOM || "seed").trim();
  const fullSeedDenom = `factory/${address}/${subdenom}`;
  console.log(`Target SEED Denom       : ${fullSeedDenom}`);

  console.log(`Step 4a: Ensuring TokenFactory denom exists...`);
  const msgCreateDenom = MsgCreateDenom.fromJSON({
    sender: address,
    subdenom: subdenom,
  });

  try {
    await broadcaster.broadcast({ msgs: [msgCreateDenom] });
    console.log(`[PASS] Denom created: ${fullSeedDenom}`);
  } catch (createErr: any) {
    const errMsg = (createErr?.message || "").toLowerCase();
    if (errMsg.includes("already exists") || errMsg.includes("duplicate")) {
      console.log(`[INFO] Denom ${fullSeedDenom} already exists on-chain, continuing...`);
    } else {

  // 4b. Mint test SEED tokens to deployer wallet
  const mintAmount = "1000000000000000000000"; // 1,000 units with 18 decimals
  console.log(`Step 4b: Minting ${mintAmount} ${fullSeedDenom} to deployer wallet...`);
  const msgMint = MsgMint.fromJSON({
    sender: address,
    amount: {
      denom: fullSeedDenom,
      amount: mintAmount,
    },
  });

  try {
    await broadcaster.broadcast({ msgs: [msgMint] });
    console.log(`[PASS] Minted test SEED tokens to deployer wallet.`);
  } catch (mintErr) {
    handleRawChainError("TokenFactory Mint", mintErr);
  }

  // 5. Instantiate Seed Vault with EXPLICIT risk parameters (never rely on defaults)
  const instantiateMsg = {
    owner: address,
    seed_denom: fullSeedDenom,
    treasury_denom: "inj",
    trader: address,
    allowed_markets: allowedMarkets,
    max_leverage: "2",
    max_exposure_bps: 3000,
    max_total_margin_bps: 3000,
    max_position_margin: "1000000000000000000", // 1 INJ (10^18)
    max_position_size: "1000000000000000000",   // 1 SEED (10^18)
    cooldown_seconds: 300,
  };

  console.log("\nInstantiate Msg Payload (All Risk Fields Explicit):");
  console.log(JSON.stringify(instantiateMsg, null, 2));

  const msgInstantiate = MsgInstantiateContract.fromJSON({
    sender: address,
    admin: address,
    codeId: state.codeId,
    label: "Seed Vault (Testnet Smoke)",
    msg: instantiateMsg,
  });

  console.log("\nBroadcasting MsgInstantiateContract...");
  let txResponse: any;
  try {
    txResponse = await broadcaster.broadcast({ msgs: [msgInstantiate] });
  } catch (instantiateErr) {
    handleRawChainError("InstantiateContract Broadcast", instantiateErr);
  }

  const txHash: string = txResponse.txHash || txResponse.txhash;
  const explorerLink = `${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}/transaction/${txHash}`;

  const contractAddress =
    extractAttributeFromTx(txResponse, "instantiate", "_contract_address") ||
    extractAttributeFromTx(txResponse, "wasm", "_contract_address");

  if (!contractAddress) {
    console.error("[ERROR] Failed to extract contract address from instantiate events.");
    console.error("Raw response:\n", JSON.stringify(txResponse, null, 2));
    process.exit(1);
  }

  const contractExplorerLink = `${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}/contract/${contractAddress}`;

  console.log("\n=================== INSTANTIATE CONFIRMED ===================");
  console.log(`Contract Address     : ${contractAddress}`);
  console.log(`Contract Explorer    : ${contractExplorerLink}`);
  console.log(`Instantiate Tx Hash  : ${txHash}`);
  console.log(`Transaction Explorer : ${explorerLink}`);
  console.log("=============================================================\n");

  saveState({
    contractAddress,
    instantiateTxHash: txHash,
    seedDenom: fullSeedDenom,
    treasuryDenom: "inj",
    allowedMarkets,
  });
  console.log("Updated scripts/testnet/state.json with contract address and configuration.\n");
}

main().catch((err) => handleRawChainError("Unhandled Instantiate Main Error", err));

      handleRawChainError("TokenFactory CreateDenom", createErr);
    }
  }
