use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, instruction::Instruction, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent,
};

use crate::error::GaiminError;

pub fn assert_signer(acc: &AccountInfo) -> ProgramResult {
    if !acc.is_signer {
        msg!("[Error] Expected a signature from {}", acc.key);
        Err(ProgramError::MissingRequiredSignature)
    } else {
        Ok(())
    }
}

pub fn assert_derived_from(
    acc: &AccountInfo,
    program_id: &Pubkey,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (key, bump) = Pubkey::find_program_address(seeds, program_id);
    if &key != acc.key {
        msg!(
            "[Error] Expected a PDA derived from PID {} and seeds: {:?}",
            program_id,
            seeds
        );
        Err(ProgramError::InvalidSeeds)
    } else {
        Ok(bump)
    }
}

pub fn assert_derived_from_with_bump(
    acc: &AccountInfo,
    program_id: &Pubkey,
    seeds_with_bump: &[&[u8]],
) -> ProgramResult {
    Pubkey::create_program_address(seeds_with_bump, program_id)
        .ok()
        .filter(|key| key == acc.key)
        .map(|_| ())
        .ok_or_else(|| {
            msg!(
                "[Error] Expected a PDA derived from PID {} and seeds with bump: {:?}",
                program_id,
                seeds_with_bump
            );
            ProgramError::InvalidSeeds
        })
}

pub fn assert_initialized(acc: &AccountInfo) -> ProgramResult {
    if is_initialized(acc)? {
        Ok(())
    } else {
        Err(ProgramError::UninitializedAccount)
    }
}

pub fn assert_uninitialized(acc: &AccountInfo) -> ProgramResult {
    if is_initialized(acc)? {
        Err(ProgramError::AccountAlreadyInitialized)
    } else {
        Ok(())
    }
}

pub fn assert_ix_data_length(data: &[u8], len: usize) -> ProgramResult {
    if data.len() != len {
        msg!(
            "[Error] Expected {} bytes of instruction data, received {} instead",
            len,
            data.len()
        );
        Err(ProgramError::InvalidInstructionData)
    } else {
        Ok(())
    }
}

pub fn is_initialized(acc: &AccountInfo) -> Result<bool, ProgramError> {
    acc.try_borrow_lamports().map(|lamports| **lamports != 0)
}

pub fn create_account_ix<T: Pack>(acc: &Pubkey, payer: &Pubkey, owner: &Pubkey) -> Instruction {
    solana_program::system_instruction::create_account(
        &payer,
        &acc,
        Rent::default().minimum_balance(T::LEN),
        T::LEN as u64,
        owner,
    )
}

pub fn delete_account(acc: &AccountInfo, dest: &AccountInfo) -> ProgramResult {
    **dest.lamports.borrow_mut() = dest
        .lamports()
        .checked_add(acc.lamports())
        .ok_or(ProgramError::ArithmeticOverflow)?;
    **acc.lamports.borrow_mut() = 0;

    acc.data.borrow_mut().fill(0);

    Ok(())
}

pub fn parse_string(bytes: &[u8]) -> Result<String, ProgramError> {
    String::from_utf8(Vec::from(bytes)).map_err(|_| GaiminError::InvalidString.into())
}
