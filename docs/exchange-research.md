# Injective Exchange Integration Research (Seed Vault)

> **Execution Context:** Read-only architectural research for the Injective exchange integration on `injective-888` (testnet).  
> **Target Branch:** `wip/exchange-integration`  
> **Source Grounding:** Public LCD (`https://testnet.sentry.lcd.injective.network`), live chain parameters, Swagger OpenAPI specs, Injective Core (`x/exchange`, `x/wasmx`), `injective-std 1.20.0`, `@injectivelabs/sdk-ts`, and official Injective Protocol documentation.

---

## 1. Market Analysis

### 1.1 Target Market: `0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23`

- **Existence on Testnet:** **VERIFIED (Active)**
- **Ticker:** `GOLD/USDT PERP`
- **Market Type:** Perpetual (`isPerpetual: true`)
- **Status:** `Active`
- **Market ID:** `0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23`
- **Quote Denom:** `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5` (Testnet USDT via Peggy bridge)
- **Quote Decimals:** `6`
- **Oracle Base:** `GOLD`
- **Oracle Quote:** `USDT`
- **Oracle Type:** `PriceFeed`
- **Oracle Scale Factor:** `6`
- **Initial Margin Ratio (IMR):** `0.195000000000000000` (19.5% minimum margin $\implies$ maximum allowable leverage $\approx 5.128\times$)
- **Maintenance Margin Ratio (MMR):** `0.050000000000000000` (5.0% liquidation threshold)
- **Maker Fee Rate:** `-0.000100000000000000` (-0.01% maker rebate)
- **Taker Fee Rate:** `0.000500000000000000` (0.05% taker fee)
- **Relayer Fee Share Rate:** `0.400000000000000000` (40% of trading fees allocated to relayer)
- **Min Price Tick Size:** `100000.000000000000000000` (Corresponds to $100000 / 10^6 = 0.10$ USDT price tick)
- **Min Quantity Tick Size:** `0.000100000000000000` (Corresponds to $0.0001$ GOLD contracts)
- **Min Notional:** `0.000001000000000000` (Minimum notional value)
- **Current Mark Price:** `1860940000.000000000000000000` (Decoded with $10^{\text{quote\_decimals}} = 10^6 \implies \mathbf{1860.94\text{ USDT}}$)
- **Admin:** `inj17gkuet8f6pssxd8nycm3qr9d9y699rupv6397z` (Admin Permissions: `127`)

*Verification Query:*
```bash
curl -s "https://testnet.sentry.lcd.injective.network/injective/exchange/v1beta1/derivative/markets/0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23"
```

### 1.2 Alternative Active Perpetual Markets on Testnet

In the event that `GOLD/USDT PERP` lacks resting orderbook depth on testnet, the following three active perpetual markets are available:

#### Alternative 1: `BTC/USDC PERP`
- **Market ID:** `0xfd704649cf3a516c0c145ab0111717c44640d8dbe52a462ae35cadf2f6df1515`
- **Ticker:** `BTC/USDC PERP`
- **Quote Denom:** `erc20:0x0C382e685bbeeFE5d3d9C29e29E341fEE8E84C5d` (Testnet USDC, 6 decimals)
- **Type / Status:** Perpetual (`isPerpetual: true`), `Active`
- **Oracle:** Base `0x00037da...`, Quote `0x0003dc8...`, Type `ChainlinkDataStreams`
- **IMR / MMR:** `0.019231000000000000` (~52x max leverage) / `0.010000000000000000` (1%)
- **Fees (Maker/Taker):** `0.000000000000000000` / `0.000500000000000000`
- **Ticks (Price/Qty):** `1000000.000000000000000000` ($1.00) / `0.000100000000000000` ($0.0001$ BTC)
- **Min Notional:** `1000000.000000000000000000` ($1.00$ USDC)
- **Current Mark Price:** `81640779736.464706662248000000` (~$81,640.78 USDC)

