import * as path from "node:path";
import * as fs from "node:fs";
import { fileURLToPath } from "node:url";
import dotenv from "dotenv";
import {
  PrivateKey,
  MsgBroadcasterWithPk,
  ChainGrpcWasmApi,
  IndexerGrpcDerivativesApi,
} from "@injectivelabs/sdk-ts";
import { Network, getNetworkEndpoints } from "@injectivelabs/networks";
import { ChainId } from "@injectivelabs/ts-types";

// -----------------------------------------------------------------------------
// INJECTIVE TESTNET CONSTANTS & OFFICIAL SOURCE REFERENCES
// -----------------------------------------------------------------------------
//
// 1. Chain ID: "injective-888"
//    Source: https://docs.injective.network/developers/network-information
//    SDK Source: @injectivelabs/ts-types (ChainId.Testnet = "injective-888")
//
// 2. Chain RPC Endpoint: "https://testnet.sentry.tm.injective.network:443"
//    Source: https://docs.injective.network/infra/public-endpoints
//    SDK Source: @injectivelabs/networks (getNetworkEndpoints(Network.Testnet).rpc)
//
// 3. Chain REST / LCD Endpoint: "https://testnet.sentry.lcd.injective.network:443"
//    Source: https://docs.injective.network/infra/public-endpoints
//    SDK Source: @injectivelabs/networks (getNetworkEndpoints(Network.Testnet).rest)
//
// 4. Chain gRPC Endpoint: "https://testnet.sentry.chain.grpc-web.injective.network:443"
//    Source: https://docs.injective.network/infra/public-endpoints
//    SDK Source: @injectivelabs/networks (getNetworkEndpoints(Network.Testnet).grpc)
//
// 5. Indexer gRPC Endpoint: "https://testnet.sentry.exchange.grpc-web.injective.network:443"
//    Source: https://docs.injective.network/infra/public-endpoints
//    SDK Source: @injectivelabs/networks (getNetworkEndpoints(Network.Testnet).indexer)
//
// 6. Testnet Explorer URL: "https://testnet.explorer.injective.network"
//    Source: https://docs.injective.network/infra/public-endpoints
//    Source: https://docs.injective.network/developers/network-information
//
// 7. Testnet Faucet URL: "https://testnet.faucet.injective.network/"
//    Source: https://docs.injective.network/infra/public-endpoints
//
// 8. Gas Price: "500000000inj" (0.0000000005 INJ per gas unit, default testnet gas price)
//    Validator Minimum Gas Price Floor: 160,000,000inj
//    Source: https://docs.injective.network/defi/transaction-fees
//
// 9. TokenFactory Denom Creation Fee: "1000000000000000000inj" (1 INJ)
//    Source: On-chain query via https://testnet.sentry.lcd.injective.network:443/injective/tokenfactory/v1beta1/params
//    Source: https://docs.injective.network/developers-native/injective/tokenfactory/05_params
//
// 10. CosmWasm Code Upload Permission: "Everybody" (Permissionless on testnet)
//     Source: On-chain query via https://testnet.sentry.lcd.injective.network:443/cosmwasm/wasm/v1/codes/params
//     Source: https://docs.injective.network/developers-native/injective/wasmx/05_params
// -----------------------------------------------------------------------------

