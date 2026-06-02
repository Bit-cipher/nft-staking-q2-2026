# NFT Staking Program (Anchor + MPL Core)

A modern NFT staking program built on Solana using Anchor and MPL Core.  
Users can stake NFT assets from an MPL Core collection, earn time-based rewards in SPL tokens, and unstake after a configurable freeze period.

This project demonstrates real-world Solana patterns including PDAs, CPI calls to MPL Core, token minting, and attribute-based NFT state tracking.



## Features

- Stake NFTs from an MPL Core collection
- Time-based reward system (based on staking duration)
- Freeze period enforcement before unstaking
- Reward minting using SPL Token-2022 interface
- NFT state tracking using MPL Core Attributes plugin
- Secure PDA-based authority system



## Program ID
Aq1EUwoSAMKKPzHgVKQkbHHDRBQQZLyDrzXUZZZX8zYU




## Architecture

### State

#### Config (PDA)

Stores global staking configuration.

| Field          | Type    | Description |
|----------------|---------|-------------|
| rewards_bps    | u16     | Reward rate per day (basis points) |
| freeze_period  | u16     | Lock period before unstaking |
| bump           | u8      | PDA bump |
| rewards_bump   | u8      | Mint PDA bump |


### PDAs

| Account            | Seeds |
|--------------------|------|
| Config             | `["config", collection_pubkey]` |
| Rewards Mint       | `["rewards_mint", config]` |
| Update Authority   | `["update_authority", collection_pubkey]` |



## Instructions

### 1. `create_collection`
Creates an MPL Core collection that will be used for NFT staking.

- Initializes collection metadata
- Sets update authority PDA



### 2. `mint_asset`
Mints an NFT into the collection.

- Creates MPL Core asset
- Assigns it to user wallet
- Links it to collection



### 3. `initialize`
Initializes staking configuration.

- Sets reward rate (bps)
- Sets freeze period
- Creates rewards mint
- Stores PDA state



### 4. `stake`
Stake an NFT into the protocol.

- Marks NFT as staked using MPL Core Attributes plugin
- Stores timestamp (`staked_at`)
- Locks NFT from being unstaked immediately



### 5. `unstake`
Unstakes NFT after freeze period.

- Validates staking state
- Checks freeze period has elapsed
- Calculates rewards based on time staked
- Mints reward tokens to user ATA
- Updates NFT attributes to unstaked state



## Reward Formula
reward =
(staked_days × rewards_bps × 10^decimals) / 10_000



## Tech Stack

- Anchor 0.31+
- MPL Core
- SPL Token Interface
- TypeScript (Anchor tests)
- Surfpool (local testing + time travel)
- LiteSVM (fast simulation alternative)



## Testing

This project includes full end-to-end tests:

- Collection creation
- NFT minting
- Config initialization
- Staking flow
- Freeze-period enforcement
- Unstaking + reward distribution

### Run tests

```bash
anchor build
anchor test --skip-local-validator