use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::{Clock, UnixTimestamp},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::{Account, Mint};

use crate::{
    instruction::GaiminInstruction,
    state::{ClaimRecord, Config, NftRecord}, error::GaiminError,
};

const CONFIG_PDA_SEED: &[u8] = b"config";
const NFT_PDA_SEED: &[u8] = b"nft";
const CLAIM_PDA_SEED: &[u8] = b"claim";

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = GaiminInstruction::unpack(instruction_data)?;

        msg!("Processing instruction: {:?}", instruction);
        match instruction {
            GaiminInstruction::SetConfig {
                claimable_from,
                total_reward,
                initial_reward_frac,
                reward_period_sec,
            } => Self::set_config(
                program_id,
                accounts,
                claimable_from,
                total_reward,
                initial_reward_frac,
                reward_period_sec,
            ),
            GaiminInstruction::RegisterNft => Self::register_nft(program_id, accounts),
            GaiminInstruction::ClaimReward {
                bnb_chain_wallet_address,
            } => Self::claim_reward(program_id, accounts, &bnb_chain_wallet_address),
        }
    }

    fn set_config(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        claimable_from: UnixTimestamp,
        total_reward: f64,
        initial_reward_frac: f32,
        reward_period_sec: i32,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let auth_acc = next_account_info(account_info_iter).filter(
            |acc| acc.is_signer,
            |_| ProgramError::MissingRequiredSignature,
        )?;

        let (config_pda_acc, config_bump) = Self::validate_pda(
            next_account_info(account_info_iter)?,
            &[CONFIG_PDA_SEED],
            program_id,
        )?;

        if **config_pda_acc.try_borrow_lamports()? == 0 {
            msg!("Creating config account");
            invoke_signed(
                &solana_program::system_instruction::create_account(
                    auth_acc.key,
                    config_pda_acc.key,
                    Rent::default().minimum_balance(Config::LEN),
                    Config::LEN as u64,
                    program_id,
                ),
                &[auth_acc.clone(), config_pda_acc.clone()],
                &[&[CONFIG_PDA_SEED, &[config_bump]]],
            )?;
        }

        Config::unpack_unchecked(&config_pda_acc.try_borrow_data()?).filter(
            |config| !config.is_initialized() || &config.authority == auth_acc.key,
            |config| {
                msg!("[Error] Permission to edit initialized config denied. Expected authority {:?}, received {:?}", config.authority, auth_acc.key);
                GaiminError::PermissionDenied.into()
            }
        )?;

        Config::pack(
            Config {
                authority: *auth_acc.key,
                claimable_from,
                total_reward,
                initial_reward_frac,
                reward_period_sec,
            },
            &mut config_pda_acc.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn register_nft(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let auth_acc = next_account_info(account_info_iter).filter(
            |acc| acc.is_signer,
            |_| ProgramError::MissingRequiredSignature,
        )?;

        let nft_acc = next_account_info(account_info_iter)
            .filter(
                |acc| *acc.owner == spl_token::id(),
                |acc| {
                    msg!("[Error] {:?} is not a valid NFT (account owner is not the Token Program)", acc.key);
                    ProgramError::InvalidAccountOwner
                }
            )
            .and_then(|acc| {
                Mint::unpack_unchecked(&acc.try_borrow_data()?)
                    .filter(
                        |mint| {
                            mint.supply == 1 && mint.decimals == 0 && mint.mint_authority.is_none()
                        },
                        |_| {
                            msg!("[Error] {:?} is not a valid NFT (token is fungible)", acc.key);
                            ProgramError::InvalidAccountData
                        }
                    )
                    .and(Ok(acc))
            })?;

        let nft_seeds = &[NFT_PDA_SEED, &nft_acc.key.to_bytes()];
        let (nft_pda_acc, nft_bump) =
            Self::validate_pda(next_account_info(account_info_iter)?, nft_seeds, program_id)?;
        Self::ensure_uninitialized(nft_pda_acc)?;

        let (config_pda_acc, _) = Self::validate_pda(
            next_account_info(account_info_iter)?,
            &[CONFIG_PDA_SEED],
            program_id,
        )?;
        Self::ensure_initialized(config_pda_acc)?;

        let config = Config::unpack_unchecked(&config_pda_acc.try_borrow_data()?).filter(
            |config| &config.authority == auth_acc.key,
            |config| {
                msg!("[Error] Permission to register NFT denied. Expected authority {:?}, received {:?}", config.authority, auth_acc.key);
                GaiminError::PermissionDenied.into()
            }
        )?;

        msg!("Creating NFT PDA Account");
        invoke_signed(
            &solana_program::system_instruction::create_account(
                auth_acc.key,
                nft_pda_acc.key,
                Rent::default().minimum_balance(NftRecord::LEN),
                NftRecord::LEN as u64,
                program_id,
            ),
            &[auth_acc.clone(), nft_pda_acc.clone()],
            &[&[NFT_PDA_SEED, &nft_acc.key.to_bytes(), &[nft_bump]]],
        )?;

        NftRecord::pack(
            NftRecord {
                nonce: 0,
                claimed_amount: 0.0,
                total_amount: config.total_reward,
                last_claim_at: 0,
            },
            &mut nft_pda_acc.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn claim_reward(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        bnb_chain_wallet_address: &[u8; 40],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let user_acc = next_account_info(account_info_iter).filter(
            |acc| acc.is_signer,
            |_| ProgramError::MissingRequiredSignature,
        )?;

        let nft_acc = next_account_info(account_info_iter)?;

        let _token_acc = next_account_info(account_info_iter)
            .filter(
                |acc| *acc.owner == spl_token::id(),
                |acc| {
                    msg!("[Error] Account {:?} is not a token account", acc.key);
                    ProgramError::InvalidAccountOwner
                }
            )
            .and_then(|acc| {
                Account::unpack_unchecked(&acc.try_borrow_data()?)
                    .filter(
                        |token_acc| &token_acc.owner == user_acc.key,
                        |_| {
                            msg!("[Error] Token account {:?} does not belong to wallet {:?}", acc.key, user_acc.key);
                            GaiminError::PermissionDenied.into()
                        }
                    )
                    .filter(
                        |token_acc| &token_acc.mint == nft_acc.key && token_acc.amount > 0,
                        |_| {
                            msg!("[Error] Token account {:?} does not hold tokens of mint {:?}", acc.key, nft_acc.key);
                            GaiminError::PermissionDenied.into()
                        }
                    )
            })?;

        let nft_seeds = &[NFT_PDA_SEED, &nft_acc.key.to_bytes()];
        let (nft_pda_acc, _) =
            Self::validate_pda(next_account_info(account_info_iter)?, nft_seeds, program_id)?;
        Self::ensure_initialized(nft_pda_acc)?;

        let nft_record = NftRecord::unpack_unchecked(&nft_pda_acc.try_borrow_data()?)?;

        let nonce_seed = &(nft_record.nonce + 1).to_le_bytes();
        let claim_seeds = &[CLAIM_PDA_SEED, &nft_acc.key.to_bytes(), nonce_seed];
        let (claim_pda_acc, claim_bump) = Self::validate_pda(
            next_account_info(account_info_iter)?,
            claim_seeds,
            program_id,
        )?;
        Self::ensure_uninitialized(claim_pda_acc)?;

        let (config_pda_acc, _) = Self::validate_pda(
            next_account_info(account_info_iter)?,
            &[CONFIG_PDA_SEED],
            program_id,
        )?;

        let config = Config::unpack_unchecked(&config_pda_acc.try_borrow_data()?)?;
        let now = Clock::get()
            .filter(
                |now| now.unix_timestamp >= config.claimable_from,
                |now| {
                    msg!("[Error] Claiming is only available from {:?}", now.unix_timestamp);
                    GaiminError::ClaimingNotAvailable.into()
                }
            )?
            .unix_timestamp;

        invoke_signed(
            &solana_program::system_instruction::create_account(
                user_acc.key,
                claim_pda_acc.key,
                Rent::default().minimum_balance(ClaimRecord::LEN),
                ClaimRecord::LEN as u64,
                program_id,
            ),
            &[user_acc.clone(), claim_pda_acc.clone()],
            &[&[
                CLAIM_PDA_SEED,
                &nft_acc.key.to_bytes(),
                nonce_seed,
                &[claim_bump],
            ]],
        )?;

        let mut nft_record = NftRecord::unpack_unchecked(&nft_pda_acc.try_borrow_mut_data()?)
            .filter(
                |rec| rec.claimed_amount < rec.total_amount,
                |_| {
                    msg!("[Error] No claimable amount left");
                    GaiminError::AmountExhausted.into()
                }
            )?;

        let reward = if nft_record.last_claim_at == 0 {
            config.initial_reward_frac as f64 * nft_record.total_amount
        } else {
            let time_since_last_claim = now - nft_record.last_claim_at;
            let reward_per_sec = ((1.0 - config.initial_reward_frac as f64)
                * nft_record.total_amount)
                / config.reward_period_sec as f64;
            let reward_by_time = time_since_last_claim as f64 * reward_per_sec;
            f64::min(
                nft_record.total_amount - nft_record.claimed_amount,
                reward_by_time,
            )
        };

        nft_record.last_claim_at = now;
        nft_record.claimed_amount += reward;
        nft_record.nonce += 1;
        NftRecord::pack(nft_record, &mut nft_pda_acc.try_borrow_mut_data()?)?;

        ClaimRecord::pack(
            ClaimRecord {
                claimed_amount: reward,
                bnb_chain_wallet_address: *bnb_chain_wallet_address,
            },
            &mut claim_pda_acc.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn ensure_uninitialized<'a, 'b>(
        acc: &'a AccountInfo<'b>,
    ) -> Result<&'a AccountInfo<'b>, ProgramError> {
        acc.try_borrow_lamports()
            .filter(
                |lamports| ***lamports == 0,
                |_| {
                    msg!("[Error] Account {:?} must NOT be initialized", acc.key);
                    ProgramError::AccountAlreadyInitialized
                }
            )
            .and(Ok(acc))
    }

    fn ensure_initialized<'a, 'b>(
        acc: &'a AccountInfo<'b>,
    ) -> Result<&'a AccountInfo<'b>, ProgramError> {
        acc.try_borrow_lamports()
            .filter(
                |lamports| ***lamports > 0,
                |_| {
                    msg!("[Error] Account {:?} must be initialized", acc.key);
                    ProgramError::UninitializedAccount
                }
            )
            .and(Ok(acc))
    }

    fn validate_pda<'a, 'b>(
        acc: &'a AccountInfo<'b>,
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Result<(&'a AccountInfo<'b>, u8), ProgramError> {
        let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
        if acc.key != &pda {
            msg!("[Error] Account {:?} is not a PDA derived from program ID {:?} and seeds {:?}", acc.key, program_id, seeds);
            Err(ProgramError::InvalidSeeds)
        } else {
            Ok((acc, bump))
        }
    }
}

trait Filter<T, E> {
    fn filter<F, G>(self, pred: F, err_get: G) -> Self
    where
        F: FnOnce(&T) -> bool,
        G: FnOnce(&T) -> E;
}

impl<T, E> Filter<T, E> for Result<T, E> {
    fn filter<F, G>(self, pred: F, err_get: G) -> Self
    where
        F: FnOnce(&T) -> bool,
        G: FnOnce(&T) -> E,
    {
        self.and_then(|t| if pred(&t) { Ok(t) } else { Err(err_get(&t)) })
    }
}
