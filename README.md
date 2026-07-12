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

**Deployed and verified on devnet:**
[`HSSLcVQmCdCo8qt9UBMSAL9vbqpxYLkCLHoz74dgrBE1`](https://explorer.solana.com/address/HSSLcVQmCdCo8qt9UBMSAL9vbqpxYLkCLHoz74dgrBE1?cluster=devnet)

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

## Running it

This program was built, deployed, and tested using
**[Solana Playground](https://beta.solpg.io)** — a browser-based Anchor IDE
that compiles and runs on Playground's own servers, with no local Rust/Solana
toolchain required.

**Why:** a local build in WSL hit a hard blocker — a transitive dependency
required Cargo's `edition2024` feature, which the Solana CLI's bundled
platform-tools compiler (Rust 1.79.0) doesn't support. Combined with an
unreliable network that repeatedly timed out on the ~375MB platform-tools
download, Playground was the faster, more reliable path for a weekend
deadline. (If you have a newer local toolchain with platform-tools that
supports `edition2024`, `anchor build`/`anchor test` should also work locally
— just note the local path isn't what was verified for this submission.)

**To run it yourself in Playground:**

1. Go to [beta.solpg.io](https://beta.solpg.io) and create a new Anchor project.
2. Replace the default `src/lib.rs` with [`programs/passionproof/src/lib.rs`](programs/passionproof/src/lib.rs) from this repo.
3. Replace the default test file with [`tests/passionproof.ts`](tests/passionproof.ts) from this repo.
4. Connect/save a Playground wallet, and airdrop devnet SOL (`solana airdrop 2` in the Playground terminal, or use [faucet.solana.com](https://faucet.solana.com) if rate-limited).
5. Click **Build**, then **Deploy** (~2.1 SOL for initial deployment).
6. Click **Test**.

The test suite does three things, all executed as real transactions against
the deployed program on devnet:

1. Creates the badge mint with real metadata (statix PR merge as the example).
2. Mints the badge to a recipient wallet.
3. **Attempts to transfer it and asserts that the transfer fails** — the
   actual proof-of-concept the whole project rests on. The rejection comes
   directly from the Token-2022 program:
   ```
   Program log: Transfer is disabled for this mint
   ```

## Verified on-chain

- Program account: [`HSSLcVQmCdCo8qt9UBMSAL9vbqpxYLkCLHoz74dgrBE1`](https://explorer.solana.com/address/HSSLcVQmCdCo8qt9UBMSAL9vbqpxYLkCLHoz74dgrBE1?cluster=devnet)
- `create_badge_mint` transaction: [`2JcbBHty...`](https://explorer.solana.com/tx/2JcbBHtyRpFCcrsNJ4jNChMy46EKCg73wtUfY2j3Qq4ADCzRWbGLvD5DWQsDMnn7WjZHwWefpCNCrrbjst94hGtq?cluster=devnet) — logs show `InitializeNonTransferableMint`, `MetadataPointerInstruction::Initialize`, `InitializeMint2`, and the `TokenMetadataInstruction::UpdateField` calls writing each reason field.
- `mint_badge` transaction: [`3NAjTLLF...`](https://explorer.solana.com/tx/3NAjTLLFPf1PxkVf1BpxmLio68o357FEcaTSBA9WBjVs1KKSxKkPieSJr4UmDYmqKrAFKW6Us3nfZNSeBezESGFi?cluster=devnet)

## Honest scope note

The `awarded_for` field is self-attested by whoever holds mint authority —
the program records *what* the contribution was, but doesn't independently
verify it against, say, the GitHub API. The trust model is the same as a
signed certificate: the record is permanent and tamper-proof once issued, but
the issuing authority is what backs the claim.

## What's next (not built this weekend)

- GitHub-verified minting, so `awarded_for` becomes a verified fact instead
  of a self-attested string.
- Frontend: connect wallet → "Mint PassionProof" → display badge + "Soulbound:
  Cannot Transfer" state.
- Streak tracking / multiple badge categories.
- A path from devnet to mainnet once the extension logic has more mileage.