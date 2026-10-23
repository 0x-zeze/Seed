# Exchange Liquidity Research (read-only)

Read-only public testnet queries. Nothing was broadcast. No mnemonic or private key was used.

- LCD: `https://testnet.sentry.lcd.injective.network`
- Indexer: `https://testnet.sentry.exchange.grpc-web.injective.network`
- Chain at query time: `injective-888`, `injectived` `v1.20.4-bl.2`, block `145064015` at `2026-10-23T06:16:02.781159646Z`
  - `GET /cosmos/base/tendermint/v1beta1/node_info` excerpt: `"network":"injective-888"`, `"version":"v1.20.4-bl.2"`
  - `GET /cosmos/base/tendermint/v1beta1/blocks/latest` excerpt: `"height":"145064015"`

Orderbook responses are **aggregated price levels**, not individual resting orders. `Level` is `{p, q}`. LCD `GET /injective/exchange/v1beta1/derivative/orders/<id>` returned `{"code":12, "message":"Not Implemented", "details":[]}`. Individual order counts are **UNKNOWN**. Counts below are price-level counts. `limit=100` returned the same counts as the unfiltered book, so these books are complete.

Human price = chain `p` / 10^6 (`quote_decimals` 6 on every market below). Quantity is already in contract units.

## 1. Order book depth

URL that worked, for every market:

`GET https://testnet.sentry.lcd.injective.network/injective/exchange/v1beta1/derivative/orderbook/<market_id>?limit=100`

`?limit=5` returned the same best levels.

### GOLD/USDT PERP `0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23`

4 buy levels, 0 sell levels. Best bid `1860700000` ($1,860.70), qty `0.0005`. Best ask: none.

Raw (`limit=5`, identical to `limit=100`):

```json
{"buys_price_level":[{"p":"1860700000.000000000000000000","q":"0.000500000000000000"},{"p":"1860600000.000000000000000000","q":"0.000500000000000000"},{"p":"1860500000.000000000000000000","q":"0.000500000000000000"},{"p":"100000000.000000000000000000","q":"0.499700000000000000"}],"sells_price_level":[]}
```

| side | level | chain price | human | qty |
|---|---|---|---|---|
| bid | 1 | 1860700000 | $1,860.70 | 0.0005 |
| bid | 2 | 1860600000 | $1,860.60 | 0.0005 |
| bid | 3 | 1860500000 | $1,860.50 | 0.0005 |
| bid | 4 | 100000000 | $100.00 | 0.4997 |
| ask | — | none | none | 0 |

An atomic BUY from the vault cannot fill. There is no ask. An atomic SELL can hit 0.0005 at the best bid.

### BTC/USDC PERP `0xfd704649cf3a516c0c145ab0111717c44640d8dbe52a462ae35cadf2f6df1515`

10 buy levels, 0 sell levels. Best bid `81574000000` ($81,574), qty `0.001`. Best ask: none.

Raw top 5 (`?limit=5`; full book is 10 bids, 0 asks):

```json
{"buys_price_level":[{"p":"81574000000.000000000000000000","q":"0.001000000000000000"},{"p":"81000000000.000000000000000000","q":"0.001200000000000000"},{"p":"63272000000.000000000000000000","q":"0.002500000000000000"},{"p":"63265000000.000000000000000000","q":"0.010100000000000000"},{"p":"62268000000.000000000000000000","q":"0.001900000000000000"}],"sells_price_level":[]}
```

| side | level | chain price | human | qty |
|---|---|---|---|---|
| bid | 1 | 81574000000 | $81,574 | 0.0010 |
| bid | 2 | 81000000000 | $81,000 | 0.0012 |
| bid | 3 | 63272000000 | $63,272 | 0.0025 |
| bid | 4 | 63265000000 | $63,265 | 0.0101 |
| bid | 5 | 62268000000 | $62,268 | 0.0019 |
| ask | — | none | none | 0 |

### ETH/USDC PERP `0x135de28700392fb1c17d40d5170a74f30055a4ad522feddafec42fbbbb780897`

5 buy levels, 4 sell levels. Best bid `1818700000` ($1,818.70) qty `0.1322`. Best ask `1917000000` ($1,917.00) qty `70.6882`.

