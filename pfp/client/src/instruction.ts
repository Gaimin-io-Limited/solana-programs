import {
    Connection,
    PublicKey,
    SystemProgram,
    TransactionInstruction,
    SYSVAR_INSTRUCTIONS_PUBKEY, 
} from '@solana/web3.js';

import * as pda from './pda';

import {
    ConfigArgs,
} from './types';

import * as PID from './pid';

const GAIMIN_PFP_INSTRUCTIONS = {
    CONFIG: 0,
    DELETE: 1,
    NFT: 2,
    CREATE_CLAIM: 3,
    CLAIM: 4,
};

const MPL_INSTRUCTIONS = {
    delegate: 44,
    revoke: 45,
    lock: 46,
    unlock: 47,
};

const MPL_DELEGATE_TYPES = {
    staking: 5,
};

// Some accounts in mpl token metadata instructions are optional, use this account to omit them
const MPL_EMPTY_ACCOUNT = {pubkey: PID.MPL_TOKEN_METADATA, isSigner: false, isWritable: false};

export function setConfigInstruction(signer: PublicKey, creator: PublicKey, config: ConfigArgs): TransactionInstruction {
    const data = Buffer.allocUnsafe(1 + 5 * 4);
    data.writeInt8(GAIMIN_PFP_INSTRUCTIONS.CONFIG);
    data.writeInt32LE(config.claimable_from, 1);
    data.writeInt32LE(config.accumulated_reward, 5);
    data.writeInt32LE(config.initial_reward, 9);
    data.writeInt32LE(config.total_accumulation_period, 13);
    data.writeInt32LE(config.generation_duration, 17);

    return new TransactionInstruction({
        data,
        keys: [
            {pubkey: signer, isSigner: true, isWritable: false},
            {pubkey: creator, isSigner: false, isWritable: false},
            {pubkey: pda.findConfigPda()[0], isSigner: false, isWritable: true},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: PID.GAIMIN_PFP,
    });
}

export function deleteAccountInstruction(signer: PublicKey, acc: PublicKey): TransactionInstruction {
    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(GAIMIN_PFP_INSTRUCTIONS.DELETE)),
        keys: [
            {pubkey: signer, isSigner: true, isWritable: false},
            {pubkey: acc, isSigner: false, isWritable: true},
            {pubkey: signer, isSigner: false, isWritable: true},
            {pubkey: pda.findConfigPda()[0], isSigner: false, isWritable: false},
        ],
        programId: PID.GAIMIN_PFP,
    });
}

export function registerNftInstruction(signer: PublicKey, mint: PublicKey): TransactionInstruction {
    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(GAIMIN_PFP_INSTRUCTIONS.NFT)),
        keys: [
            {pubkey: signer, isSigner: true, isWritable: false},
            {pubkey: mint, isSigner: false, isWritable: false},
            {pubkey: pda.findMetadataAccountPda(mint)[0], isSigner: false, isWritable: false},
            {pubkey: pda.findMasterEditionAccountPda(mint)[0], isSigner: false, isWritable: false},
            {pubkey: pda.findNftPda(mint)[0], isSigner: false, isWritable: true},
            {pubkey: pda.findConfigPda()[0], isSigner: false, isWritable: false},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: PID.GAIMIN_PFP,
    });
}

export function createClaimInstruction(wallet: PublicKey, bnbWallet: string, claim_seed: Buffer): TransactionInstruction {
    const enc = new TextEncoder();
    const [claim, bump] = pda.findClaimPda(wallet, claim_seed);

    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(GAIMIN_PFP_INSTRUCTIONS.CREATE_CLAIM, bump, ...claim_seed, ...enc.encode(bnbWallet.slice(2)))),
        keys: [
            {pubkey: wallet, isSigner: true, isWritable: false},
            {pubkey: claim, isSigner: false, isWritable: true},
            {pubkey: pda.findConfigPda()[0], isSigner: false, isWritable: false},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: PID.GAIMIN_PFP,
    });
}

export function claimInstruction(wallet: PublicKey, mint: PublicKey, claim_seed: Buffer): TransactionInstruction {
    const [nft_record, nft_bump] = pda.findNftPda(mint);
    const [token, token_bump] = pda.findTokenAccountPda(mint, wallet);
    const config = pda.findConfigPda()[0];
    const [token_record, token_record_bump] = pda.findTokenRecordPda(mint, token);
    const claim = pda.findClaimPda(wallet, claim_seed)[0];

    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(
            GAIMIN_PFP_INSTRUCTIONS.CLAIM,
            token_bump,
            token_record_bump,
            nft_bump,
        )),
        keys: [
            {pubkey: wallet, isSigner: true, isWritable: false},
            {pubkey: token, isSigner: false, isWritable: false},
            {pubkey: token_record, isSigner: false, isWritable: false},
            {pubkey: nft_record, isSigner: false, isWritable: true},
            {pubkey: claim, isSigner: false, isWritable: true},
            {pubkey: config, isSigner: false, isWritable: false},
        ],
        programId: PID.GAIMIN_PFP,
    });
}

