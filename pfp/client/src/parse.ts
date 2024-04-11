import {
    Connection,
    AccountInfo,
    PublicKey,
} from '@solana/web3.js';

import {
    TokenRecord,
    ConfigRecord,
    NftRecord,
    ClaimRecord,
} from './types';

const bs58 = require("bs58");

function getAccountInfo(connection: Connection, acc: PublicKey): Promise<AccountInfo<Buffer>> {
    return connection.getAccountInfo(acc).then(resp => resp == null
        ? Promise.reject("Uninitialized account")
        : resp);
}

export function parseTokenRecord(connection: Connection, acc: PublicKey): Promise<TokenRecord> {
    return getAccountInfo(connection, acc).then(resp => {
        let offset = 2;
        const state = resp.data.readInt8(offset++);
        offset += resp.data.readInt8(offset) == 0 ? 1 : 9; // skip over optional field (rule set revision)
        const delegate = resp.data.readInt8(offset) == 0 ? null : bs58.encode(resp.data.slice(offset + 1, offset + 33));
        const delegateRole = delegate == null ? null : resp.data.readInt8(offset + 34);

        return { state, delegate, delegateRole };
    });
}

export function parseConfig(connection: Connection, acc: PublicKey): Promise<ConfigRecord> {
    return getAccountInfo(connection, acc).then(resp => {
        return {
            authority: new PublicKey(bs58.encode(resp.data.slice(0, 32))),
            creator: new PublicKey(bs58.encode(resp.data.slice(32, 64))),
            claimable_from: resp.data.readInt32LE(64),
            accumulated_reward: resp.data.readInt32LE(68),
            initial_reward: resp.data.readInt32LE(72),
            accumulation_duration: resp.data.readInt32LE(76),
            generation_duration: resp.data.readInt32LE(80),
        };
    });
}

export function parseNft(connection: Connection, acc: PublicKey): Promise<NftRecord> {
    return getAccountInfo(connection, acc).then(resp => {
        return {
            claimed_amount: resp.data.readInt32LE(0),
            total_amount: resp.data.readInt32LE(4),
            last_claimed_at: Number(resp.data.readInt32LE(8)),
        };
    });
}

export function parseClaim(connection: Connection, acc: PublicKey): Promise<ClaimRecord> {
    return getAccountInfo(connection, acc).then(resp => {
        return {
            generation: resp.data.readInt32LE(0),
            amount: resp.data.readInt32LE(4),
            owner: new PublicKey(bs58.encode(resp.data.slice(8, 40))),
            bnb_chain_wallet_address: '0x' + resp.data.toString('utf8', 40)
        };
    });
}