Raw (`limit=5` equals the full book):

```json
{"buys_price_level":[{"p":"1818700000.000000000000000000","q":"0.132200000000000000"},{"p":"1728700000.000000000000000000","q":"0.005700000000000000"},{"p":"1727400000.000000000000000000","q":"0.057000000000000000"},{"p":"1725300000.000000000000000000","q":"0.285000000000000000"},{"p":"1491500000.000000000000000000","q":"0.081700000000000000"}],"sells_price_level":[{"p":"1917000000.000000000000000000","q":"70.688200000000000000"},{"p":"2109800000.000000000000000000","q":"0.047000000000000000"},{"p":"2111500000.000000000000000000","q":"0.235000000000000000"},{"p":"3000000000.000000000000000000","q":"0.333100000000000000"}]}
```

### INJ/USDC PERP `0xdc70164d7120529c3cd84278c98df4151210c0447a65a2aab03459cf328de41e`

2 buy levels, 7 sell levels. Best bid `3000000` ($3.00) qty `1052.81`. Best ask `4900000` ($4.90) qty `86.63`.

Raw top 5 (`?limit=5`; `limit=100` count is buys=2, sells=7). The two asks past this cut were not re-printed.

```json
{"buys_price_level":[{"p":"3000000.000000000000000000","q":"1052.810000000000000000"},{"p":"2900000.000000000000000000","q":"8.960000000000000000"}],"sells_price_level":[{"p":"4900000.000000000000000000","q":"86.630000000000000000"},{"p":"5023000.000000000000000000","q":"89.550000000000000000"},{"p":"5180000.000000000000000000","q":"2.770000000000000000"},{"p":"5824000.000000000000000000","q":"99.530000000000000000"},{"p":"6540000.000000000000000000","q":"0.480000000000000000"}]}
```

### Why `mid_price_and_tob` is null on `GET /derivative/markets/<id>`

It is null for all four markets, including ETH and INJ which have both sides. That field is not evidence of an empty book.

`QueryDerivativeMarket` (singular) has no `with_mid_price_and_tob` flag. The list request does. From `injective-std` 1.20.0 `src/types/injective/exchange/v1beta1.rs`:

```rust
/// Flag to return the markets mid price and top of the book buy and sell orders.
pub with_mid_price_and_tob: bool, // QueryDerivativeMarketsRequest, tag 3

/// mid_price_and_tob defines the mid price for this market and the best ask and bid orders
pub mid_price_and_tob: ::core::option::Option<MidPriceAndTob>, // FullDerivativeMarket, tag 5
```

Passing `with_mid_price_and_tob=true` on the singular GOLD URL still returned `"mid_price_and_tob": null`. The LCD path does not forward that flag into the singular query.

The dedicated query does compute it, and mid price is null exactly when one side is missing:

`GET /injective/exchange/v1beta1/derivative/mid_price_and_tob/<id>`

| market | raw response |
|---|---|
| GOLD | `{"mid_price":null,"best_buy_price":"1860700000.000000000000000000","best_sell_price":null}` |
| BTC | `{"mid_price":null,"best_buy_price":"81574000000.000000000000000000","best_sell_price":null}` |
| ETH | `{"mid_price":"1867850000.000000000000000000","best_buy_price":"1818700000.000000000000000000","best_sell_price":"1917000000.000000000000000000"}` |
| INJ | `{"mid_price":"3950000.000000000000000000","best_buy_price":"3000000.000000000000000000","best_sell_price":"4900000.000000000000000000"}` |

ETH mid = (1818700000 + 1917000000) / 2 = 1867850000. INJ mid = (3000000 + 4900000) / 2 = 3950000. Mid price is the average of best bid and best ask, and it is null when either side is absent. The Go setter was not read: `github.com/InjectiveLabs/injective-core` returned HTTP 404 on 2026-10-23. Chain binary is `v1.20.4-bl.2`.

## 2. Recent trades

URL that worked:

`GET https://testnet.sentry.exchange.grpc-web.injective.network/api/exchange/derivative/v1/trades?marketId=<id>&skip=0&limit=5`