export function delegateApproveInstruction(mint: PublicKey, wallet: PublicKey, payer: PublicKey, delegate: PublicKey): TransactionInstruction {
    const data = Buffer.allocUnsafe(11);
    data.writeInt8(MPL_INSTRUCTIONS.delegate);
    data.writeInt8(MPL_DELEGATE_TYPES.staking, 1);
    data.writeBigInt64LE(1n, 2);
    data.writeInt8(0, 10);

    const token_acc = pda.findTokenAccountPda(mint, wallet)[0];
    const metadata_acc = pda.findMetadataAccountPda(mint)[0];
    const master_edition_acc = pda.findMasterEditionAccountPda(mint)[0];
    const token_rec = pda.findTokenRecordPda(mint, token_acc)[0];

    return new TransactionInstruction({
        data,
        keys: [
            MPL_EMPTY_ACCOUNT,
            {pubkey: delegate, isSigner: false, isWritable: false},

            {pubkey: metadata_acc, isSigner: false, isWritable: true},
            {pubkey: master_edition_acc, isSigner: false, isWritable: false},
            {pubkey: token_rec, isSigner: false, isWritable: true},
            {pubkey: mint, isSigner: false, isWritable: false},
            {pubkey: token_acc, isSigner: false, isWritable: true},

            {pubkey: wallet, isSigner: true, isWritable: false},
            {pubkey: payer, isSigner: true, isWritable: false},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
            {pubkey: PID.TOKEN, isSigner: false, isWritable: false },
            MPL_EMPTY_ACCOUNT,
            MPL_EMPTY_ACCOUNT,
        ],
        programId: PID.MPL_TOKEN_METADATA,
    });
}

export function delegateRevokeInstruction(mint: PublicKey, wallet: PublicKey, payer: PublicKey, delegate: PublicKey): TransactionInstruction {
    const token_acc = pda.findTokenAccountPda(mint, wallet)[0];
    const metadata_acc = pda.findMetadataAccountPda(mint)[0];
    const master_edition_acc = pda.findMasterEditionAccountPda(mint)[0];
    const token_rec = pda.findTokenRecordPda(mint, token_acc)[0];

    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(MPL_INSTRUCTIONS.revoke, MPL_DELEGATE_TYPES.staking)),
        keys: [
            MPL_EMPTY_ACCOUNT,
            {pubkey: delegate, isSigner: false, isWritable: false},

            {pubkey: metadata_acc, isSigner: false, isWritable: true},
            {pubkey: master_edition_acc, isSigner: false, isWritable: false},
            {pubkey: token_rec, isSigner: false, isWritable: true},
            {pubkey: mint, isSigner: false, isWritable: false},
            {pubkey: token_acc, isSigner: false, isWritable: true},

            {pubkey: wallet, isSigner: true, isWritable: false},
            {pubkey: payer, isSigner: true, isWritable: false},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
            {pubkey: PID.TOKEN, isSigner: false, isWritable: false },
            MPL_EMPTY_ACCOUNT,
            MPL_EMPTY_ACCOUNT,
        ],
        programId: PID.MPL_TOKEN_METADATA,
    });
}

export function lockInstruction(mint: PublicKey, wallet: PublicKey, payer: PublicKey, delegate: PublicKey): TransactionInstruction {
    const token_acc = pda.findTokenAccountPda(mint, wallet)[0];
    const metadata_acc = pda.findMetadataAccountPda(mint)[0];
    const master_edition_acc = pda.findMasterEditionAccountPda(mint)[0];
    const token_rec = pda.findTokenRecordPda(mint, token_acc)[0];

    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(MPL_INSTRUCTIONS.lock, 0, 0)),
        keys: [
            {pubkey: delegate, isSigner: true, isWritable: false},
            MPL_EMPTY_ACCOUNT,

            {pubkey: token_acc, isSigner: false, isWritable: true},
            {pubkey: mint, isSigner: false, isWritable: false},
            {pubkey: metadata_acc, isSigner: false, isWritable: true},
            {pubkey: master_edition_acc, isSigner: false, isWritable: false},
            {pubkey: token_rec, isSigner: false, isWritable: true},

            {pubkey: payer, isSigner: true, isWritable: true},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
            {pubkey: PID.TOKEN, isSigner: false, isWritable: false },
            MPL_EMPTY_ACCOUNT,
            MPL_EMPTY_ACCOUNT,
        ],
        programId: PID.MPL_TOKEN_METADATA,
    });
}

export function unlockInstruction(mint: PublicKey, wallet: PublicKey, payer: PublicKey, delegate: PublicKey): TransactionInstruction {
    const token_acc = pda.findTokenAccountPda(mint, wallet)[0];
    const metadata_acc = pda.findMetadataAccountPda(mint)[0];
    const master_edition_acc = pda.findMasterEditionAccountPda(mint)[0];
    const token_rec = pda.findTokenRecordPda(mint, token_acc)[0];

    return new TransactionInstruction({
        data: Buffer.from(Uint8Array.of(MPL_INSTRUCTIONS.unlock, 0, 0)),
        keys: [
            {pubkey: delegate, isSigner: true, isWritable: false},
            MPL_EMPTY_ACCOUNT,

            {pubkey: token_acc, isSigner: false, isWritable: true},
            {pubkey: mint, isSigner: false, isWritable: false},
            {pubkey: metadata_acc, isSigner: false, isWritable: true},
            {pubkey: master_edition_acc, isSigner: false, isWritable: false},
            {pubkey: token_rec, isSigner: false, isWritable: true},

            {pubkey: payer, isSigner: true, isWritable: true},
            {pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            {pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
            {pubkey: PID.TOKEN, isSigner: false, isWritable: false },
            MPL_EMPTY_ACCOUNT,
            MPL_EMPTY_ACCOUNT,
        ],
        programId: PID.MPL_TOKEN_METADATA,
    });
}
