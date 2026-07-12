# PassionProof

A soulbound Solana badge for consistent, meaningful contribution — starting with
open source. Built for the DEV Weekend Challenge: Passion Edition.

Digital achievements are usually transferable, sellable, or detached from the
person who earned them. PassionProof explores the opposite idea: what if
recognition for real contribution couldn't be traded? Every badge is a
Token-2022 mint with Solana's **NonTransferable** extension, permanently
locking it to the wallet that earned it — plus an on-chain **reason** field,
so the badge is a verifiable record of a specific milestone, not just a
picture.

## What it uses

- **Token-2022 NonTransferable extension** — enforced at the protocol level;
  any `TransferChecked` instruction against the badge fails on-chain.
- **Token-2022 MetadataPointer + TokenMetadata extensions** — metadata lives
  directly on the mint account, no external metadata program required.
- **Custom metadata fields** (`category`, `awarded_for`, `recipient`,
  `issued`) — the "reason" a badge exists, written on-chain via
  `update_field`.

## Program flow

1. `create_badge_mint` — creates the Token-2022 mint, initializes the
   NonTransferable + MetadataPointer extensions, sets name/symbol/uri, then
   writes the reason fields.
2. `mint_badge` — mints exactly 1 unit into the recipient's associated token
   account. Because the mint is NonTransferable, the resulting token account
   is permanently soulbound.

## Running it (WSL / local Anchor environment)

```bash
# from the passionproof/ directory
yarn install
anchor keys list          # copy the generated program ID
# paste it into Anchor.toml [programs.localnet] and declare_id!() in lib.rs
anchor build
anchor test
```

The test suite (`tests/passionproof.ts`) does three things:

1. Creates the badge mint with real metadata (statix PR merge as the example).
2. Mints the badge to a recipient wallet.
3. **Attempts to transfer it and asserts that the transfer fails** — this is
   the actual proof-of-concept the whole project rests on.

## Demo script (for the recording)

1. Run `anchor test` and let the terminal show all three tests passing —
   especially the failed-transfer assertion.
2. Pull up the mint account in Solana Explorer (devnet) and show the
   `NonTransferable` and `MetadataPointer` extensions listed on the account.
3. Show the metadata fields (`category`, `awarded_for`, `recipient`,
   `issued`) resolved from the mint.

## What's next (not built this weekend)

- Frontend: connect wallet → "Mint PassionProof" → display badge + "Soulbound:
  Cannot Transfer" state.
- Streak tracking / multiple badge categories.
- Deployed devnet program + hosted single-page demo.