#### Alternative 2: `ETH/USDC PERP`
- **Market ID:** `0x135de28700392fb1c17d40d5170a74f30055a4ad522feddafec42fbbbb780897`
- **Ticker:** `ETH/USDC PERP`
- **Quote Denom:** `erc20:0x0C382e685bbeeFE5d3d9C29e29E341fEE8E84C5d` (Testnet USDC, 6 decimals)
- **Type / Status:** Perpetual (`isPerpetual: true`), `Active`
- **Oracle:** Base `0x0003598...`, Quote `0x0003dc8...`, Type `ChainlinkDataStreams`
- **IMR / MMR:** `0.019231000000000000` (~52x max leverage) / `0.010000000000000000` (1%)
- **Fees (Maker/Taker):** `0.000000000000000000` / `0.000500000000000000`
- **Ticks (Price/Qty):** `100000.000000000000000000` ($0.10) / `0.000100000000000000` ($0.0001$ ETH)
- **Min Notional:** `1000000.000000000000000000` ($1.00$ USDC)
- **Current Mark Price:** `2397725636.696935837611000000` (~$2,397.73 USDC)

#### Alternative 3: `INJ/USDC PERP`
- **Market ID:** `0xdc70164d7120529c3cd84278c98df4151210c0447a65a2aab03459cf328de41e`
- **Ticker:** `INJ/USDC PERP`
- **Quote Denom:** `erc20:0x0C382e685bbeeFE5d3d9C29e29E341fEE8E84C5d` (Testnet USDC, 6 decimals)
- **Type / Status:** Perpetual (`isPerpetual: true`), `Active`
- **Oracle:** Base `46`, Quote `7`, Type `PythPro`
- **IMR / MMR:** `0.033333000000000000` (30x max leverage) / `0.020000000000000000` (2%)
- **Fees (Maker/Taker):** `0.000000000000000000` / `0.000500000000000000`
- **Ticks (Price/Qty):** `1000.000000000000000000` ($0.001) / `0.010000000000000000` ($0.01$ INJ)
- **Min Notional:** `1000000.000000000000000000` ($1.00$ USDC)
- **Current Mark Price:** `8250848.288074716502000000` (~$8.25 USDC)

*Verification Query:*
```bash
curl -s "https://testnet.sentry.lcd.injective.network/injective/exchange/v1beta1/derivative/markets?market_status=Active"
```


---

## 2. Margin Denom & Acquisition

### 2.1 Denom Identification & Decimals
- **Exact Quote Denom:** `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`
- **Decimals:** **6 Decimals** (NOT 18).
  - Verified on-chain bank denom metadata:
    - Base: `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5` (exponent: 0)
    - Display: `USDT` (exponent: 6)
  - Verified market configuration: `quote_decimals: 6`

*Verification Query:*
```bash
curl -s "https://testnet.sentry.lcd.injective.network/cosmos/bank/v1beta1/denoms_metadata/peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5"
```

### 2.2 Acquisition Channels for Testnet Wallets

1. **Official Injective Testnet Faucet:**
   - **Endpoint:** `https://testnet.faucet.injective.network`
   - **Distribution:** Dispenses testnet `inj` and `USDT` (`peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`).
   - Hot wallet `inj1d6qx83nhx3a3gx7e654x4su8hur5s83u84h2xc` holds $>2\times 10^9$ units of this denom.
2. **Peggy Bridge (Ethereum Sepolia $\to$ Injective Testnet):**
   - **Bridge Hub:** `https://testnet.hub.injective.network/bridge`
   - **Underlying ERC-20:** `0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5` on Ethereum Sepolia testnet.
3. **On-Chain Spot Swap (INJ $\to$ USDT):**
   - **Active Testnet Spot Market:** `INJ/USDT`
   - **Market ID:** `0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe`
   - **Base / Quote:** `inj` / `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`

---

## 3. Scaling & Wire Format Encoding

### 3.1 Protobuf Types & Gogoproto Definitions
In `injective-std 1.20.0` and the upstream proto specification (`injective/exchange/v1beta1/exchange.proto`):

