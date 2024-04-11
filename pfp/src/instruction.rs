use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use shank::ShankContext;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::{fmt::Debug, mem};

use crate::{
    error::GaiminError,
    state::BNB_CHAIN_WALLET_ADDRESS_LENGTH,
    utils::{assert_ix_data_length, parse_string},
};

/// Instructions supported by the GMRX Claim Program
#[derive(Debug, ShankContext)]
pub enum GaiminInstruction {
    /// Instruction code: `0x0`
    ///
    /// Create and initialize the config account. It is a system instruction that can be executed
    /// by anyone. The config account may NOT be initialized. The `authority` account will pay for
    /// rent exemption and become the config authority with the exclusive right to execute other
    /// system instructions. To update the config account, it must first be deleted and then
    /// created again.
    #[account(0, signer, name = "authority", desc = "Config authority/Rent payer")]
    #[account(1, name = "creator", desc = "Creator of claimable NFTs")]
    #[account(2, writable, name = "config", desc = "Config PDA")]
    #[account(3, name = "system_program", desc = "System program")]
    Config(ConfigArgs),

    /// Instruction code: `0x1`
    ///
    /// Delete an account owned by this program. It is a system instruction that must be signed by
    /// the config authority. The config account must be initialized first. The account to be
    /// deleted must also be initialized.
    #[account(0, signer, name = "authority", desc = "Config authority")]
    #[account(1, writable, name = "target", desc = "Account to be deleted")]
    #[account(2, writable, name = "receiver", desc = "Account to send lamports to")]
    #[account(3, name = "config", desc = "Config PDA")]
    Delete,

    /// Instruction code: `0x2`
    ///
    /// Create and initialize an NFT record account. The config account must be initialized first.
    /// The NFT record account may NOT be initialized. To update an existing NFT record, delete it
    /// and create again. The NFT being registered must be valid and of the programmable standard.
    /// The config authority will pay for rent exemption.
    #[account(0, signer, name = "payer", desc = "Rent payer")]
    #[account(1, name = "mint", desc = "NFT mint account")]
    #[account(2, name = "metadata", desc = "NFT metadata account")]
    #[account(3, name = "edition", desc = "NFT edition account")]
    #[account(4, writable, name = "nft_record", desc = "NFT record PDA")]
    #[account(5, name = "config", desc = "Config PDA")]
    #[account(6, name = "system_program", desc = "System program")]
    Nft,

    /// Instruction code: `0x3`
    ///
    /// Create an initial claim record. Claim record account must not be initialized. The user will
    /// pay for rent exemption.
    #[account(0, signer, name = "wallet", desc = "User wallet account/Rent payer")]
    #[account(1, writable, name = "claim", desc = "Claim record PDA")]
    #[account(2, name = "config", desc = "Config PDA")]
    #[account(3, name = "system_program", desc = "System program")]
    CreateClaim(CreateClaimArgs),

    /// Instruction code: `0x4`
    ///
    /// Add a reward for the given NFT to a provided claim record. It is a user instruction and
    /// must be signed with the user's wallet account key. The user must own the NFT and the token
    /// account must be locked. All accounts must be initialized.
    #[account(0, signer, name = "wallet", desc = "User wallet account/Rent payer")]
    #[account(1, name = "token", desc = "Token account")]
    #[account(2, name = "token_record", desc = "Token record account")]
    #[account(3, writable, name = "nft_record", desc = "NFT record PDA")]
    #[account(4, writable, name = "claim", desc = "Claim record PDA")]
    #[account(5, name = "config", desc = "Config PDA")]
    Claim(ClaimArgs),
}

impl GaiminInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => Self::Config(ConfigArgs::unpack_from_slice(rest)?),
            1 => Self::Delete,
            2 => Self::Nft,
            3 => Self::CreateClaim(CreateClaimArgs::unpack_from_slice(rest)?),
            4 => Self::Claim(ClaimArgs::unpack_from_slice(rest)?),
            i => {
                msg!("[Error] Invalid instruction code: {}", i);
                return Err(GaiminError::InvalidInstruction.into());
            }
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        match self {
            Self::Config(args) => {
                let mut res = vec![0; ConfigArgs::LEN + 1];
                args.pack_into_slice(&mut res[1..]);
                res
            }
            Self::Delete => vec![1],
            Self::Nft => vec![2],
            Self::CreateClaim(args) => {
                let mut res = vec![3; CreateClaimArgs::LEN + 1];
                args.pack_into_slice(&mut res[1..]);
                res
            }
            Self::Claim(args) => {
                let mut res = vec![4; ClaimArgs::LEN + 1];
                args.pack_into_slice(&mut res[1..]);
                res
            }
        }
    }
}

