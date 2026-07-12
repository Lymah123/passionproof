import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createTransferCheckedInstruction,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";

describe("PassionProof", () => {
  const mint = new web3.Keypair();
  const recipient = new web3.Keypair(); // stands in for the badge recipient's wallet
  const attacker = new web3.Keypair(); // stands in for anyone trying to receive a "transfer"

  it("creates the badge mint with NonTransferable + Metadata extensions", async () => {
    const txHash = await pg.program.methods
      .createBadgeMint(
        "PassionProof #001",
        "PPROOF",
        "https://example.com/passionproof/001.json",
        "Open Source Contributions",
        "Merged first PR into statix",
        "Harlimat Odunola"
      )
      .accounts({
        payer: pg.wallet.publicKey,
        mintAuthority: pg.wallet.publicKey,
        mint: mint.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([mint])
      .rpc();

    console.log(`create_badge_mint tx: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);
  });

  it("mints exactly 1 soulbound badge to the recipient", async () => {
    const recipientAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipient.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    const txHash = await pg.program.methods
      .mintBadge()
      .accounts({
        payer: pg.wallet.publicKey,
        mintAuthority: pg.wallet.publicKey,
        mint: mint.publicKey,
        recipient: recipient.publicKey,
        recipientTokenAccount: recipientAta,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: new web3.PublicKey(
          "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        ),
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log(`mint_badge tx: ${txHash}`);
    await pg.connection.confirmTransaction(txHash);

    const account = await pg.connection.getTokenAccountBalance(recipientAta);
    console.log("Recipient badge balance:", account.value.amount);
    assert(account.value.amount === "1");
  });

  it("rejects any attempt to transfer the badge -- this IS the proof", async () => {
    const fundTx = await pg.connection.requestAirdrop(recipient.publicKey, web3.LAMPORTS_PER_SOL / 100).catch(() => null);
    if (!fundTx) {
      // Faucet rate-limited -- fund from our own wallet instead
      const transferTx = new web3.Transaction().add(
        web3.SystemProgram.transfer({
          fromPubkey: pg.wallet.publicKey,
          toPubkey: recipient.publicKey,
          lamports: web3.LAMPORTS_PER_SOL / 100,
        })
      );
      await web3.sendAndConfirmTransaction(pg.connection, transferTx, [pg.wallet.keypair]);
    } else {
      await pg.connection.confirmTransaction(fundTx);
    }

    const recipientAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipient.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );
    const attackerAta = getAssociatedTokenAddressSync(
      mint.publicKey,
      attacker.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    // Create the attacker's token account first, so the transfer failure
    // below is unambiguously caused by NonTransferable, not a missing account.
    const createAtaTx = new web3.Transaction().add(
      createAssociatedTokenAccountInstruction(
        pg.wallet.publicKey,
        attackerAta,
        attacker.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await web3.sendAndConfirmTransaction(pg.connection, createAtaTx, [pg.wallet.keypair]);

    const ix = createTransferCheckedInstruction(
      recipientAta,
      mint.publicKey,
      attackerAta,
      recipient.publicKey,
      1,
      0,
      [],
      TOKEN_2022_PROGRAM_ID
    );

    let failedAsExpected = false;
    try {
      const tx = new web3.Transaction().add(ix);
      await web3.sendAndConfirmTransaction(pg.connection, tx, [recipient]);
    } catch (err) {
      failedAsExpected = true;
      console.log("Transfer correctly rejected:", err.message ?? err);
    }

    assert(failedAsExpected, "Transfer should be rejected by NonTransferable extension");
  });
});