```protobuf
message OrderInfo {
  string subaccount_id = 1;
  string fee_recipient = 2;
  string price = 3 [
    (gogoproto.customtype) = "cosmossdk.io/math.LegacyDec",
    (gogoproto.nullable) = false
  ];
  string quantity = 4 [
    (gogoproto.customtype) = "cosmossdk.io/math.LegacyDec",
    (gogoproto.nullable) = false
  ];
  string cid = 5;
}

message DerivativeOrder {
  string market_id = 1;
  OrderInfo order_info = 2 [(gogoproto.nullable) = false];
  OrderType order_type = 3;
  string margin = 4 [
    (gogoproto.customtype) = "cosmossdk.io/math.LegacyDec",
    (gogoproto.nullable) = false
  ];
  string trigger_price = 5 [
    (gogoproto.customtype) = "cosmossdk.io/math.LegacyDec",
    (gogoproto.nullable) = true
  ];
}
```

### 3.2 "Chain Format" vs Human Decimals
1. **Chain Format Mechanics (`x/exchange` Go Core):**
   - **Price:** Denominated in quote asset per 1 contract unit.
     $$\text{Price}_{\text{chain}} = \text{Price}_{\text{human}} \times 10^{\text{quote\_decimals}}$$
     For `GOLD/USDT PERP` with $\text{quote\_decimals} = 6$, a human price of $\$1,865.00$ converts to $\mathbf{1865000000}$.
   - **Quantity:** For derivatives, 1 contract corresponds to 1 unit of base asset (base decimals = 0).
     $$\text{Quantity}_{\text{chain}} = \text{Quantity}_{\text{human}}$$
     A human order of $1.0$ GOLD contract is $\mathbf{1.0}$ (or integer $1$).
   - **Margin:** Denominated in the quote asset.
     $$\text{Margin}_{\text{chain}} = \text{Margin}_{\text{human}} \times 10^{\text{quote\_decimals}}$$
     A human margin of $\$932.50$ USDT converts to $\mathbf{932500000}$.
   - **Reduce-Only Close Margin:** MUST be explicitly set to string `"0"`.

2. **Wire Representation (`cosmossdk.io/math.LegacyDec`):**
   Because fields in `OrderInfo` and `DerivativeOrder` are unmarshaled via Cosmos SDK's `LegacyNewDecFromStr(string)`, the string value may be provided either:
   - As an integer string in chain units: `"1865000000"` (which `LegacyDec` decodes as $1865000000 \times 10^{18}$ internal integer).
   - Or with decimal representation: `"1865000000.000000000000000000"`.
   - **CRITICAL PITFALL:** Sending human numbers (e.g. `price = "1865"`) causes the chain to interpret the price as $1865$ micro-USDT ($\$0.001865$), which is immediately rejected because it violates `min_price_tick_size` ($100000$). Sending plain unscaled integers from user input guarantees immediate on-chain failure.

### 3.3 Worked Numerical Example

**Target Market:** `GOLD/USDT PERP` (`quote_decimals: 6`, `IMR: 0.195`, `min_price_tick: 100000`, `min_qty_tick: 0.0001`)

**Human Target Order:**
- Action: BUY Long
- Quantity $Q$: `1.0` GOLD contract
- Execution / Worst Acceptable Price $P$: `$1,865.00` USDT (slippage buffer above current mark price $1860.94$)
- Leverage: $2\times$
- Margin $M$: Notional / Leverage $= (1.0 \times 1865.00) / 2 = \mathbf{\$932.50\text{ USDT}}$ (exceeds minimum required IMR of $1.0 \times 1865 \times 0.195 = \$363.675$)

**Values Sent by CosmWasm Contract in `MsgCreateDerivativeMarketOrder`:**

| Parameter | Human Value | Exact Chain String Required |
|---|---|---|
| `order_info.subaccount_id` | Contract Subaccount | `"0xc959f80872386393899ddae27d62fcca77e3a8c8000000000000000000000000"` |
| `order_info.fee_recipient` | Contract Address | `"inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k"` |
| `order_info.price` | $\$1,865.00$ | `"1865000000"` (or `"1865000000.000000000000000000"`) |
| `order_info.quantity` | $1.0$ contract | `"1"` (or `"1.000000000000000000"`) |
| `order_info.cid` | Order Identifier | `""` |
| `order.order_type` | Buy (or Buy Atomic) | `1` (`OrderType::Buy`) or `9` (`OrderType::BuyAtomic`) |
| `order.margin` | $\$932.50$ | `"932500000"` (or `"932500000.000000000000000000"`) |
| `order.trigger_price` | None / Zero | `""` (or `"0"`) |

