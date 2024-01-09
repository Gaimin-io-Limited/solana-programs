use std::{fmt::Debug, mem};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::UnixTimestamp,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

// Seeds: "config"
pub struct Config {
    pub authority: Pubkey,
    pub claimable_from: UnixTimestamp,
    pub total_reward: f64,
    pub initial_reward_frac: f32,
    pub reward_period_sec: i32,
}

// Seeds: NFT address
#[derive(Debug)]
pub struct NftRecord {
    pub nonce: i32,
    pub claimed_amount: f64,
    pub total_amount: f64,
    pub last_claim_at: UnixTimestamp,
}

// Seeds: NFT address + Nonce
#[derive(Debug)]
pub struct ClaimRecord {
    pub claimed_amount: f64,
    pub bnb_chain_wallet_address: [u8; 40],
}

impl Sealed for Config {}
impl Sealed for NftRecord {}
impl Sealed for ClaimRecord {}

impl IsInitialized for Config {
    fn is_initialized(&self) -> bool {
        self.authority.to_bytes().iter().any(|&x| x != 0)
    }
}

impl IsInitialized for NftRecord {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl IsInitialized for ClaimRecord {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for Config {
    const LEN: usize = mem::size_of::<Pubkey>()
        + mem::size_of::<UnixTimestamp>()
        + mem::size_of::<f64>()
        + mem::size_of::<f32>()
        + mem::size_of::<i32>();

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Config::LEN];
        let (
            authority,
            claimable_from,
            total_reward,
            initial_reward_frac,
            reward_period_sec,
        ) = array_refs![
            src,
            mem::size_of::<Pubkey>(),
            mem::size_of::<UnixTimestamp>(),
            mem::size_of::<f64>(),
            mem::size_of::<f32>(),
            mem::size_of::<i32>()
        ];

        Ok(Config {
            authority: Pubkey::from(*authority),
            claimable_from: i64::from_le_bytes(*claimable_from),
            total_reward: f64::from_le_bytes(*total_reward),
            initial_reward_frac: f32::from_le_bytes(*initial_reward_frac),
            reward_period_sec: i32::from_le_bytes(*reward_period_sec),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Config::LEN];
        let (
            authority,
            claimable_from,
            total_reward,
            initial_reward_frac,
            reward_period_sec,
        ) = mut_array_refs![
            dst,
            mem::size_of::<Pubkey>(),
            mem::size_of::<UnixTimestamp>(),
            mem::size_of::<f64>(),
            mem::size_of::<f32>(),
            mem::size_of::<i32>()
        ];

        authority.copy_from_slice(&self.authority.to_bytes());
        *claimable_from = self.claimable_from.to_le_bytes();
        *total_reward = self.total_reward.to_le_bytes();
        *initial_reward_frac = self.initial_reward_frac.to_le_bytes();
        *reward_period_sec = self.reward_period_sec.to_le_bytes();
    }
}

impl Pack for NftRecord {
    const LEN: usize = mem::size_of::<i32>()
        + mem::size_of::<f64>()
        + mem::size_of::<f64>()
        + mem::size_of::<UnixTimestamp>();

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, NftRecord::LEN];
        let (nonce, claimed_amount, total_amount, last_claim_at) = array_refs![
            src,
            mem::size_of::<i32>(),
            mem::size_of::<f64>(),
            mem::size_of::<f64>(),
            mem::size_of::<UnixTimestamp>()
        ];

        Ok(NftRecord {
            nonce: i32::from_le_bytes(*nonce),
            claimed_amount: f64::from_le_bytes(*claimed_amount),
            total_amount: f64::from_le_bytes(*total_amount),
            last_claim_at: i64::from_le_bytes(*last_claim_at),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, NftRecord::LEN];
        let (nonce, claimed_amount, total_amount, last_claim_at) = mut_array_refs![
            dst,
            mem::size_of::<i32>(),
            mem::size_of::<f64>(),
            mem::size_of::<f64>(),
            mem::size_of::<UnixTimestamp>()
        ];

        *nonce = self.nonce.to_le_bytes();
        *claimed_amount = self.claimed_amount.to_le_bytes();
        *total_amount = self.total_amount.to_le_bytes();
        *last_claim_at = self.last_claim_at.to_le_bytes();
    }
}

impl Pack for ClaimRecord {
    const LEN: usize = mem::size_of::<f64>() + mem::size_of::<[u8; 40]>();

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, ClaimRecord::LEN];
        let (claimed_amount, bnb_chain_wallet_address) =
            array_refs![src, mem::size_of::<f64>(), mem::size_of::<[u8; 40]>()];

        Ok(ClaimRecord {
            claimed_amount: f64::from_le_bytes(*claimed_amount),
            bnb_chain_wallet_address: *bnb_chain_wallet_address,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ClaimRecord::LEN];
        let (claimed_amount, bnb_chain_wallet_address) =
            mut_array_refs![dst, mem::size_of::<f64>(), mem::size_of::<[u8; 40]>()];

        *claimed_amount = self.claimed_amount.to_le_bytes();
        bnb_chain_wallet_address.copy_from_slice(&self.bnb_chain_wallet_address);
    }
}
