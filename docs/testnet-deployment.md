# Injective Testnet Deployment & Verification Record

> **Notice:** Testnet only, not audited, do not deposit real funds.

This document records the official deployment of the **Seed Vault** contract on the Injective testnet (`injective-888`), including deployment parameters, cryptographic checksums, smoke test verification, and reproducible instructions to independently verify the contract bytecode and state.

---

## 1. Deployment Parameters

| Parameter | On-Chain Value / Record |
|---|---|
| **Network** | Injective Testnet |
| **Chain ID** | `injective-888` |
| **Code ID** | `40036` |
| **Store Tx Hash** | `456E5868B8F8C79D007B06B00DB0327202B4050A51FEE95CB2C71EAC209FD19A` |
| **Contract Address** | `inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k` |
| **Instantiate Tx Hash** | `C7892B6B1E635B89CCA8CD95646A3034BFFA0A11DCFD8D2D0D345D8E73BC1B69` |
| **WASM Code Hash (SHA-256)** | `10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60` |
| **Artifact** | `artifacts/seed_vault.wasm` (~410 KB) |
| **Compiler Toolchain (MSRV)** | Rust `1.86` |
| **Deterministic Optimizer** | `cosmwasm/optimizer:0.17.0` |
| **Treasury Denom** | `inj` (testnet shortcut) |
| **Max Leverage** | `2` (2x ceiling) |
| **Max Exposure BPS** | `3000` (30% of total treasury) |
| **Max Position Margin** | `1000000000000000000` (1e18) |
| **Max Position Size** | `1000000000000000000` (1e18) |
| **Cooldown Seconds** | `300` seconds (5 minutes) |
| **Allowed Markets** | `["0x0511ddc4e6586f3bfe1acb2dd905f8b8a82c97e1edaef65470f6a2cdbb493b60"]` |
| **Role Key Distribution** | Owner = Trader = Admin (Single testnet deployer wallet) |

---

## 2. Testnet Smoke Test Verification (8/8 PASS)

The testnet smoke test suite (`scripts/testnet/src/smoke.ts`) validated all 8 testnet operational flows:

| # | Operational Flow | Expected Behavior | Status |
|---|---|---|---|
| 1 | `Stake` | Staked `$SEED` balance recorded 1:1 in vault storage | **PASS** |
| 2 | `Unstake` | Staked balance deducted and `$SEED` returned to wallet | **PASS** |
| 3 | `DepositTreasury` | Treasury margin balance increased in contract storage | **PASS** |
| 4 | `Pause` Circuit Breaker | Halts new stakes while keeping withdrawals open | **PASS** |
| 5 | `Unpause` | Staking and trade management resumed successfully | **PASS** |
| 6 | `OpenPosition` / `ClosePosition` | Position opened under risk limits and closed at 0 PnL | **PASS** |
| 7 | Risk Parameter Guard | Non-whitelisted market ID rejected immediately | **PASS** |
| 8 | Governance Transfer | Two-step ownership transfer (`ProposeOwner` / `AcceptOwner`) round trip | **PASS** |


---

## 3. How to Verify Yourself

### A. Reproduce WASM Build & Verify Checksum

You can run the deterministic build locally via Docker using `cosmwasm/optimizer:0.17.0` or trigger the GitHub Actions workflow `.github/workflows/optimize.yml`.

#### Bash:
```bash
# 1. Run deterministic docker optimizer from the workspace root
docker run --rm -v "$(pwd)/contracts/seed-vault":/code \
  --mount type=volume,source="seed_vault_cache",target=/target \
  --mount type=volume,source="registry_cache",target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.17.0

# 2. Compute sha256 checksum
sha256sum contracts/seed-vault/artifacts/seed_vault.wasm

# 3. Compare with expected hash:
# 10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60
```

#### PowerShell:
```powershell
# 1. Run deterministic docker optimizer from workspace root
docker run --rm -v "${PWD}/contracts/seed-vault:/code" `
  --mount type=volume,source="seed_vault_cache",target=/target `
  --mount type=volume,source="registry_cache",target=/usr/local/cargo/registry `
  cosmwasm/optimizer:0.17.0

# 2. Compute sha256 checksum
(Get-FileHash -Path "contracts/seed-vault/artifacts/seed_vault.wasm" -Algorithm SHA256).Hash.ToLower()

# 3. Expected:
# 10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60
```

---

### B. Query On-Chain Code Hash (Public LCD)

Query the Injective testnet REST endpoint to retrieve the stored WASM metadata for Code ID `40036`.

#### Bash:
```bash
# Query code info from public LCD endpoint
curl -s "https://testnet.sentry.lcd.injective.network/cosmwasm/wasm/v1/code/40036" | jq -r '.code_info.data_hash'

# Compare output with expected hash:
# 10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60
```

#### PowerShell:
```powershell
# Query code info from public LCD endpoint
$resp = Invoke-RestMethod -Uri "https://testnet.sentry.lcd.injective.network/cosmwasm/wasm/v1/code/40036"
$resp.code_info.data_hash.ToLower()

# Compare output with expected hash:
# 10d74510d09dcded3251ca48a4d6e779df30f8a1da57392acdf13aef22a9ac60
```

---

### C. Query Contract Configuration & Risk Limits

Query the live contract state on `injective-888` using the base64-encoded query payloads.

#### Bash:
```bash
CONTRACT="inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k"
LCD="https://testnet.sentry.lcd.injective.network"

# 1. Query Config ({"config":{}} = eyJjb25maWciOnt9fQ==)
curl -s "$LCD/cosmwasm/wasm/v1/contract/$CONTRACT/smart/eyJjb25maWciOnt9fQ==" | jq .

# 2. Query RiskConfig ({"risk_config":{}} = eyJyaXNrX2NvbmZpZyI6e319)
curl -s "$LCD/cosmwasm/wasm/v1/contract/$CONTRACT/smart/eyJyaXNrX2NvbmZpZyI6e319" | jq .
```

#### PowerShell:
```powershell
$Contract = "inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k"
$LCD = "https://testnet.sentry.lcd.injective.network"

# 1. Query Config ({"config":{}} in base64: eyJjb25maWciOnt9fQ==)
$cfg = Invoke-RestMethod -Uri "$LCD/cosmwasm/wasm/v1/contract/$Contract/smart/eyJjb25maWciOnt9fQ=="
$cfg.data

# 2. Query RiskConfig ({"risk_config":{}} in base64: eyJyaXNrX2NvbmZpZyI6e319)
$risk = Invoke-RestMethod -Uri "$LCD/cosmwasm/wasm/v1/contract/$Contract/smart/eyJyaXNrX2NvbmZpZyI6e319"
$risk.data
```

---

## 4. Known Limitations

1. **Admin Migration Authority**: The contract admin address can migrate the contract bytecode (`MsgMigrateContract`). This migration capability is retained by the deployer key during testnet development.
2. **Key Consolidation**: On this testnet deployment, the Contract Admin, Owner, and Trader roles are assigned to the same testnet wallet address. Production deployments will segregate these roles across multisig and keeper keys.
3. **Treasury Denom Shortcut**: The treasury denom is configured as `inj` rather than a pegged stablecoin (such as USDT) as a testnet shortcut to simplify funding and smoke-test execution.
4. **Exchange Integration Pending**: Derivative order placement, exchange subaccount margin transfers, and chain-derived PnL settlement are not implemented in this build. `OpenPosition` and `ClosePosition` execute internal accounting only, and position closes record 0 realized PnL locally.
5. **No Third-Party Audit**: The contract has undergone internal developer unit and invariant testing only. No independent external security audit has been performed.