**Companion `MsgDeposit` Amount:**
- Amount: $M + \text{estimated taker fee}$
- Taker Fee ($0.05\%$ of $\$1,865.00$ notional) $= \$0.9325$ USDT ($932500$ micro-units)
- Total Deposit Required: $932500000 + 932500 = \mathbf{933432500}$
- Denom: `"peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5"`


---

## 4. Market Order Semantics

### 4.1 OrderInfo.price Semantics
In Injective, a market order is NOT an unrestricted order with unlimited slippage. `OrderInfo.price` defines the **worst acceptable execution price**:
- **BUY Market Order:** `price` is the **maximum (ceiling) price** the trader is willing to pay.
- **SELL Market Order:** `price` is the **minimum (floor) price** the trader is willing to accept.

### 4.2 Fill Behavior & Failure Conditions
- **Immediate Rejection in `DeliverTx`:**
  During message execution, Injective validates that the market order's worst price crosses the top of the opposing orderbook:
  - BUY order: `order.price >= best_ask`
  - SELL order: `order.price <= best_bid`
  If the orderbook lacks opposing resting orders or the worst price cannot cross the spread, the transaction reverts immediately with **Error 25 (`Orderbook liquidity cannot satisfy current worst price`)** or **Error 24 (`no liquidity on the orderbook!`)**.
- **Execution in `EndBlocker` (Frequent Batch Auction):**
  Standard market orders (`OrderType::Buy = 1`, `OrderType::Sell = 2`) do not execute immediately; they are placed into transient storage and matched in the EndBlocker against the resting orderbook at a uniform clearing price.
  - If the available orderbook depth up to `price` is insufficient to satisfy the full quantity, the order **partially fills**, and the remaining unfilled quantity is **cancelled and refunded**.
  - **A market order is NOT guaranteed to fill fully or not at all.** Injective derivative market orders operate with Immediate-Or-Cancel (IOC) semantics: fill whatever depth is available up to the worst price, cancel remainder.

### 4.3 Reduce-Only Close Semantics
- **Encoding:** A reduce-only close is uniquely signaled by setting **`margin: "0"`**.
- **Direction:** Must be the exact opposite side of the open position:
  - Closing an open Long: `order_type = OrderType::Sell` (or `SellAtomic`) with `margin = "0"`.
  - Closing an open Short: `order_type = OrderType::Buy` (or `BuyAtomic`) with `margin = "0"`.
- **Validation Rules:**
  1. A position for `subaccount_id` on `market_id` must exist.
  2. Order direction must strictly oppose existing position (otherwise rejects with `Error 29: Position direction does not oppose the reduce-only order`).
  3. Order quantity must satisfy: $\text{position.quantity} - \text{aggregate\_reduce\_only} - \text{order.quantity} \ge 0$.
  4. Margin hold deducted from subaccount deposit is strictly **0**.

---

## 5. Accounting Facts & On-Chain State Queries

### 5.1 Fee Deduction & Subaccount Balances
1. **Taker Fee Deduction:**
   - Taker fees are charged against the subaccount deposit in `quote_denom`.
   - When an order is placed: $\text{margin} + \text{estimated\_taker\_fee}$ is placed on `margin_hold`, reducing `available_balance` while `total_balance` remains constant.
   - When the order fills: Taker fee is permanently deducted ($\text{total\_balance}$ drops by the fee). Active margin is transferred into the position record (`Position.margin`).
2. **Subaccount Balances Lifecycle:**
   - **Before Open:** $\text{total\_balance} = D$, $\text{available\_balance} = D$.
   - **After Open (Position Active):**
     $$\text{available\_balance} = D - \text{margin} - \text{taker\_fee}$$
     $$\text{total\_balance} = D - \text{margin} - \text{taker\_fee}$$
     (Note: Margin held in an active position is tracked inside the position state, not in the subaccount deposit).
   - **After Close (Position Exited):**
     When the closing reduce-only order matches, payout is computed:
     $$\text{Payout} = \text{Initial Margin} + \text{Realized PnL} - \text{Closing Taker Fee}$$
     The payout is credited directly back to the subaccount deposit:
     $$\text{available\_balance}_{\text{new}} = \text{available\_balance}_{\text{prior}} + \text{Payout}$$
     $$\text{total\_balance}_{\text{new}} = \text{total\_balance}_{\text{prior}} + \text{Payout}$$

