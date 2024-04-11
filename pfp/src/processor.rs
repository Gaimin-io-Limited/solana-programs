use mpl_token_metadata::{
    accounts::{Metadata, TokenRecord},
    types::{TokenStandard, TokenState},
};

use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program::invoke_signed, program_error::ProgramError, program_pack::Pack, pubkey,
    pubkey::Pubkey, sysvar::Sysvar,
};

use spl_token::state::{Account, Mint};

use crate::{
    error::GaiminError,
    instruction::{accounts::*, ClaimArgs, ConfigArgs, CreateClaimArgs, GaiminInstruction},
    state::{ClaimRecord, Config, NftRecord},
    utils::*,
};

pub const CONFIG_PDA_SEED: &[u8] = b"config";
pub const NFT_PDA_SEED: &[u8] = b"nft";
pub const CLAIM_PDA_SEED: &[u8] = b"claim";
pub const MPL_TOKEN_METADATA_PROGRAM_ID: Pubkey =
    pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub struct Processor;

impl Processor {
    pub fn process<'a>(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'a>],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = GaiminInstruction::unpack(instruction_data)?;

        msg!("Processing instruction: {:?}", instruction);
        match instruction {
            GaiminInstruction::Config(data) => Self::process_config(
                program_id,
                ConfigAccounts::context(accounts)?.accounts,
                data,
            ),
            GaiminInstruction::Delete => {
                Self::process_delete(program_id, DeleteAccounts::context(accounts)?.accounts)
            }
            GaiminInstruction::Nft => {
                Self::process_nft(program_id, NftAccounts::context(accounts)?.accounts)
            }
            GaiminInstruction::CreateClaim(data) => Self::process_create_claim(
                program_id,
                CreateClaimAccounts::context(accounts)?.accounts,
                data,
            ),
            GaiminInstruction::Claim(data) => {
                Self::process_claim(program_id, ClaimAccounts::context(accounts)?.accounts, data)
            }
        }
    }

    fn process_config(
        program_id: &Pubkey,
        accounts: ConfigAccounts,
        data: ConfigArgs,
    ) -> ProgramResult {
        // Authority validation
        assert_signer(accounts.authority)?;

        // Config validation
        let bump = assert_derived_from(accounts.config, program_id, &[CONFIG_PDA_SEED])?;
        assert_uninitialized(accounts.config)?;

        let accumulation_duration = data.total_accumulation_period / data.accumulated_reward;

        if data.accumulated_reward < 0
            || data.initial_reward < 0
            || accumulation_duration <= 0
            || data.generation_duration < 0
        {
            msg!("[Error] Config data is invalid");
            return Err(GaiminError::InvalidConfig.into());
        }

        data.initial_reward
            .checked_add(data.accumulated_reward)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Config creation
        invoke_signed(
            &create_account_ix::<Config>(accounts.config.key, accounts.authority.key, program_id),
            &[accounts.authority.clone(), accounts.config.clone()],
            &[&[CONFIG_PDA_SEED, &[bump]]],
        )?;

        Config::pack(
            Config {
                authority: *accounts.authority.key,
                creator: *accounts.creator.key,
                claimable_from: data.claimable_from,
                accumulated_reward: data.accumulated_reward,
                initial_reward: data.initial_reward,
                accumulation_duration,
                generation_duration: data.generation_duration,
            },
            &mut accounts.config.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn process_delete(program_id: &Pubkey, accounts: DeleteAccounts) -> ProgramResult {
        // Config validation
        assert_derived_from(accounts.config, program_id, &[CONFIG_PDA_SEED])?;
        assert_initialized(accounts.config)?;

        // Authority validation
        assert_signer(accounts.authority)?;
        let config = Config::unpack_unchecked(&accounts.config.try_borrow_data()?)?;
        if config.authority != *accounts.authority.key {
            return Err(GaiminError::PermissionDenied.into());
        }

        delete_account(accounts.target, accounts.receiver)
    }

    fn process_nft(program_id: &Pubkey, accounts: NftAccounts) -> ProgramResult {
        // Config validation
        assert_derived_from(accounts.config, program_id, &[CONFIG_PDA_SEED])?;
        assert_initialized(accounts.config)?;

        let config = Config::unpack_unchecked(&accounts.config.try_borrow_data()?)?;

        // Authority validation
        assert_signer(accounts.payer)?;

        // NFT Record validation
        let bump = assert_derived_from(
            accounts.nft_record,
            program_id,
            &[NFT_PDA_SEED, &accounts.mint.key.to_bytes()],
        )?;
        if is_initialized(accounts.nft_record)? {
            return Ok(());
        }

        // Mint/Edition validation
        if *accounts.mint.owner != spl_token::id() {
            msg!("[Error] Mint address is not owned by the Token Program");
            return Err(GaiminError::InvalidNft.into());
        }

        assert_derived_from(
            accounts.edition,
            &MPL_TOKEN_METADATA_PROGRAM_ID,
            &[
                b"metadata",
                &MPL_TOKEN_METADATA_PROGRAM_ID.to_bytes(),
                &accounts.mint.key.to_bytes(),
                b"edition",
            ],
        )?;
        assert_initialized(accounts.edition)?;

        let mint = Mint::unpack_unchecked(&accounts.mint.try_borrow_data()?)?;
        if !mint.mint_authority.contains(accounts.edition.key) {
            msg!("[Error] Unexpected mint authority");
            return Err(GaiminError::InvalidNft.into());
        }

        // Metadata validation
        assert_derived_from(
            accounts.metadata,
            &MPL_TOKEN_METADATA_PROGRAM_ID,
            &[
                b"metadata",
                &MPL_TOKEN_METADATA_PROGRAM_ID.to_bytes(),
                &accounts.mint.key.to_bytes(),
            ],
        )?;

        let metadata = Metadata::safe_deserialize(&accounts.metadata.try_borrow_data()?)?;
        let token_standard = metadata
            .token_standard
            .ok_or(GaiminError::InvalidTokenStandard)?;
        match token_standard {
            TokenStandard::ProgrammableNonFungible => (),
            TokenStandard::ProgrammableNonFungibleEdition => (),
            _ => return Err(GaiminError::InvalidTokenStandard.into()),
        }

        let valid_creator = metadata.creators.map_or(false, |creators| {
            creators
                .iter()
                .any(|creator| creator.verified && creator.address == config.creator)
        });
        if !valid_creator && &config.authority != accounts.payer.key {
            return Err(GaiminError::InvalidCreator.into());
        }

        // Account creation
        invoke_signed(
            &create_account_ix::<NftRecord>(
                accounts.nft_record.key,
                accounts.payer.key,
                program_id,
            ),
            &[accounts.payer.clone(), accounts.nft_record.clone()],
            &[&[NFT_PDA_SEED, &accounts.mint.key.to_bytes(), &[bump]]],
        )?;

        NftRecord::pack(
            NftRecord {
                claimed_amount: 0,
                total_amount: config.initial_reward + config.accumulated_reward,
                last_claim_at: config.claimable_from,
            },
            &mut accounts.nft_record.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn process_create_claim(
        program_id: &Pubkey,
        accounts: CreateClaimAccounts,
        data: CreateClaimArgs,
    ) -> ProgramResult {
        // User wallet validation
        assert_signer(accounts.wallet)?;

        // Config validation
        assert_derived_from(accounts.config, program_id, &[CONFIG_PDA_SEED])?;
        assert_initialized(accounts.config)?;

        // Claim record validation
        let claim_seeds_with_bump = &[
            CLAIM_PDA_SEED,
            &accounts.wallet.key.to_bytes(),
            &data.seed,
            &[data.bump],
        ];
        assert_derived_from_with_bump(accounts.claim, program_id, claim_seeds_with_bump)?;
        assert_uninitialized(accounts.claim)?;

        // Claim record creation
        invoke_signed(
            &create_account_ix::<ClaimRecord>(accounts.claim.key, accounts.wallet.key, program_id),
            &[accounts.wallet.clone(), accounts.claim.clone()],
            &[claim_seeds_with_bump],
        )?;

        let config = Config::unpack_unchecked(&accounts.config.try_borrow_data()?)?;
        let now = Clock::get()?.unix_timestamp as i32;

        ClaimRecord::pack(
            ClaimRecord {
                generation: now / config.generation_duration,
                amount: 0,
                owner: *accounts.wallet.key,
                bnb_chain_wallet_address: data.bnb_chain_wallet_address,
            },
            &mut accounts.claim.try_borrow_mut_data()?,
        )?;

        Ok(())
    }

    fn process_claim(
        program_id: &Pubkey,
        accounts: ClaimAccounts,
        data: ClaimArgs,
    ) -> ProgramResult {
        // User wallet validation
        assert_signer(accounts.wallet)?;

        // Token account validation
        if *accounts.token.owner != spl_token::id() {
            msg!("[Error] Token account does not belong to the Token Program");
            return Err(GaiminError::InvalidTokenAccount.into());
        }

        let token = Account::unpack_unchecked(&accounts.token.try_borrow_data()?)?;

        assert_derived_from_with_bump(
            accounts.token,
            &SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
            &[
                &accounts.wallet.key.to_bytes(),
                &spl_token::ID.to_bytes(),
                &token.mint.to_bytes(),
                &[data.token_acc_bump],
            ],
        )?;

        if &token.owner != accounts.wallet.key {
            msg!("[Error] Token account does not belong to the user");
            return Err(GaiminError::InvalidTokenAccount.into());
        } else if token.amount == 0 {
            msg!("[Error] Token account does not hold the NFT");
            return Err(GaiminError::ZeroNftBalance.into());
        }

        // Token record validation
        assert_derived_from_with_bump(
            accounts.token_record,
            &MPL_TOKEN_METADATA_PROGRAM_ID,
            &[
                b"metadata",
                &MPL_TOKEN_METADATA_PROGRAM_ID.to_bytes(),
                &token.mint.to_bytes(),
                b"token_record",
                &accounts.token.key.to_bytes(),
                &[data.token_record_bump],
            ],
        )?;

        let token_record =
            TokenRecord::safe_deserialize(&accounts.token_record.try_borrow_data()?)?;
        if token_record.state != TokenState::Locked {
            msg!("[Error] Token account is unlocked");
            return Err(GaiminError::TokenAccountUnlocked.into());
        }

        // Config validation
        assert_derived_from(accounts.config, program_id, &[CONFIG_PDA_SEED])?;
        assert_initialized(accounts.config)?;

        let config = Config::unpack_unchecked(&accounts.config.try_borrow_data()?)?;
        let now = Clock::get()?.unix_timestamp as i32;

        if now < config.claimable_from {
            msg!("[Error] Claiming is not available yet");
            return Err(GaiminError::ClaimingNotAvailable.into());
        }

        // NFT record validation
        assert_derived_from_with_bump(
            accounts.nft_record,
            program_id,
            &[
                NFT_PDA_SEED,
                &token.mint.to_bytes(),
                &[data.nft_record_bump],
            ],
        )?;
        assert_initialized(accounts.nft_record)?;

        let mut nft_record = NftRecord::unpack_unchecked(&accounts.nft_record.try_borrow_data()?)?;
        if nft_record.claimed_amount >= nft_record.total_amount {
            msg!("[Error] No claimable amount left");
            return Err(GaiminError::AmountExhausted.into());
        }

        // Claim record validation
        assert_initialized(accounts.claim)?;

        let mut claim = ClaimRecord::unpack_unchecked(*accounts.claim.try_borrow_data()?)?;

        if &claim.owner != accounts.wallet.key {
            msg!("[Error] Claim record doesn't belong to this wallet");
            return Err(GaiminError::PermissionDenied.into());
        }

        // Reward calculation
        let base_reward = if nft_record.claimed_amount == 0 { config.initial_reward } else { 0 };
        let stake_duration = now - nft_record.last_claim_at;
        let reward = i32::min(
            nft_record.total_amount - nft_record.claimed_amount,
            base_reward + (stake_duration / config.accumulation_duration),
        );

        claim.amount += reward;

        // Claim update
        ClaimRecord::pack(claim, &mut accounts.claim.try_borrow_mut_data()?)?;

        // NFT record update
        nft_record.last_claim_at = now;
        nft_record.claimed_amount += reward;
        NftRecord::pack(nft_record, &mut accounts.nft_record.try_borrow_mut_data()?)?;

        Ok(())
    }
}
