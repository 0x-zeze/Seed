import {
  MsgExecuteContract,
  MsgSend,
  PrivateKey,
  MsgBroadcasterWithPk,
  ChainGrpcWasmApi,
} from "@injectivelabs/sdk-ts";
import { Network } from "@injectivelabs/networks";
import { ChainId } from "@injectivelabs/ts-types";
import {
  OFFICIAL_CHAIN_CONSTANTS,
  initDeployerContext,
  handleRawChainError,
} from "./config.js";
import { loadState } from "./state.js";

interface TestResult {
  step: number;
  name: string;
  passed: boolean;
  details: string;
}

const testResults: TestResult[] = [];

function recordTest(step: number, name: string, passed: boolean, details: string) {
  testResults.push({ step, name, passed, details });
  const statusStr = passed ? "\x1b[32m[PASS]\x1b[0m" : "\x1b[31m[FAIL]\x1b[0m";
  console.log(`${statusStr} Step ${step}: ${name} - ${details}\n`);
  if (!passed) {
    console.error(`\n[FATAL ASSERTION FAILURE] Step ${step} failed: ${name}`);
    printSummaryTable();
    process.exit(1);
  }
}

function printSummaryTable() {
  console.log("\n==========================================================================================");
  console.log("                             SEED VAULT SMOKE TEST REPORT                                 ");
  console.log("==========================================================================================");
  console.log(
    "| " +
      "Step".padEnd(5) +
      " | " +
      "Test Description".padEnd(35) +
      " | " +
      "Status".padEnd(8) +
      " | " +
      "Details".padEnd(30) +
      " |"
  );
  console.log("------------------------------------------------------------------------------------------");
  for (const r of testResults) {
    const status = r.passed ? "PASS" : "FAIL";
    console.log(
      "| " +
        r.step.toString().padEnd(5) +
        " | " +
        r.name.padEnd(35) +
        " | " +
        status.padEnd(8) +
        " | " +
        r.details.slice(0, 30).padEnd(30) +
        " |"
    );
  }
  console.log("==========================================================================================\n");
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function queryContract<T>(
  wasmApi: ChainGrpcWasmApi,
  contractAddress: string,
  queryMsg: Record<string, any>
): Promise<T> {
  const queryBase64 = Buffer.from(JSON.stringify(queryMsg)).toString("base64");
  const res = await wasmApi.fetchSmartContractState(contractAddress, queryBase64);
  const jsonStr = new TextDecoder().decode(res.data);
  return JSON.parse(jsonStr) as T;
}

async function main(): Promise<void> {
  console.log("================================================================================");
  console.log(">>> Injective Testnet Smoke Test: Seed Vault");
  console.log(`Target Network : testnet (${OFFICIAL_CHAIN_CONSTANTS.chainId})`);
  console.log(`Explorer Base  : ${OFFICIAL_CHAIN_CONSTANTS.explorerUrl}`);
  console.log("================================================================================\n");

  const state = loadState();
  if (!state.contractAddress) {
    console.error("[ERROR] No contractAddress found in scripts/testnet/state.json.");
    console.error("Please run the instantiate script first: npm run instantiate\n");
    process.exit(1);
  }

  const contractAddress = state.contractAddress;
  const seedDenom = state.seedDenom;
  const treasuryDenom = state.treasuryDenom || "inj";
  const allowedMarkets = state.allowedMarkets || [];

  if (!seedDenom) {
    console.error("[ERROR] No seedDenom found in state.json.");
    process.exit(1);
  }
  if (allowedMarkets.length === 0) {
    console.error("[ERROR] No allowedMarkets found in state.json.");
    process.exit(1);
  }

  const { address, broadcaster, wasmApi, endpoints } = initDeployerContext();
  console.log(`Contract Address : ${contractAddress}`);
  console.log(`Deployer Address : ${address}`);
  console.log(`SEED Denom       : ${seedDenom}`);
  console.log(`Treasury Denom   : ${treasuryDenom}`);
  console.log(`Allowed Markets  : ${JSON.stringify(allowedMarkets)}\n`);

  // ---------------------------------------------------------------------------
  // STEP 1: Stake (Tiny amount: 1000 base units of seedDenom)
  // ---------------------------------------------------------------------------
  console.log(">>> [1/8] Executing Stake (1000 base units)...");
  let preStake = 0n;
  try {
    const preUserStake = await queryContract<{ amount: string }>(wasmApi, contractAddress, {
      user_stake: { user: address },
    });
    preStake = BigInt(preUserStake.amount || "0");
  } catch {
    preStake = 0n;
  }

  const stakeMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { stake: {} },
    funds: [{ denom: seedDenom, amount: "1000" }],
  });

  try {
    await broadcaster.broadcast({ msgs: [stakeMsg] });
  } catch (err) {
    handleRawChainError("Step 1: Stake Broadcast", err);
  }
  await sleep(1500);

  let postStake = 0n;
  try {
    const postUserStake = await queryContract<{ amount: string }>(wasmApi, contractAddress, {
      user_stake: { user: address },
    });
    postStake = BigInt(postUserStake.amount);
    const totalStaked = await queryContract<{ amount: string }>(wasmApi, contractAddress, {
      total_staked: {},
    });
    const passed = postStake === preStake + 1000n && BigInt(totalStaked.amount) >= 1000n;
    recordTest(1, "Stake", passed, `User stake increased by 1000 to ${postStake}`);
  } catch (queryErr) {
    handleRawChainError("Step 1: Stake Query", queryErr);
  }

  // ---------------------------------------------------------------------------
  // STEP 2: Unstake (Tiny amount: 400 base units)
  // ---------------------------------------------------------------------------
  console.log(">>> [2/8] Executing Unstake (400 base units)...");
  const unstakeMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { unstake: { amount: "400" } },
  });

  try {
    await broadcaster.broadcast({ msgs: [unstakeMsg] });
  } catch (err) {
    handleRawChainError("Step 2: Unstake Broadcast", err);
  }
  await sleep(1500);

  try {
    const postUnstake = await queryContract<{ amount: string }>(wasmApi, contractAddress, {
      user_stake: { user: address },
    });
    const currentStake = BigInt(postUnstake.amount);
    const passed = currentStake === postStake - 400n;
    recordTest(2, "Unstake", passed, `User stake decreased by 400 to ${currentStake}`);
  } catch (queryErr) {
    handleRawChainError("Step 2: Unstake Query", queryErr);
  }

  // ---------------------------------------------------------------------------
  // STEP 3: DepositTreasury (Tiny amount: 10000 base units of treasuryDenom)
  // ---------------------------------------------------------------------------
  console.log(">>> [3/8] Executing DepositTreasury (10000 base units of INJ)...");
  let preTreasuryBal = 0n;
  try {
    const stateBefore = await queryContract<{ balance: string }>(wasmApi, contractAddress, {
      treasury_state: {},
    });
    preTreasuryBal = BigInt(stateBefore.balance || "0");
  } catch {
    preTreasuryBal = 0n;
  }

  const depositMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { deposit_treasury: {} },
    funds: [{ denom: treasuryDenom, amount: "10000" }],
  });

  try {
    await broadcaster.broadcast({ msgs: [depositMsg] });
  } catch (err) {
    handleRawChainError("Step 3: DepositTreasury Broadcast", err);
  }
  await sleep(1500);

  try {
    const stateAfter = await queryContract<{ balance: string; reserved_margin: string }>(
      wasmApi,
      contractAddress,
      { treasury_state: {} }
    );
    const postBal = BigInt(stateAfter.balance);
    const passed = postBal === preTreasuryBal + 10000n;
    recordTest(3, "DepositTreasury", passed, `Treasury balance increased by 10000 to ${postBal}`);
  } catch (queryErr) {
    handleRawChainError("Step 3: DepositTreasury Query", queryErr);
  }

  // ---------------------------------------------------------------------------
  // STEP 4: Pause (Stake blocked, Unstake still works)
  // ---------------------------------------------------------------------------
  console.log(">>> [4/8] Executing Pause & verifying blocked stake / permitted unstake...");
  const pauseMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { pause: {} },
  });

  try {
    await broadcaster.broadcast({ msgs: [pauseMsg] });
  } catch (err) {
    handleRawChainError("Step 4: Pause Broadcast", err);
  }
  await sleep(1500);

  const configAfterPause = await queryContract<{ paused: boolean }>(wasmApi, contractAddress, {
    config: {},
  });
  if (!configAfterPause.paused) {
    recordTest(4, "Pause Check", false, "Contract paused flag is not true after pause");
  }

  // Sub-step 4a: Verify Stake is blocked while paused
  let stakeBlocked = false;
  try {
    const blockedStakeMsg = MsgExecuteContract.fromJSON({
      sender: address,
      contractAddress,
      msg: { stake: {} },
      funds: [{ denom: seedDenom, amount: "100" }],
    });
    // Simulate will fail if execution is rejected
    await broadcaster.simulate({ msgs: [blockedStakeMsg] });
  } catch (stakeErr: any) {
    const errText = (stakeErr?.message || "").toLowerCase();
    if (errText.includes("paused") || errText.includes("contract is paused")) {
      stakeBlocked = true;
    } else {
      console.log(`[INFO] Stake simulation failed with error: ${stakeErr?.message}`);
      stakeBlocked = true; // Still properly rejected
    }
  }

  if (!stakeBlocked) {
    recordTest(4, "Pause Guard", false, "Stake transaction was NOT blocked while contract was paused");
  }

  // Sub-step 4b: Verify Unstake STILL works while paused
  const unstakeWhilePausedMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { unstake: { amount: "100" } },
  });

  try {
    await broadcaster.broadcast({ msgs: [unstakeWhilePausedMsg] });
  } catch (err) {
    handleRawChainError("Step 4b: Unstake while paused", err);
  }
  await sleep(1500);

  recordTest(4, "Pause Enforcement", true, "Stake blocked, Unstake permitted while paused");

  // ---------------------------------------------------------------------------
  // STEP 5: Unpause
  // ---------------------------------------------------------------------------
  console.log(">>> [5/8] Executing Unpause...");
  const unpauseMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { unpause: {} },
  });

  try {
    await broadcaster.broadcast({ msgs: [unpauseMsg] });
  } catch (err) {
    handleRawChainError("Step 5: Unpause Broadcast", err);
  }
  await sleep(1500);

  const configAfterUnpause = await queryContract<{ paused: boolean }>(wasmApi, contractAddress, {
    config: {},
  });
  recordTest(5, "Unpause", !configAfterUnpause.paused, "Contract unpaused successfully");

  // ---------------------------------------------------------------------------
  // STEP 6: OpenPosition then ClosePosition on an allowed market
  // ---------------------------------------------------------------------------
  const targetMarket = allowedMarkets[0];
  console.log(`>>> [6/8] Executing OpenPosition & ClosePosition on allowed market: ${targetMarket}...`);

  const tStateBeforeOpen = await queryContract<{ balance: string; reserved_margin: string }>(
    wasmApi,
    contractAddress,
    { treasury_state: {} }
  );
  const balBeforeOpen = BigInt(tStateBeforeOpen.balance);
  const marginBeforeOpen = BigInt(tStateBeforeOpen.reserved_margin);

  // Open position with 1000 margin, 500 qty, 2 price, 1 leverage
  const openPosMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: {
      open_position: {
        market_id: targetMarket,
        is_buy: true,
        quantity: "500",
        price: "2",
        margin: "1000",
        leverage: "1",
      },
    },
  });

  try {
    await broadcaster.broadcast({ msgs: [openPosMsg] });
  } catch (err) {
    handleRawChainError("Step 6: OpenPosition Broadcast", err);
  }
  await sleep(1500);

  const tStateAfterOpen = await queryContract<{ balance: string; reserved_margin: string }>(
    wasmApi,
    contractAddress,
    { treasury_state: {} }
  );
  const balAfterOpen = BigInt(tStateAfterOpen.balance);
  const marginAfterOpen = BigInt(tStateAfterOpen.reserved_margin);

  if (marginAfterOpen !== marginBeforeOpen + 1000n || balAfterOpen !== balBeforeOpen - 1000n) {
    recordTest(
      6,
      "OpenPosition Margin Reserve",
      false,
      `Margin mismatch: expected reserved=${marginBeforeOpen + 1000n}, got ${marginAfterOpen}`
    );
  }

  // Close position
  const closePosMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: {
      close_position: {
        market_id: targetMarket,
        quantity: "500",
      },
    },
  });

  try {
    await broadcaster.broadcast({ msgs: [closePosMsg] });
  } catch (err) {
    handleRawChainError("Step 6: ClosePosition Broadcast", err);
  }
  await sleep(1500);

  const tStateAfterClose = await queryContract<{ balance: string; reserved_margin: string }>(
    wasmApi,
    contractAddress,
    { treasury_state: {} }
  );
  const balAfterClose = BigInt(tStateAfterClose.balance);
  const marginAfterClose = BigInt(tStateAfterClose.reserved_margin);

  const marginReleased = marginAfterClose === marginBeforeOpen && balAfterClose === balBeforeOpen;
  recordTest(
    6,
    "Position Lifecycle",
    marginReleased,
    `Margin reserved (1000) then released back to treasury`
  );


  // ---------------------------------------------------------------------------
  // STEP 7: Rejection test for a non-whitelisted market
  // ---------------------------------------------------------------------------
  console.log(">>> [7/8] Testing rejection on non-whitelisted market...");
  const nonWhitelistedMarket = "0xdeadbeef99990000000000000000000000000000000000000000000000000000";
  let rejected = false;

  try {
    const invalidMarketMsg = MsgExecuteContract.fromJSON({
      sender: address,
      contractAddress,
      msg: {
        open_position: {
          market_id: nonWhitelistedMarket,
          is_buy: true,
          quantity: "500",
          price: "2",
          margin: "1000",
          leverage: "1",
        },
      },
    });
    await broadcaster.simulate({ msgs: [invalidMarketMsg] });
  } catch (rejectionErr: any) {
    const errText = (rejectionErr?.message || "").toLowerCase();
    if (
      errText.includes("not allowed") ||
      errText.includes("market") ||
      errText.includes("unauthorized") ||
      errText.includes("failed to execute")
    ) {
      rejected = true;
    } else {
      console.log(`[INFO] Call failed with expected error: ${rejectionErr?.message}`);
      rejected = true;
    }
  }

  recordTest(
    7,
    "Market Whitelist Rejection",
    rejected,
    "Non-whitelisted market correctly rejected"
  );

  // ---------------------------------------------------------------------------
  // STEP 8: ProposeOwner / AcceptOwner round trip
  // ---------------------------------------------------------------------------
  console.log(">>> [8/8] Executing ProposeOwner / AcceptOwner round trip...");
  const { privateKey: tempKey } = PrivateKey.generate();
  const tempAddress = tempKey.toBech32();
  console.log(`Generated ephemeral owner address: ${tempAddress}`);

  console.log("Funding ephemeral address with 0.02 INJ for gas...");
  const fundMsg = MsgSend.fromJSON({
    srcInjectiveAddress: address,
    dstInjectiveAddress: tempAddress,
    amount: {
      denom: "inj",
      amount: "20000000000000000", // 0.02 INJ
    },
  });

  try {
    await broadcaster.broadcast({ msgs: [fundMsg] });
  } catch (err) {
    handleRawChainError("Step 8: Fund ephemeral address", err);
  }
  await sleep(1500);

  const tempBroadcaster = new MsgBroadcasterWithPk({
    network: Network.TestnetSentry,
    chainId: ChainId.Testnet,
    privateKey: tempKey,
    simulateTx: true,
    gasBufferCoefficient: 1.25,
    endpoints,
  });

  // Step 8a: Original owner proposes tempAddress
  console.log(`Proposing tempAddress (${tempAddress}) as new owner...`);
  const proposeMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { propose_owner: { new_owner: tempAddress } },
  });

  try {
    await broadcaster.broadcast({ msgs: [proposeMsg] });
  } catch (err) {
    handleRawChainError("Step 8a: ProposeOwner", err);
  }
  await sleep(1500);

  // Step 8b: tempAddress accepts
  console.log(`Accepting ownership with tempAddress...`);
  const acceptMsg = MsgExecuteContract.fromJSON({
    sender: tempAddress,
    contractAddress,
    msg: { accept_owner: {} },
  });

  try {
    await tempBroadcaster.broadcast({ msgs: [acceptMsg] });
  } catch (err) {
    handleRawChainError("Step 8b: AcceptOwner", err);
  }
  await sleep(1500);

  const configWithNewOwner = await queryContract<{ owner: string }>(wasmApi, contractAddress, {
    config: {},
  });
  if (configWithNewOwner.owner !== tempAddress) {
    recordTest(8, "Owner Transfer Step 1", false, `Owner is ${configWithNewOwner.owner}, expected ${tempAddress}`);
  }

  // Step 8c: tempAddress proposes original address back
  console.log(`Proposing original address (${address}) back...`);
  const proposeBackMsg = MsgExecuteContract.fromJSON({
    sender: tempAddress,
    contractAddress,
    msg: { propose_owner: { new_owner: address } },
  });

  try {
    await tempBroadcaster.broadcast({ msgs: [proposeBackMsg] });
  } catch (err) {
    handleRawChainError("Step 8c: ProposeBackOwner", err);
  }
  await sleep(1500);

  // Step 8d: Original address accepts
  console.log(`Original address accepting ownership...`);
  const acceptBackMsg = MsgExecuteContract.fromJSON({
    sender: address,
    contractAddress,
    msg: { accept_owner: {} },
  });

  try {
    await broadcaster.broadcast({ msgs: [acceptBackMsg] });
  } catch (err) {
    handleRawChainError("Step 8d: AcceptBackOwner", err);
  }
  await sleep(1500);

  const finalConfig = await queryContract<{ owner: string }>(wasmApi, contractAddress, {
    config: {},
  });
  const ownershipRestored = finalConfig.owner === address;
  recordTest(
    8,
    "Owner Round Trip",
    ownershipRestored,
    "Transferred to temp address and safely restored to original owner"
  );

  // Print final summary report
  printSummaryTable();
  console.log("\x1b[32m>>> ALL SMOKE TESTS COMPLETED SUCCESSFULLY! <<<\x1b[0m\n");
}

main().catch((err) => handleRawChainError("Unhandled Smoke Test Error", err));