### 5.2 Required Queries & Response Formats

#### 1. Subaccount Deposit Query
- **Rust Query:**
  ```rust
  use injective_std::types::injective::exchange::v1beta1::ExchangeQuerier;
  let querier = ExchangeQuerier::new(&deps.querier);
  let res = querier.subaccount_deposit(subaccount_id, quote_denom)?;
  ```
- **Protobuf Type:** `injective.exchange.v1beta1.QuerySubaccountDepositRequest`
- **gRPC Path:** `/injective.exchange.v1beta1.Query/SubaccountDeposit`
- **REST LCD:** `/injective/exchange/v1beta1/exchange/subaccountDeposit?subaccount_id={id}&denom={denom}`
- **Response Fields (`QuerySubaccountDepositResponse`):**
  ```rust
  pub struct QuerySubaccountDepositResponse {
      pub deposits: Option<Deposit>,
  }
  pub struct Deposit {
      pub available_balance: String, // e.g. "932500000.000000000000000000"
      pub total_balance: String,     // e.g. "932500000.000000000000000000"
  }
  ```

#### 2. Subaccount Position Query
- **Rust Query:**
  ```rust
  let res = querier.subaccount_position_in_market(subaccount_id, market_id)?;
  ```
- **Protobuf Type:** `injective.exchange.v1beta1.QuerySubaccountPositionInMarketRequest`
- **gRPC Path:** `/injective.exchange.v1beta1.Query/SubaccountPositionInMarket`
- **REST LCD:** `/injective/exchange/v1beta1/positions/{subaccount_id}/{market_id}`
- **Response Fields (`QuerySubaccountPositionInMarketResponse`):**
  ```rust
  pub struct QuerySubaccountPositionInMarketResponse {
      pub state: Option<Position>, // None if no position exists
  }
  pub struct Position {
      pub is_long: bool,
      pub quantity: String,                 // In chain format (18 decimals string)
      pub entry_price: String,              // In chain format (scaled by quote decimals)
      pub margin: String,                   // In chain format (scaled by quote decimals)
      pub cumulative_funding_entry: String, // Funding index tracking
  }
  ```


---

## 6. Contract-Specific Checks & Execution Invariants

### 6.1 Contract as Sender & Subaccount Derivation
- **Finding:** **VERIFIED (Fully Supported)**
- **Evidence:**
  - A CosmWasm contract can be the `sender` of `MsgDeposit`, `MsgCreateDerivativeMarketOrder`, and `MsgWithdraw`.
  - In CosmWasm, `env.contract.address` is the signer of messages emitted by the contract.
  - The default subaccount ID is derived deterministically:
    $$\text{SubaccountID} = \text{0x} + \text{hex}(\text{bech32\_decode}(\text{contract\_addr})) + \text{"0"}\times 24$$
    For contract `inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k`, the default subaccount ID is `0xc959f80872386393899ddae27d62fcca77e3a8c8000000000000000000000000`.
  - Upstream `CheckValidSubaccountIDOrNonce` checks that `subaccountAddress.Bytes() == sender.Bytes()`. Because the first 20 bytes equal the contract's decoded address, ownership validation succeeds.

### 6.2 Stargate / Any Messages & Query Whitelist
- **Finding:** **VERIFIED**
- **Evidence:**
  - `injective-std 1.20.0` messages implement `Into<CosmosMsg<T>>` via `CosmosMsg::Stargate { type_url, value }` using `#[derive(CosmwasmExt)]`.
  - Official Injective documentation (`developers-cosmwasm/cosmwasm-any.mdx`) confirms that protobuf messages (under both `CosmosMsg::Stargate` and `CosmosMsg::Any`) targeting `injective.exchange.v1beta1` are supported.
  - Stargate queries for `/injective.exchange.v1beta1.Query/DerivativeMarket`, `/injective.exchange.v1beta1.Query/SubaccountDeposit`, and `/injective.exchange.v1beta1.Query/SubaccountPositionInMarket` are routed directly to the exchange keeper.

