use std::mem;
use arrayref::{array_ref, array_refs};
use solana_program::{clock::UnixTimestamp, program_error::ProgramError, msg};

use crate::error::GaiminError;

#[derive(Debug)]
pub enum GaiminInstruction {
    // Accounts:
    // 0. Signer + Authority
    // 1. Config PDA
    SetConfig {
        claimable_from: UnixTimestamp,
        total_reward: f64,
        initial_reward_frac: f32,
        reward_period_sec: i32,
    },

    // Accounts:
    // 0. Signer + Fee payer
    // 1. NFT
    // 2. NFT PDA
    // 3. Config PDA
    RegisterNft,

    // Account:
    // 0. User (Signer)
    // 1. NFT
    // 2. User Token Account
    // 4. NFT PDA
    // 6. Claim PDA
    // 8. Config PDA
    ClaimReward {
        bnb_chain_wallet_address: [u8; 40],
    },
}

impl GaiminInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => Self::unpack_config(rest)?,
            1 => Self::RegisterNft,
            2 => Self::ClaimReward {
                bnb_chain_wallet_address: Self::unpack_bnb_chain_wallet_address(rest)?,
            },
            i => {
                msg!("[Error] Invalid Instruction: `{}`", i);
                return Err(GaiminError::InvalidInstruction.into())
            }
        })
    }

    fn unpack_bnb_chain_wallet_address(input: &[u8]) -> Result<[u8; 40], ProgramError> {
        input
            .get(..40)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(ProgramError::InvalidInstructionData)
    }

    fn unpack_config(input: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![
            input,
            0,
            mem::size_of::<UnixTimestamp>()
                + mem::size_of::<f64>()
                + mem::size_of::<f32>()
                + mem::size_of::<i32>()
        ];
        let (claimable_from, total_reward, initial_reward_frac, reward_period_sec) = array_refs![
            src,
            mem::size_of::<UnixTimestamp>(),
            mem::size_of::<f64>(),
            mem::size_of::<f32>(),
            mem::size_of::<i32>()
        ];

        Ok(Self::SetConfig {
            claimable_from: i64::from_le_bytes(*claimable_from),
            total_reward: f64::from_le_bytes(*total_reward),
            initial_reward_frac: f32::from_le_bytes(*initial_reward_frac),
            reward_period_sec: i32::from_le_bytes(*reward_period_sec),
        })
    }
}