POST to that path returned `405` with `allow: GET`. `executedAt` is unix milliseconds.

### GOLD — paging.total = 7

Latest trade `executedAt` 1789002460447 = 2026-09-08T05:07:40.447Z. Nothing in the returned page is from the last day. Two subaccounts (`0xd0e6…` taker, `0xbabe…` maker cid `o100_b676e13c` / `o202_3f533f3c`). Not active liquidity right now: the ask side is empty.

```json
{"orderHash":"0xc25310583869eee5f1efec5ae71cb4b666506d9b48fd92d1502eb7a9c8c5e9b4","tradeExecutionType":"market","positionDelta":{"tradeDirection":"sell","executionPrice":"1860800000","executionQuantity":"0.0005","executionMargin":"310200"},"executedAt":1789002460447,"tradeId":"139571471_1","executionSide":"taker"}
{"orderHash":"0x42a94b3c20e481ff18d271fabca13c81b55231b44e2a598f30877d61e558a71e","tradeExecutionType":"limitFill","positionDelta":{"tradeDirection":"buy","executionPrice":"1860800000","executionQuantity":"0.0005","executionMargin":"930400"},"executedAt":1789002460447,"tradeId":"139571471_2","executionSide":"maker","cid":"o100_b676e13c"}
{"orderHash":"0xbfb3edd00320ac63efe02f83ef08a93885891775491cbeae944d0f90aec92920","tradeExecutionType":"limitFill","positionDelta":{"tradeDirection":"sell","executionPrice":"1861200000","executionQuantity":"0.0005","executionMargin":"930600"},"executedAt":1782440704453,"tradeId":"131559625_4","executionSide":"maker","cid":"o202_3f533f3c"}
{"orderHash":"0x348e74505cc424d4b9da84478df79c0a83e463ceaba2911b1b40cb7bf763d046","tradeExecutionType":"limitFill","positionDelta":{"tradeDirection":"sell","executionPrice":"1861100000","executionQuantity":"0.0005","executionMargin":"930550"},"executedAt":1782440704453,"tradeId":"131559625_3","executionSide":"maker","cid":"o201_6dbbcf35"}
{"orderHash":"0x2d70725f9c6f4386c77c94189a6ca04779cbf8f7bf5bd0d8c80e5820ed193162","tradeExecutionType":"market","positionDelta":{"tradeDirection":"buy","executionPrice":"1861150000","executionQuantity":"0.002","executionMargin":"1806451.612903225806"},"executedAt":1782440704453,"tradeId":"131559625_1","executionSide":"taker"}
```

### BTC — paging.total = 1000 (capped)

Latest `executedAt` 1792386891892 = 2026-10-19T04:21:31.892Z. All five rows are `tradeExecutionType: "synthetic"` with `orderHash` all zeros and fee recipient `inj1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqe2hm49`. These are not resting-book matches. The book has no asks, so nobody is currently offering the sell side.

```json
{"orderHash":"0x0000000000000000000000000000000000000000000000000000000000000000","tradeExecutionType":"synthetic","positionDelta":{"tradeDirection":"sell","executionPrice":"82185000000","executionQuantity":"0.0019","executionMargin":"154823400"},"executedAt":1792386891892,"tradeId":"144440788_1","executionSide":"taker","feeRecipient":"inj1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqe2hm49"}
{"orderHash":"0x0000000000000000000000000000000000000000000000000000000000000000","tradeExecutionType":"synthetic","positionDelta":{"tradeDirection":"buy","executionPrice":"82185000000","executionQuantity":"0.0019","executionMargin":"0"},"executedAt":1792386891892,"tradeId":"144440788_2","executionSide":"maker"}
```

Next three, same shape: `144440778_1` maker sell px `83012000000` qty `0.0038` at `1792386886402`; `144440778_2` taker buy same px/qty; `144440771_1` maker sell px `83012000000` qty `0.0019` at `1792386882542`.

### ETH — paging.total = 1000 (capped)