### 6.3 On-Chain Mark Price Query
- **Finding:** **VERIFIED**
- **Evidence:**
  - A contract can read the on-chain mark price directly through `ExchangeQuerier::new(&deps.querier).derivative_market(market_id)`.
  - **Path:** `/injective.exchange.v1beta1.Query/DerivativeMarket`
  - **Returned Field:** `res.market.unwrap().mark_price`
  - The returned `mark_price` string is formatted in chain units ($10^{\text{quote\_decimals}}$ with 18 decimal places, e.g. `"1860940000.000000000000000000"` for $\$1,860.94$).

### 6.4 Order Execution Timing (Synchronous vs EndBlocker)
- **Finding:** **CRITICAL ARCHITECTURAL SPLIT — FBA vs ATOMIC**
- **Evidence:**
  1. **Standard Market Orders (`OrderType::Buy = 1`, `OrderType::Sell = 2`):**
     - Executed **asynchronously in the `EndBlocker`** via Frequent Batch Auction (FBA).
     - During `DeliverTx` (when the contract executes and enters `reply`), the order is only placed into the `transient` store.
     - **Implication:** In `reply`, the order has NOT matched, the position has NOT updated, and released margin has NOT been credited to the subaccount deposit!
  2. **Atomic Market Orders (`OrderType::BuyAtomic = 9`, `OrderType::SellAtomic = 10`):**
     - Live testnet parameter verified via LCD:
       `"atomic_market_order_access_level": "SmartContractsOnly"`
       `"derivative_atomic_market_order_fee_multiplier": "2.000000000000000000"`
     - Atomic orders execute **synchronously inside `DeliverTx`**, immediately matching against the resting orderbook and returning `DerivativeMarketOrderResults` in the reply data.
     - Contracts pay a $2\times$ fee multiplier for atomic synchronous execution.

### 6.5 Liquidation Behavior & Detection
- **Finding:** **VERIFIED**
- **Evidence:**
  - If a position breaches the Maintenance Margin Ratio (MMR), anyone can call `MsgLiquidatePosition`.
  - The exchange automatically executes an opposite reduce-only market order with worst price set to infinity/0.
  - Position margin is forfeited to liquidators and the insurance fund; the contract receives zero payout.
  - **Detection:**
    The contract can detect that its position disappeared by querying `subaccount_position_in_market`:
    If liquidated, `QuerySubaccountPositionInMarketResponse.state` returns `None` (`null`), while the contract's internal storage (`POSITIONS`) still has a record.


---

## 7. Implications for `contracts/seed-vault`

An audit of `contracts/seed-vault/src/contract.rs` (commit `0275689`) reveals critical structural and accounting defects that will cause live on-chain operations to fail immediately:

### Flaw 1: Margin Denom Mismatch (`inj` vs `quote_denom`)
- **Current WIP Code:**
  `deposit_treasury` and `open_position` use `config.treasury_denom` (configured as `"inj"` in testnet deployment record line 23).
  `deposit_msg` deposits `config.treasury_denom` (`"inj"`) into the subaccount.
- **Root Cause & Impact:**
  `GOLD/USDT PERP` requires margin in `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`.
  The contract subaccount receives INJ, but the exchange order attempts to encumber USDT.
- **Result:** Reverts immediately with **`Error 6: subaccount has insufficient deposits`**.

### Flaw 2: Integer Scaling Violation (Human Stringification)
- **Current WIP Code:**
  `parse_units` parses input as plain integers (`raw.parse::<Uint128>()`).
  `OrderInfo` is constructed with `price: price_u128.to_string()`, `quantity: qty_u128.to_string()`, and `margin: margin_u128.to_string()`.
- **Root Cause & Impact:**
  In `smoke.ts`, the developer passed `price: "2"`, `margin: "1000"`, `quantity: "500"`.
  For `GOLD/USDT PERP`, `min_price_tick_size` is `100000` ($0.1$ USDT). Price `"2"` is $\$0.000002$, which breaches min price tick size.
  Similarly, quantity for derivatives represents contracts (500 contracts $= 500$ gold ounces $\approx \$930,000$ notional).
