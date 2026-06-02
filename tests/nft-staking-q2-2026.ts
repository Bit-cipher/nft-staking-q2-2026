import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_PROGRAM_ID as ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  associatedAddress,
} from "@coral-xyz/anchor/dist/cjs/utils/token";

const MILLISECONDS_PER_DAY = 86400000;
const REWARDS_BPS = 10000;
const FREEZE_PERIOD_IN_DAYS = 0;
const TIME_TRAVEL_IN_DAYS = 8;

const MPL_CORE_PROGRAM_ID = new anchor.web3.PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);

describe("NftStakingQ22026", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.NftStakingQ22026 as Program<any>;

  const collectionKeypair = anchor.web3.Keypair.generate();
  const nftKeypair = anchor.web3.Keypair.generate();

  // Find update authority PDA
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Find config PDA
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Find rewards mint PDA
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards_mint"), config.toBuffer()],
    program.programId
  )[0];

  // Helper function to advance time
  async function advanceTime(params: {
    absoluteEpoch?: number;
    absoluteSlot?: number;
    absoluteTimestamp?: number;
  }): Promise<void> {
    const result = await (provider.connection as any)._rpcRequest(
      "surfnet_timeTravel",
      [params]
    );

    if (result.error) {
      throw new Error(
        `Time travel failed: ${JSON.stringify(result.error)}`
      );
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  it("Create a collection", async () => {
    const collectionName = "Test Collection";
    const collectionUri = "https://example.com/collection";

    const tx = await program.methods
      .createCollection(collectionName, collectionUri)
      .accountsPartial({
        payer: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collectionKeypair])
      .rpc();

    console.log("\n========== COLLECTION CREATED ==========");
    console.log("Transaction Signature:", tx);
    console.log(
      "Collection Address:",
      collectionKeypair.publicKey.toBase58()
    );
    console.log(
      "Update Authority PDA:",
      updateAuthority.toBase58()
    );
    console.log("========================================\n");
  });

  it("Mint an NFT", async () => {
    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";

    const tx = await program.methods
      .mintAsset(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        asset: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc();

    console.log("\n============= NFT MINTED =============");
    console.log("Transaction Signature:", tx);
    console.log("NFT Address:", nftKeypair.publicKey.toBase58());
    console.log(
      "Collection Address:",
      collectionKeypair.publicKey.toBase58()
    );
    console.log("======================================\n");
  });

  it("Initialize Config", async () => {
    const tx = await program.methods
      .initialize(REWARDS_BPS, FREEZE_PERIOD_IN_DAYS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    console.log("\n=========== CONFIG INITIALIZED ==========");
    console.log("Transaction Signature:", tx);
    console.log("Config Address:", config.toBase58());
    console.log("Rewards Mint:", rewardsMint.toBase58());
    console.log("Rewards BPS:", REWARDS_BPS);
    console.log(
      "Freeze Period (Days):",
      FREEZE_PERIOD_IN_DAYS
    );
    console.log("=========================================\n");
  });

  it("Stake an NFT", async () => {
    const tx = await program.methods
      .stake()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        updateAuthority,
        config,
        asset: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();

    console.log("\n============= NFT STAKED =============");
    console.log("Transaction Signature:", tx);
    console.log("NFT:", nftKeypair.publicKey.toBase58());
    console.log("Config:", config.toBase58());
    console.log("======================================\n");
  });

  it(
    "Try to unstake an NFT before the freeze periods ends",
    async function () {
      if (FREEZE_PERIOD_IN_DAYS === 0) {
        console.log(
          "\nSkipping freeze period test because FREEZE_PERIOD_IN_DAYS = 0\n"
        );
        this.skip();
      }

      const userRewardsAta = associatedAddress({
        mint: rewardsMint,
        owner: provider.wallet.publicKey,
      });

      console.log("\n===== ATTEMPTING EARLY UNSTAKE =====");
      console.log("User Rewards ATA:", userRewardsAta.toBase58());

      try {
        const tx = await program.methods
          .unstake()
          .accountsPartial({
            owner: provider.wallet.publicKey,
            updateAuthority,
            config,
            rewardsMint,
            userRewardsAta,
            asset: nftKeypair.publicKey,
            mplCoreProgram: MPL_CORE_PROGRAM_ID,
            collection: collectionKeypair.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram:
              ASSOCIATED_TOKEN_PROGRAM_ID,
          })
          .rpc();

        throw new Error(
          `Unstake should have failed before freeze period elapsed, but succeeded with tx: ${tx}`
        );
      } catch (err: any) {
        if (
          err instanceof anchor.AnchorError &&
          err.error.errorCode.code ===
            "FreezePeriodNotElapsed"
        ) {
          console.log(
            "\nUnstake failed as expected:"
          );
          console.log(err.error.errorMessage);
        } else {
          throw err;
        }
      }

      console.log("====================================\n");
    }
  );

  it(
    "Advance time beyond the freeze period and try to unstake again",
    async function () {
      if (FREEZE_PERIOD_IN_DAYS === 0) {
        console.log(
          "\nSkipping time travel because FREEZE_PERIOD_IN_DAYS = 0\n"
        );
        this.skip();
      }

      const currentTimestamp = Date.now();

      await advanceTime({
        absoluteTimestamp:
          currentTimestamp +
          TIME_TRAVEL_IN_DAYS *
            MILLISECONDS_PER_DAY,
      });

      console.log("\n=========== TIME TRAVEL ===========");
      console.log(
        `Advanced time by ${TIME_TRAVEL_IN_DAYS} days`
      );
      console.log("===================================\n");
    }
  );

  it("Unstake an NFT", async () => {
    const userRewardsAta = associatedAddress({
      mint: rewardsMint,
      owner: provider.wallet.publicKey,
    });

    const tx = await program.methods
      .unstake()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        asset: nftKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        collection: collectionKeypair.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram:
          ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    const rewardsBalance =
      await provider.connection.getTokenAccountBalance(
        userRewardsAta
      );

    console.log("\n============ NFT UNSTAKED ============");
    console.log("Transaction Signature:", tx);
    console.log(
      "User Rewards ATA:",
      userRewardsAta.toBase58()
    );
    console.log(
      "User Rewards Balance:",
      rewardsBalance.value.uiAmountString
    );
    console.log("======================================\n");
  });
});