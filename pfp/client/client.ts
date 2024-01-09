const web3 = require("@solana/web3.js");
const bs58 = require("bs58");

// --- Schema ---------------------------------------------------------------------
//  Config Account
//
//               |- Claim Account
//  NFT Account -|- ...
//               |- Claim Account
//
//  ...
//
//               |- Claim Account
//  NFT Account -|- ...
//               |- Claim Account
//
// --------------------------------------------------------------------------------
// 1. Config account
// {
//     authority: Pubkey,                   // Who can edit this config and register new NFTs for staking
//     claimable_from: UnixTimestamp,       // Starting date for claiming
//     total_reward: f64,                   // Total reward for a single NFT over the whole staking period
//     initial_reward_frac: f32,            // What fraction of the total reward should be given to user on the initial claim
//     reward_period_sec: i32,              // Staking period in seconds
// }
// Seeds: "config"
async function getConfigPda(program_id) {
    const seeds = [Buffer.from("config")]
    return web3.PublicKey.findProgramAddress(seeds, program_id);
}

async function parseConfig(config_pda, connection) {
    const response = await connection.getAccountInfo(config_pda);
    if (response == null) {
        return null;
    }
    return {
        authority: bs58.encode(response.data.slice(0, 32)),
        claimable_from: new Date(1000 * Number(response.data.readBigInt64LE(32))),
        total_reward: response.data.readDoubleLE(40),
        initial_reward_frac: response.data.readFloatLE(48),
        reward_period_sec: response.data.readInt32LE(52),
    };
}

// 2. NFT record
// {
//     nonce: i32,                          // How many times this NFT has been claimed
//     claimed_amount: f64,                 // GMRX amount already claimed
//     total_amount: f64,                   // Total amount available for claim
//     last_claim_at: UnixTimestamp,        // Time of the latest claim
// }
// Seeds: "nft", NFT address
async function getNftPda(nft_public_key, program_id) {
    const seeds = [Buffer.from("nft"), nft_public_key.toBuffer()]
    return web3.PublicKey.findProgramAddress(seeds, program_id);
}

async function parseNft(nft_pda, connection) {
    const response = await connection.getAccountInfo(nft_pda);
    if (response == null) {
        return null;
    }
    const timestamp = response.data.readBigInt64LE(20);
    return {
        nonce: response.data.readInt32LE(0),
        claimed_amount: response.data.readDoubleLE(4),
        total_amount: response.data.readDoubleLE(12),
        last_claimed_at: timestamp == 0 ? null : new Date(1000 * Number(timestamp))
    };
}

// 3. Claim record
// {
//     claimed_amount: f64,                 // Reward amount for this single claim
//     bnb_chain_wallet_address: [u8; 40],  // BNB chain wallet address that we need to send GMRXs to
// }
// Seeds: "claim", NFT address, nonce
async function getClaimPda(nft_public_key, nonce, program_id) {
    const nonce_buf = Buffer.allocUnsafe(4);
    nonce_buf.writeInt32LE(nonce);
    const seeds = [Buffer.from("claim"), nft_public_key.toBuffer(), nonce_buf]
    return web3.PublicKey.findProgramAddress(seeds, program_id);
}

async function parseClaim(claim_pda, connection) {
    const response = await connection.getAccountInfo(claim_pda);
    return response == null ? null : {
        claimed_amount: response.data.readDoubleLE(0),
        bnb_chain_wallet_address: '0x' + response.data.toString('utf8', 8)
    };
}
// --------------------------------------------------------------------------------