- **Result:** Reverts immediately with **`Error 16: invalid price`** or **`Error 34: invalid minimum price tick size`**.

### Flaw 3: Close Order Uses Entry Price as Worst Price
- **Current WIP Code (lines 736-747):**
  ```rust
  price: pos.price.to_string(), // entry price passed as close order price
  order_type: if pos.is_buy { OrderType::Sell as i32 } else { OrderType::Buy as i32 },
  margin: "0".to_string(),
  ```
- **Root Cause & Impact:**
  For a market order, `price` is the **worst acceptable price**.
  - When closing a Long, the contract issues a SELL order with `price = entry_price`. If the market price dropped by even 1 cent below entry, the order rejects because the best bid is lower than the worst acceptable price.
  - When closing a Short, the contract issues a BUY order with `price = entry_price`. If the market price rose by 1 cent above entry, the order rejects.
- **Result:** Any position closed in a loss or after price fluctuation fails with **`Error 25: Orderbook liquidity cannot satisfy current worst price`**.

### Flaw 4: `MsgWithdraw` Emitted in Same Message with Fixed `pos.margin`
- **Current WIP Code (lines 751-765):**
  ```rust
  let withdraw_msg: CosmosMsg<Empty> = MsgWithdraw {
      sender: env.contract.address.to_string(),
      subaccount_id,
      amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
          denom: config.treasury_denom,
          amount: pos.margin.to_string(),
      }),
  }.into();
  ```
- **Root Cause & Impact:**
  1. **Timing Disaster:** The close order is placed with `OrderType::Sell` (FBA mode). FBA orders execute in `EndBlocker`. During the transaction execution, the position is still open and margin is locked in the exchange module. `available_balance` in the subaccount is 0. `MsgWithdraw` executes immediately and reverts because available balance is insufficient.
  2. **Accounting Disaster:** Margin released upon closing equals $\text{Initial Margin} \pm \text{PnL} - \text{Taker Fees}$. Attempting to withdraw exact `pos.margin` assumes $0$ trading fee and $0$ PnL. If either is non-zero, the withdrawal amount will not match the available subaccount balance.
- **Result:** Reverts immediately with insufficient withdrawal balance, aborting the entire closing transaction.

### Flaw 5: Blind Reply Handler & Lack of Fill Verification
- **Current WIP Code (lines 779-806):**
  ```rust
  REPLY_OPEN_POSITION => match msg.result {
      SubMsgResult::Ok(_) => Ok(Response::new().add_attribute("action", "reply_open_position")),
      ...
  ```
- **Root Cause & Impact:**
  The reply handler ignores `msg.result` payload. For standard FBA orders, `reply` occurs before the order has even executed in EndBlocker. If the order fails or cancels in EndBlocker, the vault contract retains internal state recording an active position that does not exist on-chain.
  For atomic orders, the reply payload contains `MsgCreateDerivativeMarketOrderResponse` with execution details (`results.payout`, `results.price`, `results.fee`, `results.position_delta`) which the current contract completely ignores.


---

## 8. Summary Table of Research Findings

