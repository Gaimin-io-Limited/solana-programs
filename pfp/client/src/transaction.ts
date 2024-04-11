import * as crypto from "crypto";
import { AddressLookupTableAccount, Connection, Keypair, PublicKey, TransactionMessage, TransactionMessageArgs, VersionedTransaction } from "@solana/web3.js";
import { ConfigArgs, } from "./types";
import * as ix from "./instruction";

export async function sendAndConfirmTransaction(connection: Connection, signers: Keypair[], msg: Omit<TransactionMessageArgs, 'recentBlockhash'>, luts?: AddressLookupTableAccount[]) {
    const blockhash = await connection.getLatestBlockhash();
    const tx = new VersionedTransaction(new TransactionMessage({
        recentBlockhash: blockhash.blockhash,
        ...msg,
    }).compileToV0Message(luts));
    tx.sign(signers);

    console.log("Size", tx.serialize().length);

    const signature = await connection.sendTransaction(tx);
    return await connection.confirmTransaction({ signature, ...blockhash });
}

export async function setConfig(connection: Connection, authPayer: Keypair, creator: PublicKey, config: ConfigArgs) {
    return await sendAndConfirmTransaction(connection, [authPayer], {
        payerKey: authPayer.publicKey,
        instructions: [
            ix.setConfigInstruction(authPayer.publicKey, creator, config),
        ],
    });
}

export async function deleteAccount(connection: Connection, authPayer: Keypair, acc: PublicKey) {
    return await sendAndConfirmTransaction(connection, [authPayer], {
        payerKey: authPayer.publicKey,
        instructions: [
            ix.deleteAccountInstruction(authPayer.publicKey, acc),
        ],
    });
}

export async function registerNft(connection: Connection, authPayer: Keypair, mint: PublicKey) {
    return await sendAndConfirmTransaction(connection, [authPayer], {
        payerKey: authPayer.publicKey,
        instructions: [
            ix.registerNftInstruction(authPayer.publicKey, mint),
        ],
    });
}

export async function claimReward(connection: Connection, userPayer: Keypair, mints: PublicKey[], bnbWallet: string, luts?: AddressLookupTableAccount[]) {
    const claim_seed = crypto.randomBytes(32);

    return await sendAndConfirmTransaction(connection, [userPayer], {
        payerKey: userPayer.publicKey,
        instructions: [
            ix.createClaimInstruction(userPayer.publicKey, bnbWallet, claim_seed),
            ...mints.map(mint => ix.registerNftInstruction(userPayer.publicKey, mint)),
            ...mints.map(mint => ix.claimInstruction(userPayer.publicKey, mint, claim_seed)),
        ],
    }, luts);
}

export async function delegateAndLock(connection: Connection, userPayer: Keypair, mints: PublicKey[], luts?: AddressLookupTableAccount[]) {
    return await sendAndConfirmTransaction(connection, [userPayer], {
        payerKey: userPayer.publicKey,
        instructions: [
            ...mints.map(mint => ix.delegateApproveInstruction(mint, userPayer.publicKey, userPayer.publicKey, userPayer.publicKey)),
            ...mints.map(mint => ix.lockInstruction(mint, userPayer.publicKey, userPayer.publicKey, userPayer.publicKey)),
        ],
    }, luts);
}

export async function revokeAndUnlock(connection: Connection, userPayer: Keypair, mints: PublicKey[], luts?: AddressLookupTableAccount[]) {
    return await sendAndConfirmTransaction(connection, [userPayer], {
        payerKey: userPayer.publicKey,
        instructions: [
            ...mints.map(mint => ix.unlockInstruction(mint, userPayer.publicKey, userPayer.publicKey, userPayer.publicKey)),
            ...mints.map(mint => ix.delegateRevokeInstruction(mint, userPayer.publicKey, userPayer.publicKey, userPayer.publicKey)),
        ],
    });
}