Latest two are real book trades at `1791817300481` = 2026-10-12T14:08:20.481Z and `1791817230916` = 2026-10-12T14:07:10.916Z. Maker cids `tc-web-…` and `tc-mobile-…`. The book still has both sides, so liquidity is resting, but the last non-synthetic trade in this page is about 11 days before the query block.

```json
{"orderHash":"0x2c9342645129178280f794a3d4782d53e658fa7144108ad40e9e8d3ea6d75fe2","tradeExecutionType":"market","positionDelta":{"tradeDirection":"sell","executionPrice":"1818700000","executionQuantity":"0.001","executionMargin":"0"},"executedAt":1791817300481,"tradeId":"143412659_1","executionSide":"taker","cid":"tc-mobile-and-07004da39b2446e3e4d5"}
{"orderHash":"0x17128e1854fc443e769716bd8b3a744b8e909f96e717bff1f988e92cbdc88931","tradeExecutionType":"limitFill","positionDelta":{"tradeDirection":"buy","executionPrice":"1818700000","executionQuantity":"0.001","executionMargin":"36374"},"executedAt":1791817300481,"tradeId":"143412659_2","executionSide":"maker","cid":"tc-web-2069d434-cf94-49c4-a"}
{"orderHash":"0xd389135968c96732f883d8d50302ab206f7497ba8a41b493ccf7bbdbdebf3f08","tradeExecutionType":"market","positionDelta":{"tradeDirection":"buy","executionPrice":"1917000000","executionQuantity":"0.001","executionMargin":"2000000"},"executedAt":1791817230916,"tradeId":"143412536_1","executionSide":"taker"}
```

Fifth row is synthetic: `140392979_1`, px `2445300000`, qty `0.0441`, `executedAt` 1789674582173.

### INJ — paging.total = 1000 (capped)

Latest five are all synthetic, zero order hash, fee recipient `inj1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqe2hm49`. Latest `executedAt` 1791906429134 = 2026-10-13T14:53:49.134Z. Both sides of the book are resting, but this page shows no recent non-synthetic trade.

```json
{"orderHash":"0x0000000000000000000000000000000000000000000000000000000000000000","tradeExecutionType":"synthetic","positionDelta":{"tradeDirection":"sell","executionPrice":"7769000","executionQuantity":"0.26","executionMargin":"0"},"executedAt":1791906429134,"tradeId":"143573773_1","executionSide":"taker"}
{"orderHash":"0x0000000000000000000000000000000000000000000000000000000000000000","tradeExecutionType":"synthetic","positionDelta":{"tradeDirection":"buy","executionPrice":"7769000","executionQuantity":"0.26","executionMargin":"673314"},"executedAt":1791906429134,"tradeId":"143573773_2","executionSide":"maker"}
```

Next three: `143573560_1` maker sell px `7808000` qty `0.27` at `1791906311720`; `143573560_2` taker buy same px/qty; `143553581_1` taker sell px `7715000` qty `2.66` margin `0` at `1791895266549`.

## 3. Seeding the GOLD book (not executed)

The ask side is empty. A vault `BuyAtomic` needs at least one resting **sell** limit that its worst price can cross. One order message is enough.

Smallest set:

1. `MsgDeposit` — `/injective.exchange.v1beta1.MsgDeposit` — fund the second wallet's exchange subaccount in `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5`.
2. `MsgCreateDerivativeLimitOrder` — `/injective.exchange.v1beta1.MsgCreateDerivativeLimitOrder` — `order.order_type = 2` (`SELL`), not post-only, not atomic. Atomic types are market orders and do not rest.

Do not use `BUY_PO`/`SELL_PO` unless the price is strictly non-crossing. A plain `SELL` above the best bid rests.

Market constraints from `GET /injective/exchange/v1beta1/derivative/markets/0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23`:

```json
{"quote_denom":"peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5","initial_margin_ratio":"0.195000000000000000","maker_fee_rate":"-0.000100000000000000","taker_fee_rate":"0.000500000000000000","min_price_tick_size":"100000.000000000000000000","min_quantity_tick_size":"0.000100000000000000","min_notional":"0.000001000000000000","quote_decimals":6,"mark_price":"1860940000.000000000000000000"}
```