| # | Question / Topic | Answer / Architectural Value | Verification Status | Exact Source / Reference |
|---|---|---|---|---|
| 1.1 | Market `0x8f00...5d23` exists? | Exists, Active, Perpetual (`GOLD/USDT PERP`) | **VERIFIED** | Public LCD `/injective/exchange/v1beta1/derivative/markets/0x8f00...` |
| 1.2 | Market parameters | IMR: 0.195, MMR: 0.05, Maker: -0.01%, Taker: 0.05%, Tick: 100000 ($0.1), Qty Tick: 0.0001, Mark: 1860940000 ($1860.94) | **VERIFIED** | Public LCD `/injective/exchange/v1beta1/derivative/markets/0x8f00...` |
| 1.3 | 3 Alternative active perps | `BTC/USDC PERP` (`0xfd70...`), `ETH/USDC PERP` (`0x135d...`), `INJ/USDC PERP` (`0xdc70...`) | **VERIFIED** | Public LCD `/injective/exchange/v1beta1/derivative/markets?market_status=Active` |
| 2.1 | Quote denom & decimals | `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`, **6 decimals** | **VERIFIED** | Bank denom metadata `/cosmos/bank/v1beta1/denoms_metadata/peggy0x87...` |
| 2.2 | Denom acquisition source | Official Faucet (`https://testnet.faucet.injective.network`), Peggy Bridge, Spot Market `INJ/USDT` (`0x0611...`) | **VERIFIED** | Testnet faucet hot wallet `inj1d6qx...` balances & spot markets LCD |
| 3.1 | Encoding & Scaling | Price: $P \times 10^6$ (`1865000000`), Qty: $Q$ contracts (`1`), Margin: $M \times 10^6$ (`932500000`). String representation of `math.LegacyDec`. | **VERIFIED** | `all_protos/injective/exchange/v1beta1/exchange.proto`, `sdk-go/chain/exchange/types/market.go` |
| 3.2 | Worked buy order example | Buy 1.0 GOLD at $\$1,865.00$ with $\$932.50$ margin $\implies$ price `"1865000000"`, qty `"1"`, margin `"932500000"` | **VERIFIED** | Calculations matching `quote_decimals: 6` and `x/exchange` validation rules |
| 4.1 | OrderInfo.price meaning | Worst acceptable execution price (Buy ceiling, Sell floor) | **VERIFIED** | Official Injective Docs `defi/trading/order-types.mdx` |
| 4.2 | Order fill failure behavior | Reverts in `DeliverTx` if worst price doesn't cross spread; in EndBlocker, fills available depth and cancels unfilled remainder | **VERIFIED** | `developers-native/injective/exchange/04_state_transitions.mdx` & `08_end_block.mdx` |
| 4.3 | Reduce-only close semantics | Opposite `order_type`, `margin: "0"`, quantity $\le$ open position quantity | **VERIFIED** | `developers-native/injective/exchange/04_state_transitions.mdx` |
| 5.1 | Fee deduction | Deducted from subaccount deposit upon fill ($0.05\%$ taker fee) | **VERIFIED** | `developers-native/injective/exchange/00_derivative_market_concepts.mdx` |
| 5.2 | Accounting lifecycle | Margin held outside subaccount during trade; payout $(\text{margin} + \text{PnL} - \text{fee})$ returned to subaccount upon close | **VERIFIED** | `developers-native/injective/exchange/00_derivative_market_concepts.mdx` |
| 5.3 | Position & Deposit queries | `QuerySubaccountDepositRequest` (`available_balance`, `total_balance`), `QuerySubaccountPositionInMarketRequest` (`state: Option<Position>`) | **VERIFIED** | `injective-std-1.20.0/src/types/injective/exchange/v1beta1.rs` & Public LCD |
| 6.a | Contract as sender & subaccount | Fully supported. Subaccount deterministically derived from contract 20-byte address + 24 zeroes | **VERIFIED** | `sdk-go/chain/exchange/types/common_utils.go` & `CheckValidSubaccountIDOrNonce` |
| 6.b | Stargate/Any support & whitelist | Supported via `CosmosMsg::Stargate` / `CosmosMsg::Any`. Querier queries supported via `ExchangeQuerier` | **VERIFIED** | `injective-docs/developers-cosmwasm/cosmwasm-any.mdx`, `injective-std-1.20.0` |
| 6.c | Read mark price on-chain | Yes, query `ExchangeQuerier::derivative_market(market_id)` returns `FullDerivativeMarket.mark_price` | **VERIFIED** | Protobuf `FullDerivativeMarket` & LCD `/injective/exchange/v1beta1/derivative/markets/...` |
| 6.d | Execution timing | FBA orders (`OrderType::Buy/Sell`) run in **EndBlocker**. Atomic orders (`BuyAtomic/SellAtomic`) run **synchronously in DeliverTx** (SmartContractsOnly) | **VERIFIED** | LCD `/injective/exchange/v1beta1/exchangeParams` & `08_end_block.mdx` |
| 6.e | Liquidation behavior & detection | Position closed and deleted by liquidator bot; margin forfeited; contract detects liquidation when `state == None` | **VERIFIED** | `developers-native/injective/exchange/00_derivative_market_concepts.mdx` & LCD test |