// --- Solana Program Methods -----------------------------------------------------
// 1. Create/Update a config account
//  Accounts
//   1. Authority/Fee payer (signer)
//   2. Config PDA (writable)
//   3. System Program
//  Data
//   1. Instruction code (0), 1 byte
//   2. Claimable from, 8 bytes
//   3. Total reward, 8 bytes
//   4. Initial reward fraction, 4 bytes
//   5. Reward period, 4 bytes
async function setConfigInstruction(program_id, authority_key, claimable_from, total_reward, initial_reward_frac, reward_period_sec) {
    const [config_pda, config_bump] = await getConfigPda(program_id);
    const buf = Buffer.allocUnsafe(8 + 8 + 4 + 4);
    buf.writeBigInt64LE(BigInt(Math.floor(claimable_from.getTime() / 1000)));
    buf.writeDoubleLE(total_reward, 8);
    buf.writeFloatLE(initial_reward_frac, 16);
    buf.writeInt32LE(reward_period_sec, 20);
    return new web3.TransactionInstruction({
        data: Buffer.from(Uint8Array.of(0, ...buf)),
        keys: [
            {pubkey: authority_key, isSigner: true, isWritable: false},
            {pubkey: config_pda, isSigner: false, isWritable: true},
            {pubkey: web3.SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: program_id,
    });
}

// 2. Register an NFT for staking
//  Accounts
//   1. Authority/Fee Payer (signer)
//   2. NFT
//   3. NFT PDA (writable)
//   4. Config PDA
//   5. System Program
//  Data
//   1. Instruction code (1), 1 byte
async function registerNftInstruction(program_id, user_key, nft_public_key) {
    const [nft_pda, nft_bump] = await getNftPda(nft_public_key, program_id);
    const [config_pda, config_bump] = await getConfigPda(program_id);
    return new web3.TransactionInstruction({
        data: Buffer.from(Uint8Array.of(1)),
        keys: [
            {pubkey: user_key, isSigner: true, isWritable: true},
            {pubkey: nft_public_key, isSigner: false, isWritable: false},
            {pubkey: nft_pda, isSigner: false, isWritable: true},
            {pubkey: config_pda, isSigner: false, isWritable: false},
            {pubkey: web3.SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: program_id,
    });
}

// 3. Claim a reward
//  Accounts
//   1. User/Fee Payer (signer)
//   2. NFT
//   3. User Token Account
//   4. NFT PDA (writable)
//   5. Claim PDA (writable)
//   6. Config PDA
//   7. System Program
//  Data
//   1. Instruction code (2), 1 byte
//   2. BNB Chain Wallet Address, 40 bytes
async function claimInstruction(program_id, user_key, token_key, nft_public_key, bnb_wallet, connection) {
    const [nft_pda, nft_bump] = await getNftPda(nft_public_key, program_id);
    const nft = await parseNft(nft_pda, connection);
    const [claim_pda, claim_bump] = await getClaimPda(nft_public_key, nft == null ? 1 : nft.nonce + 1, program_id);
    const [config_pda, config_bump] = await getConfigPda(program_id);

    const enc = new TextEncoder();

    return new web3.TransactionInstruction({
        data: Buffer.from(Uint8Array.of(2, ...enc.encode(bnb_wallet.slice(2)))),
        keys: [
            {pubkey: user_key, isSigner: true, isWritable: true},
            {pubkey: nft_public_key, isSigner: false, isWritable: false},
            {pubkey: token_key, isSigner: false, isWritable: false},
            {pubkey: nft_pda, isSigner: false, isWritable: true},
            {pubkey: claim_pda, isSigner: false, isWritable: true},
            {pubkey: config_pda, isSigner: false, isWritable: false},
            {pubkey: web3.SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: program_id,
    });
}
// --------------------------------------------------------------------------------




(async () => {
    let connection = new web3.Connection("http://localhost:8899");
    const program_id = new web3.PublicKey("EyFcxAER1Qo9Qz9mRobZjwWyHjvLuGpYYrH2oyGsdE7y");

    let secretKey = Uint8Array.from(require("./keys/1.json"));
    let keypair = web3.Keypair.fromSecretKey(secretKey);

    const nft = new web3.PublicKey("Gfm8g6np7FqEg53ojRetpHxEMEaaa3SwyPb6ZenRWRHC");
    const token_acc = new web3.PublicKey("4Pj4T38xDpymxhvQcjGcnrZjLTV2o6e64bK2XwtkxwW1");
    const bnb_wallet = '0x20bebcBe6cFe1a2e97300d9328067E1b546c2Ab9';

    const transaction = new web3.Transaction();
    transaction.add(await setConfigInstruction(program_id, keypair.publicKey, new Date(), 40000, 0.2, 300))
    // transaction.add(await registerNftInstruction(program_id, keypair.publicKey, nft));
    // transaction.add(await claimInstruction(program_id, keypair.publicKey, token_acc, nft, bnb_wallet, connection));
    await web3.sendAndConfirmTransaction(
        connection,
        transaction,
        [keypair],
    );

    const [config_pda, config_bump] = await getConfigPda(program_id);
    const config = await parseConfig(config_pda, connection);
    console.log("Config account:")
    console.log(config);
    console.log("")

    const [nft_pda, nft_bump] = await getNftPda(nft, program_id);
    const nft = await parseNft(nft_pda, connection);
    console.log("NFT account:")
    console.log(nft);
    console.log("")

    console.log("Claim accounts:")
    for (let claim, nonce = 1; (claim = await parseClaim((await getClaimPda(nft, nonce, program_id))[0], connection)) != null; nonce++) {
        console.log(claim);
    }

})()