export const OFFICIAL_CHAIN_CONSTANTS = {
  chainId: "injective-888" as const,
  sources: {
    chainId: "https://docs.injective.network/developers/network-information",
    rpc: "https://docs.injective.network/infra/public-endpoints",
    rest: "https://docs.injective.network/infra/public-endpoints",
    grpc: "https://docs.injective.network/infra/public-endpoints",
    indexer: "https://docs.injective.network/infra/public-endpoints",
    explorer: "https://docs.injective.network/infra/public-endpoints",
    faucet: "https://docs.injective.network/infra/public-endpoints",
    gasPrice: "https://docs.injective.network/defi/transaction-fees",
    tokenFactoryFee: "https://testnet.sentry.lcd.injective.network/injective/tokenfactory/v1beta1/params",
    codeUploadPermission: "https://testnet.sentry.lcd.injective.network/cosmwasm/wasm/v1/codes/params",
  },
  explorerUrl: "https://testnet.explorer.injective.network",
  faucetUrl: "https://testnet.faucet.injective.network/",
  gasPrice: "500000000inj",
  denomCreationFee: "1000000000000000000inj", // 1 INJ
  codeUploadPermission: "Everybody",
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
export const SCRIPT_ROOT = path.resolve(__dirname, "..");

// -----------------------------------------------------------------------------
// STRICT TESTNET GUARD
// -----------------------------------------------------------------------------
export function assertTestnetGuard(): void {
  const configuredNetwork = (process.env.INJECTIVE_NETWORK || "testnet").trim().toLowerCase();

  if (configuredNetwork !== "testnet") {
    console.error("\n[CRITICAL ERROR] Network Guard Violation:");
    console.error(`INJECTIVE_NETWORK is set to '${configuredNetwork}'. ONLY 'testnet' is permitted.`);
    console.error("Mainnet operations are strictly prohibited.\n");
    process.exit(1);
  }

  if (OFFICIAL_CHAIN_CONSTANTS.chainId !== "injective-888") {
    console.error("\n[CRITICAL ERROR] Target Chain ID is not injective-888! Halting immediately.\n");
    process.exit(1);
  }
}

// -----------------------------------------------------------------------------
// WALLET / MNEMONIC MANAGEMENT (ZERO PRINTING / LOGGING INVARIANT)
// -----------------------------------------------------------------------------
export interface DeployerContext {
  privateKey: PrivateKey;
  address: string;
  broadcaster: MsgBroadcasterWithPk;
  wasmApi: ChainGrpcWasmApi;
  derivativesApi: IndexerGrpcDerivativesApi;
  endpoints: ReturnType<typeof getNetworkEndpoints>;
}

export function initDeployerContext(): DeployerContext {
  assertTestnetGuard();

  const rawMnemonic = process.env.MNEMONIC;
  if (!rawMnemonic || !rawMnemonic.trim() || rawMnemonic.includes("word1 word2")) {
    console.error("\n[CONFIGURATION ERROR] Missing or unpopulated MNEMONIC in scripts/testnet/.env.");
    console.error("Please copy .env.example to .env and configure your testnet wallet mnemonic phrase.");
    console.error(`Expected file: ${ENV_FILE_PATH}\n`);
    process.exit(1);
  }

  const mnemonic = rawMnemonic.trim();
  let privateKey: PrivateKey;

  try {
    privateKey = PrivateKey.fromMnemonic(mnemonic);
  } catch (err) {
    console.error("\n[KEY ERROR] Failed to derive Injective private key from provided mnemonic.");
    console.error("Ensure the mnemonic is a valid 12 or 24-word BIP39 phrase.\n");
    process.exit(1);
  }

  const address = privateKey.toBech32();
  const endpoints = getNetworkEndpoints(Network.TestnetSentry);

  const broadcaster = new MsgBroadcasterWithPk({
    network: Network.TestnetSentry,
    chainId: ChainId.Testnet,
    privateKey: privateKey,
    simulateTx: true,
    gasBufferCoefficient: 1.25,
    endpoints: endpoints,
  });

  const wasmApi = new ChainGrpcWasmApi(endpoints.grpc);
  const derivativesApi = new IndexerGrpcDerivativesApi(endpoints.indexer);

  return {
    privateKey,
    address,
    broadcaster,
    wasmApi,
    derivativesApi,
    endpoints,
  };
}

// -----------------------------------------------------------------------------
// RAW CHAIN ERROR HANDLER
// -----------------------------------------------------------------------------
export function handleRawChainError(step: string, err: unknown): never {
  console.error(`\n=================== [FATAL] RAW CHAIN ERROR DURING: ${step} ===================`);
  if (err instanceof Error) {
    console.error("ERROR MESSAGE:", err.message);
    const anyErr = err as unknown as Record<string, unknown>;
    if (anyErr.rawLog) {
      console.error("RAW LOG:", anyErr.rawLog);
    }
    if (anyErr.contextCode !== undefined) {
      console.error("CONTEXT CODE:", anyErr.contextCode);
    }
    if (anyErr.contextModule !== undefined) {
      console.error("CONTEXT MODULE:", anyErr.contextModule);
    }
    if (anyErr.originalMessage) {
      console.error("ORIGINAL MESSAGE:", anyErr.originalMessage);
    }
    if (err.stack) {
      console.error("\nSTACK TRACE:\n", err.stack);
    }
  } else {
    console.error("RAW ERROR OBJECT:", String(err));
  }
  console.error("================================================================================\n");
  process.exit(1);
}

// -----------------------------------------------------------------------------
// LOG / EVENT EXTRACTION HELPER
// -----------------------------------------------------------------------------
export function extractAttributeFromTx(
  txResponse: any,
  eventType: string,
  attributeKey: string
): string | undefined {
  if (txResponse.rawLog) {
    try {
      const logs = JSON.parse(txResponse.rawLog);
      for (const log of logs) {
        for (const event of log.events || []) {
          if (event.type === eventType) {
            for (const attr of event.attributes || []) {
              if (attr.key === attributeKey) {
                return attr.value;
              }
            }
          }
        }
      }
    } catch {
      // ignore non-json
    }
  }

  if (Array.isArray(txResponse.events)) {
    for (const event of txResponse.events) {
      if (event.type === eventType) {
        for (const attr of event.attributes || []) {
          const key = typeof attr.key === "string" ? attr.key : Buffer.from(attr.key, "base64").toString("utf-8");
          const val = typeof attr.value === "string" ? attr.value : Buffer.from(attr.value, "base64").toString("utf-8");
          if (key === attributeKey) {
            return val;
          }
        }
      }
    }
  }

  return undefined;
}

export const WORKSPACE_ROOT = path.resolve(SCRIPT_ROOT, "../..");
export const ENV_FILE_PATH = path.resolve(SCRIPT_ROOT, ".env");

if (fs.existsSync(ENV_FILE_PATH)) {
  dotenv.config({ path: ENV_FILE_PATH });
} else {
  dotenv.config();
}