Mark is $1,860.94. Nearest tick at or above the mark that does not cross the best bid ($1,860.70) is $1,861.00 = chain `1861000000`. Tick is `100000` ($0.10).

Worked example: resting SELL of 0.01 contract at $1,861.00, margin at the 0.195 IMR.

- notional human = 0.01 * 1861 = $18.61
- margin human = 18.61 * 0.195 = $3.62895
- margin chain = 3.62895 * 10^6 = `3628950`
- maker fee is negative (`-0.0001`), so a resting sell is not charged a positive fee up front. Deposit the margin. A few extra micro-units are harmless and were not measured.

`MsgDeposit`:

```json
{
  "@type": "/injective.exchange.v1beta1.MsgDeposit",
  "sender": "<second-wallet-inj>",
  "subaccount_id": "<second-wallet-default-subaccount-or-empty>",
  "amount": {"denom": "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5", "amount": "3628950"}
}
```

`MsgCreateDerivativeLimitOrder` (`DerivativeOrder` + `OrderInfo` from `injective-std` 1.20.0):

```json
{
  "@type": "/injective.exchange.v1beta1.MsgCreateDerivativeLimitOrder",
  "sender": "<second-wallet-inj>",
  "order": {
    "market_id": "0x8f002b45cb287a4c3ecb89174ee42a7e933178d89c7eea94dbed8dc5dfd35d23",
    "order_info": {
      "subaccount_id": "<second-wallet-subaccount-32-byte-hex>",
      "fee_recipient": "<second-wallet-inj>",
      "price": "1861000000",
      "quantity": "0.01",
      "cid": ""
    },
    "order_type": 2,
    "margin": "3628950",
    "trigger_price": "0"
  }
}
```

That is the opposite side of a vault BUY. `price` on the resting sell is the limit price, not a worst-price field. For the vault's later `BuyAtomic`, `order_info.price` must be >= `1861000000` or the order cannot cross this ask.

## 4. Faucet

Page fetched: `https://testnet.faucet.injective.network` (HTTP 200, `Last-Modified: Mon, 20 Oct 2025 16:12:40 GMT`).

Visible copy:

```html
<meta name="description" content="Get testnet INJ and USDT for free and start building on Injective.">
```

Runtime config in the same HTML:

```js
lambdaApi:"https://d1ikjbl0xk5pmx.cloudfront.net/v2"
network:"testnet"
faucetAddress:"inj1d6qx83nhx3a3gx7e654x4su8hur5s83u84h2xc"
hcaptchaSiteKey:"8c54351a-e2f6-452c-a5fa-35d255d6c11a"
```

The bundle `https://testnet.faucet.injective.network/_nuxt/BMln4rUi.js` posts:

```js
await se.post("faucet", {address: J.address, token: B.value})
```

`token` is the captcha token, not a denom selector. The string `peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5` does not occur in that bundle. No amount and no rate-limit number occur next to the faucet call. After a successful post the client waits 30s, then looks up a bank send from `faucetAddress` to the user. It does not display a coin amount.

`GET https://d1ikjbl0xk5pmx.cloudfront.net/v2/faucet` returned HTTP 200 body `There are no addresses to process`. `GET /v2`, `/v2/config`, `/v2/tokens`, `/v2/denoms` returned `{"message":"Not Found"}`.

Whether that USDT is `peggy0x87aB…B1B5`, the per-request amount, and the rate limit are **UNKNOWN**. The page only says "INJ and USDT".

## 5. Atomic order limits

Verified from chain params, docs, and `injective-std` 1.20.0:

- `BuyAtomic = 9`, `SellAtomic = 10` (`OrderType` in `v1beta1.rs`).
- LCD `GET /injective/exchange/v1beta1/exchangeParams`:

```json
{"atomic_market_order_access_level":"SmartContractsOnly","derivative_atomic_market_order_fee_multiplier":"2.000000000000000000"}
```
## Summary