#[derive(Debug)]
pub struct ConfigArgs {
    pub claimable_from: i32,
    pub accumulated_reward: i32,
    pub initial_reward: i32,
    pub total_accumulation_period: i32,
    pub generation_duration: i32,
}

impl Sealed for ConfigArgs {}
impl IsInitialized for ConfigArgs {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for ConfigArgs {
    const LEN: usize = 5 * 4;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        assert_ix_data_length(input, ConfigArgs::LEN)?;
        let src = array_ref![input, 0, ConfigArgs::LEN];
        let (claimable_from, total_reward, initial_reward, reward_period_sec, generation_duration) = array_refs![
            src,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        Ok(Self {
            claimable_from: i32::from_le_bytes(*claimable_from),
            accumulated_reward: i32::from_le_bytes(*total_reward),
            initial_reward: i32::from_le_bytes(*initial_reward),
            total_accumulation_period: i32::from_le_bytes(*reward_period_sec),
            generation_duration: i32::from_le_bytes(*generation_duration),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ConfigArgs::LEN];
        let (claimable_from, total_reward, initial_reward, reward_period_sec, generation_duration) = mut_array_refs![
            dst,
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>(),
            mem::size_of::<i32>()
        ];

        *claimable_from = self.claimable_from.to_le_bytes();
        *total_reward = self.accumulated_reward.to_le_bytes();
        *initial_reward = self.initial_reward.to_le_bytes();
        *reward_period_sec = self.total_accumulation_period.to_le_bytes();
        *generation_duration = self.generation_duration.to_le_bytes();
    }
}

const CLAIM_SEED_LENGTH: usize = 32;

#[derive(Debug)]
pub struct CreateClaimArgs {
    pub bump: u8,
    pub seed: [u8; CLAIM_SEED_LENGTH],
    pub bnb_chain_wallet_address: String,
}

impl Sealed for CreateClaimArgs {}
impl IsInitialized for CreateClaimArgs {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for CreateClaimArgs {
    const LEN: usize = 1 + CLAIM_SEED_LENGTH + BNB_CHAIN_WALLET_ADDRESS_LENGTH;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        assert_ix_data_length(input, CreateClaimArgs::LEN)?;
        let src = array_ref![input, 0, CreateClaimArgs::LEN];
        let (bump, seed, bnb_chain_wallet_address) =
            array_refs![src, 1, CLAIM_SEED_LENGTH, BNB_CHAIN_WALLET_ADDRESS_LENGTH];
        Ok(CreateClaimArgs {
            bump: u8::from_le_bytes(*bump),
            seed: *seed,
            bnb_chain_wallet_address: parse_string(bnb_chain_wallet_address)?,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, CreateClaimArgs::LEN];
        let (bump, seed, bnb_chain_wallet_address) =
            mut_array_refs![dst, 1, CLAIM_SEED_LENGTH, BNB_CHAIN_WALLET_ADDRESS_LENGTH];
        *bump = self.bump.to_le_bytes();
        *seed = self.seed;
        bnb_chain_wallet_address.copy_from_slice(self.bnb_chain_wallet_address.as_bytes());
    }
}

#[derive(Debug)]
pub struct ClaimArgs {
    pub token_acc_bump: u8,
    pub token_record_bump: u8,
    pub nft_record_bump: u8,
}

impl Sealed for ClaimArgs {}
impl IsInitialized for ClaimArgs {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Pack for ClaimArgs {
    const LEN: usize = 3;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        assert_ix_data_length(input, ClaimArgs::LEN)?;

        Ok(ClaimArgs {
            token_acc_bump: input[0],
            token_record_bump: input[1],
            nft_record_bump: input[2],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        dst[0] = self.token_acc_bump;
        dst[1] = self.token_record_bump;
        dst[2] = self.nft_record_bump;
    }
}
