# Seed Vault Injective Testnet Smoke-Test Toolkit

A standalone TypeScript deployment, configuration, and end-to-end smoke-testing toolkit for the **Seed Vault** CosmWasm smart contract on **Injective Testnet** (`injective-888`) using `@injectivelabs/sdk-ts`.

---

## 1. Verified Injective Testnet Constants & Official Sources

| Parameter | Value | Source / Query Reference |
|---|---|---|
| **Chain ID** | `injective-888` | [Injective Network Info](https://docs.injective.network/developers/network-information) |
| **RPC Endpoint** | `https://testnet.sentry.tm.injective.network:443` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **REST / LCD Endpoint** | `https://testnet.sentry.lcd.injective.network:443` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **gRPC Endpoint** | `https://testnet.sentry.chain.grpc-web.injective.network:443` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **Indexer Endpoint** | `https://testnet.sentry.exchange.grpc-web.injective.network:443` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **Testnet Explorer** | `https://testnet.explorer.injective.network` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **Testnet Faucet** | `https://testnet.faucet.injective.network/` | [Injective Public Endpoints](https://docs.injective.network/infra/public-endpoints) |
| **Gas Price (Default)** | `500000000inj` (0.5 Gwei) | [Transaction Fees](https://docs.injective.network/defi/transaction-fees) |
| **Denom Creation Fee** | `1000000000000000000inj` (1 INJ) | On-chain query `/injective/tokenfactory/v1beta1/params` |
| **CosmWasm Upload Perm** | `Everybody` (Permissionless on testnet) | On-chain query `/cosmwasm/wasm/v1/codes/params` |

---

## 2. Prerequisites

1. **Node.js**: v20+ or v22+
2. **Compiled Contract Bytecode**:
   - The compiled contract binary must exist at:
     `contracts/seed-vault/artifacts/seed_vault.wasm`
   - Target verified SHA256 checksum:
     `10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60`
   - If missing, download the `seed-vault-wasm` artifact from GitHub Actions and place it in the artifacts folder.
3. **Funded Testnet Wallet**:
   - Obtain testnet INJ from the [Official Injective Faucet](https://testnet.faucet.injective.network/).
   - Ensure the wallet has at least ~2 INJ (1 INJ required for TokenFactory denom creation fee + gas reserves).

---

## 3. Configuration Setup

Copy `.env.example` to `.env`:

```bash
cd scripts/testnet
cp .env.example .env
```

Edit `.env` and set:
- `MNEMONIC`: Your testnet wallet mnemonic phrase (12 or 24 words).
- `MARKET_IDS`: Comma-separated derivative market ID(s) to whitelist (e.g., INJ/USDT PERP testnet market ID).
- `INJECTIVE_NETWORK`: Must remain `testnet`.


---

## 4. Execution Workflow

All scripts are executed sequentially from the `scripts/testnet` directory (or via `npm --prefix scripts/testnet run <script>`):

### Step 1: Typecheck
Validate TypeScript compilation across all scripts:
```bash
npm run typecheck
```

### Step 2: Store Bytecode (`npm run store`)
- Reads `contracts/seed-vault/artifacts/seed_vault.wasm`.
- Checks and verifies SHA256 matches `10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60`.
- Simulates gas and broadcasts `MsgStoreCode`.
- Queries on-chain code info to confirm the stored data hash matches local SHA256.
- Saves `codeId`, `codeHash`, and `storeTxHash` to `scripts/testnet/state.json`.

### Step 3: Instantiate Vault (`npm run instantiate`)
- Verifies `codeId` exists in `state.json`.
- Creates a test SEED token via `MsgCreateDenom` (`factory/{deployer}/seed`).
- Mints test SEED tokens to the deployer wallet via `MsgMint`.
- Instantiates the Seed Vault with **explicit** risk parameters:
  - `owner`: deployer address
  - `seed_denom`: TokenFactory SEED denom
  - `treasury_denom`: `inj`
  - `trader`: deployer address
  - `allowed_markets`: parsed from `MARKET_IDS`
  - `max_leverage`: `"2"`
  - `max_exposure_bps`: `3000` (30%)
  - `max_total_margin_bps`: `3000` (30%)
  - `max_position_margin`: `1000000000000000000` (1 INJ)
  - `max_position_size`: `1000000000000000000` (1 SEED)
  - `cooldown_seconds`: `300`
- Extracts contract address and writes to `state.json`.

### Step 4: Smoke Test (`npm run smoke`)
Executes an end-to-end verification sequence using tiny amounts:
1. **Stake**: Stakes 1,000 base units of SEED; asserts `user_stake` and `total_staked`.
2. **Unstake**: Unstakes 400 base units of SEED; asserts updated `user_stake`.
3. **DepositTreasury**: Deposits 10,000 base units of INJ; asserts `treasury_state.balance`.
4. **Pause Enforcement**: Pauses contract; verifies Stake is blocked while Unstake remains permitted.
5. **Unpause**: Unpauses contract; verifies operational state restored.
6. **Position Lifecycle**: Executes `open_position` on an allowed market (asserts reserved margin and deducted free balance), then executes `close_position` (asserts margin returned to free balance).
7. **Market Whitelist Rejection**: Attempts to open a position on an unapproved market; asserts rejection.
8. **Owner Round Trip**: Generates ephemeral keypair, funds with 0.02 INJ gas, proposes ephemeral key as new owner, accepts ownership, then proposes and accepts original deployer back.
9. **Summary Report**: Prints a clean PASS/FAIL verification table.

---

## 5. Security & Safety Invariants

- **Strict Testnet Guard**: All scripts assert that `INJECTIVE_NETWORK === "testnet"` and the target chain is `injective-888`. Execution will immediately halt if any mainnet environment or endpoint is detected.
- **Zero Secret Exposure**: Mnemonics and derived private keys are never printed, logged, or recorded to state files.
- **Fail-Closed Execution**: If any transaction simulation, broadcast, or query fails, the script outputs the exact raw chain error and halts without applying blind retries or mutating contract state.
- **Git Protection**: `scripts/testnet/state.json`, `.env`, and `node_modules` are gitignored.
