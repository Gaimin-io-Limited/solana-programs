import {
    PublicKey,
} from '@solana/web3.js';

import * as PID from './pid';

// https://docs.metaplex.com/programs/token-metadata/accounts#metadata
export function findMetadataAccountPda(mint: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('metadata'),
            PID.MPL_TOKEN_METADATA.toBuffer(),
            mint.toBuffer()
        ],
        PID.MPL_TOKEN_METADATA
    );
}

// https://docs.metaplex.com/programs/token-metadata/accounts#master-edition
export function findMasterEditionAccountPda(mint: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('metadata'),
            PID.MPL_TOKEN_METADATA.toBuffer(),
            mint.toBuffer(),
            Buffer.from('edition'),
        ],
        PID.MPL_TOKEN_METADATA
    );
}

// https://spl.solana.com/associated-token-account
export function findTokenAccountPda(mint: PublicKey, wallet: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            wallet.toBuffer(),
            PID.TOKEN.toBuffer(), 
            mint.toBuffer()
        ],
        PID.SPL_ASSOCIATED_TOKEN_ACCOUNT
    );
}

// https://docs.metaplex.com/programs/token-metadata/accounts#token-record
export function findTokenRecordPda(mint: PublicKey, token: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('metadata'),
            PID.MPL_TOKEN_METADATA.toBuffer(),
            mint.toBuffer(),
            Buffer.from('token_record'),
            token.toBuffer(),
        ],
        PID.MPL_TOKEN_METADATA
    );
}

export function findConfigPda(): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('config'),
        ],
        PID.GAIMIN_PFP
    );
}

export function findNftPda(mint: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('nft'),
            mint.toBuffer(),
        ],
        PID.GAIMIN_PFP
    );
}

export function findClaimPda(wallet: PublicKey, seed: Buffer): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from('claim'),
            wallet.toBuffer(),
            seed,
        ],
        PID.GAIMIN_PFP
    );
}