| question | answer | evidence | status |
|---|---|---|---|
| 1. GOLD depth | 4 bid levels, 0 asks. Best bid 1860700000 qty 0.0005. No best ask. | LCD `/derivative/orderbook/0x8f002b45…?limit=100` raw above | VERIFIED |
| 1. BTC depth | 10 bid levels, 0 asks. Best bid 81574000000 qty 0.001. | same URL pattern, raw top 5 above | VERIFIED |
| 1. ETH depth | 5 bids, 4 asks. Bid 1818700000 qty 0.1322. Ask 1917000000 qty 70.6882. | same | VERIFIED |
| 1. INJ depth | 2 bids, 7 asks. Bid 3000000 qty 1052.81. Ask 4900000 qty 86.63. | same; top 5 raw, count at limit=100 | VERIFIED |
| 1. Individual order counts | UNKNOWN. Book query returns aggregated levels. `/derivative/orders/<id>` is `Not Implemented`. | `{"code":12,"message":"Not Implemented"}` | UNKNOWN |
| 1. Why markets endpoint mid is null | Singular market query has no flag and returns null even for two-sided books. Dedicated mid query is null only when one side is missing. | `injective-std` 1.20.0 `QueryDerivativeMarketsRequest.with_mid_price_and_tob`; GOLD/ETH mid raw above | VERIFIED |
| 1. Go function that sets the field | Not read. `injective-core` 404. | `https://github.com/InjectiveLabs/injective-core` HTTP 404 | UNKNOWN |
| 2. GOLD trades | 7 total. Latest 2026-09-08. Not active. | indexer trades raw above | VERIFIED |
| 2. BTC trades | Latest 2026-10-19, all synthetic in the page. No resting ask. | indexer trades | VERIFIED |
| 2. ETH trades | Real market/limitFill on 2026-10-12. Book still two-sided. | indexer trades + orderbook | VERIFIED |
| 2. INJ trades | Latest page is synthetic, 2026-10-13. Book still two-sided. | indexer trades + orderbook | VERIFIED |
| 3. Seed set | `MsgDeposit` + one `MsgCreateDerivativeLimitOrder` SELL `order_type` 2, price `1861000000`, qty `0.01`, margin `3628950`, denom `peggy0x87aB…B1B5`. Not sent. | market JSON + `DerivativeOrder` fields | VERIFIED as encoding; NOT executed |
| 4. Faucet dispenses peggy USDT | Page says INJ and USDT. Peggy denom, amount, and rate limit not in the page, bundle, or API. | faucet HTML + `BMln4rUi.js` + `GET /v2/faucet` | UNKNOWN |
| 5. Atomic access + fee | `SmartContractsOnly`, fee multiplier `2.0`. Types 9 and 10. | `exchangeParams` raw + order-types doc | VERIFIED |
| 5. Atomic margin `"0"` allowed | Not established for atomic types specifically. | keeper source unavailable | UNKNOWN |
| 5. Atomic requires mid price / both sides | Not stated in the order-types doc. Dedicated mid query is null iff one side is empty. | order-types page + mid_price_and_tob raw | UNKNOWN |


- Docs `https://docs.injective.network/defi/trading/order-types` (page last modified June 9, 2026): "BUY_ATOMIC (9): An atomic buy order is a market order that gets executed instantly, bypassing the Frequent Batch Auctions (FBA). It's intended for smart contracts that need to execute a trade instantly. A higher fee is paid defined in the global exchange parameters (currently it is two times the normal trading fee)." SELL_ATOMIC (10) is the same on the sell side. The page does not say atomic orders require a mid price or both sides of the book.

Not verified on the Go keeper (`injective-core` HTTP 404):

- Whether `margin: "0"` is accepted on `BuyAtomic` / `SellAtomic` specifically. Docs and the previous research note treat margin `"0"` as reduce-only for derivative orders generally. No source line in this pass says atomic close is special-cased or forbidden. **UNKNOWN for the atomic-only question.**
- Whether an atomic order is rejected when `mid_price` is null, as opposed to the normal "no opposing liquidity" failure. ETH has a mid price and both sides; GOLD has neither an ask nor a mid price. That is consistent with mid price requiring both sides, not with atomic execution requiring a mid price. **UNKNOWN.**

A 0.01 BUY near the mark, if someone is seeding the bid instead, uses `order_type: 1`, the same price and quantity, and the same margin `3628950`. That does not let a vault BUY fill.
