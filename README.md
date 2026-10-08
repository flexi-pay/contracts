# FlexiPay contracts

Soroban smart contracts (Rust) for the **Starling** wallet by FlexiPay.

| Repo | What it is |
|---|---|
| [flexi-pay/frontend](https://github.com/flexi-pay/frontend) | The Starling wallet: a web app and Chrome extension |
| [flexi-pay/backend](https://github.com/flexi-pay/backend) | Federation and names service (`you*flexipay.app`) |
| **flexi-pay/contracts** | This repo: on-chain logic |

## `escrow`: Safe pay

A buyer locks any Stellar token (XLM, USDC, … through its Stellar Asset Contract) for a seller.

| Function | Who can call it | What it does |
|---|---|---|
| `create(buyer, seller, token, amount, deadline, memo) -> u64` | buyer | Moves `amount` into the contract and returns the escrow id |
| `release(id)` | buyer | Pays the seller |
| `refund(id, caller)` | seller (any time), or buyer (after `deadline`) | Returns the funds to the buyer |
| `get(id)`, `count()`, `list(before, limit)` | anyone | Read-only views |

- Typed errors (`#[contracterror]`) and events for created, released and refunded.
- Persistent storage, with time-to-live extended on every touch.
- **No admin key.** Nobody can move escrowed funds except in the cases above.
- Amounts use the token's base units (7 decimals for Stellar assets).

## Develop

```bash
rustup target add wasm32v1-none
cargo install --locked stellar-cli

make test        # 9 unit tests in a local Soroban environment
make lint        # rustfmt + clippy (-D warnings)
make wasm        # → target/wasm32v1-none/release/flexipay_escrow.wasm
```

## Deploy

```bash
NETWORK=testnet ./scripts/deploy.sh
```

This prints the contract ID. Set it in `flexi-pay/frontend` as `VITE_ESCROW_CONTRACT_ID`.

> Get the contracts audited before using them with real funds on mainnet.

## License

MIT
