use std::mem;

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use crate::utils::parse_string;

pub const BNB_CHAIN_WALLET_ADDRESS_LENGTH: usize = 40;

/// Stores global configuration options. Created once for the entire program using
/// [`crate::instruction::GaiminInstruction::Config`]
///
/// Seeds:
/// 1. Literal `"config"`
pub struct Config {
    /// Account who has the right to issue system instructions
    pub authority: Pubkey,

    /// Creator of the claimable NFTs
    pub creator: Pubkey,

    /// Starting date when claiming becomes available
    pub claimable_from: i32,

    /// Reward amount that can be given over time
    pub accumulated_reward: i32,

    /// Reward amount given for the first claim
    pub initial_reward: i32,

    /// Duration in seconds after which the reward amount of 1 may be claimed
    pub accumulation_duration: i32,

    /// Duration of a claim record generation in seconds
    pub generation_duration: i32,
}

/// Stores staking information about an NFT. Created for each NFT using
/// [`crate::instruction::GaiminInstruction::Nft`]
///
/// Seeds:
/// 1. Literal `"nft"`
/// 2. Mint address of the NFT
pub struct NftRecord {
    /// The amount that has been claimed
    pub claimed_amount: i32,

    /// Total amount that can be claimed. Equals the sum of [`Config::initial_reward`] and
    /// [`Config::accumulated_reward`]
    pub total_amount: i32,

    /// Timestamp of the last claim. Zero if no claims have been made
    pub last_claim_at: i32,
}

/// Stores information about a claim. Created for each claim using
/// [`crate::instruction::GaiminInstruction::Claim`]. Multiple claim instructions in a single
/// transaction should use the same claim record. A claim record must be finalized by sending a
/// [`crate::instruction::GaiminInstruction::BumpNonce`] instruction after all the claim
/// instructions in a transaction
///
/// Seeds:
/// 1. Literal `"claim"`
/// 2. User's wallet account address
/// 3. Random value
pub struct ClaimRecord {
    /// Specifies the time bucket when the record was created
    pub generation: i32,

    /// The reward amount claimed
    pub amount: i32,

    /// Wallet address of the user who created the record
    pub owner: Pubkey,

    /// BNB Chain wallet address where the reward should be sent
    pub bnb_chain_wallet_address: String,
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
    const LEN: usize = 2 * 32 + 5 * 4;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Config::LEN];
        let (
            authority,
            creator,
            claimable_from,
            total_reward,
            initial_reward,
            reward_period_sec,
            generation_duration,
        ) = array_refs![
            src,
            mem::size_of::<Pubkey>(),
            mem::size_of::<Pubkey>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        Ok(Config {
            authority: Pubkey::from(*authority),
            creator: Pubkey::from(*creator),
            claimable_from: i32::from_le_bytes(*claimable_from),
            accumulated_reward: i32::from_le_bytes(*total_reward),
            initial_reward: i32::from_le_bytes(*initial_reward),
            accumulation_duration: i32::from_le_bytes(*reward_period_sec),
            generation_duration: i32::from_le_bytes(*generation_duration),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Config::LEN];
        let (
            authority,
            creator,
            claimable_from,
            total_reward,
            initial_reward,
            reward_period_sec,
            generation_duration,
        ) = mut_array_refs![
            dst,
            mem::size_of::<Pubkey>(),
            mem::size_of::<Pubkey>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        authority.copy_from_slice(&self.authority.to_bytes());
        creator.copy_from_slice(&self.creator.to_bytes());
        *claimable_from = self.claimable_from.to_le_bytes();
        *total_reward = self.accumulated_reward.to_le_bytes();
        *initial_reward = self.initial_reward.to_le_bytes();
        *reward_period_sec = self.accumulation_duration.to_le_bytes();
        *generation_duration = self.generation_duration.to_le_bytes();
    }
}

impl Pack for NftRecord {
    const LEN: usize = 3 * 4;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, NftRecord::LEN];
        let (claimed_amount, total_amount, last_claim_at) = array_refs![
            src,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        Ok(NftRecord {
            claimed_amount: i32::from_le_bytes(*claimed_amount),
            total_amount: i32::from_le_bytes(*total_amount),
            last_claim_at: i32::from_le_bytes(*last_claim_at),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, NftRecord::LEN];
        let (claimed_amount, total_amount, last_claim_at) = mut_array_refs![
            dst,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        *claimed_amount = self.claimed_amount.to_le_bytes();
        *total_amount = self.total_amount.to_le_bytes();
        *last_claim_at = self.last_claim_at.to_le_bytes();
    }
}

impl Pack for ClaimRecord {
    const LEN: usize = 2 * 4 + 32 + BNB_CHAIN_WALLET_ADDRESS_LENGTH;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, ClaimRecord::LEN];
        let (generation, amount, owner, bnb_chain_wallet_address) = array_refs![
            src,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<Pubkey>(),
            BNB_CHAIN_WALLET_ADDRESS_LENGTH
        ];

        Ok(ClaimRecord {
            generation: i32::from_le_bytes(*generation),
            amount: i32::from_le_bytes(*amount),
            owner: Pubkey::from(*owner),
            bnb_chain_wallet_address: parse_string(bnb_chain_wallet_address)?,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ClaimRecord::LEN];
        let (generation, claimed_amount, owner, bnb_chain_wallet_address) = mut_array_refs![
            dst,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<Pubkey>(),
            BNB_CHAIN_WALLET_ADDRESS_LENGTH
        ];

        *generation = self.generation.to_le_bytes();
        *claimed_amount = self.amount.to_le_bytes();
        owner.copy_from_slice(&self.owner.to_bytes());
        bnb_chain_wallet_address.copy_from_slice(&self.bnb_chain_wallet_address.as_bytes());
    }
}
