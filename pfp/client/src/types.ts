import {
    PublicKey,
} from '@solana/web3.js';

export enum TokenRecordState {
    Unlocked,
    Locked,
    Listed,
};

export enum DelegateRole {
    Sale,
    Transfer,
    Utility,
    Staking,
    Standard,
    LockedTransfer,
    Migration = 255,
}

export type TokenRecord = {
    // ...
    state: TokenRecordState,
    delegate: null | PublicKey,
    delegateRole: null | DelegateRole,
    // ...
};

export type ConfigRecord = {
    authority: PublicKey;
    creator: PublicKey;
    claimable_from: number;
    accumulated_reward: number;
    initial_reward: number;
    accumulation_duration: number;
    generation_duration: number;
};

export type ConfigArgs = {
    claimable_from: number;
    accumulated_reward: number;
    initial_reward: number;
    total_accumulation_period: number;
    generation_duration: number;
}

export type NftRecord = {
    claimed_amount: number;
    total_amount: number;
    last_claimed_at: number;
};

export type ClaimRecord = {
    generation: number;
    amount: number;
    owner: PublicKey;
    bnb_chain_wallet_address: string;
};

export type MintRequest = {
    name: string,
    uri: string,
}
