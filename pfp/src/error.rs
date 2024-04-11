use solana_program::program_error::ProgramError;

/// Custom errors of the GMRX Claim Program
#[derive(Copy, Clone, Debug)]
pub enum GaiminError {
    /// Error code: `0x0`
    ///
    /// The instruction code (the first byte of the instruction data buffer) is invalid
    InvalidInstruction,

    /// Error code: `0x1`
    ///
    /// Invalid config parameters:
    /// - Total reward is zero or negative
    /// - Initial reward fraction is not between 0 and 1
    /// - Reward period is negative
    InvalidConfig,

    /// Error code: `0x2`
    ///
    /// The config authority signature is not included in the system instruction that requires it
    PermissionDenied,

    /// Error code: `0x3`
    ///
    /// NFT mint account is not owned by the Token Program or it doesn't have a valid edition
    /// account as its mint authority
    InvalidNft,

    /// Error code: `0x4`
    ///
    /// NFT is not a programmable NFT
    InvalidTokenStandard,

    /// Error code: `0x5`
    ///
    /// Either the token account is not owned by the Token Program, or it doesn't belong to the
    /// user's wallet account, or the mint address of the token account doesn't match the mint
    /// address of the NFT
    InvalidTokenAccount,

    /// Error code: `0x6`
    ///
    /// The user doesn't own the NFT
    ZeroNftBalance,

    /// Error code: `0x7`
    ///
    /// The token account is unlocked during a claim instruction
    TokenAccountUnlocked,

    /// Error code: `0x8`
    ///
    /// Attempted to parse a byte array as a string, but it didn't contain valid UTF-8
    InvalidString,

    /// Error code: `0x9`
    ///
    /// Attempted to claim a reward before the starting date
    ClaimingNotAvailable,

    /// Error code `0xA`
    ///
    /// Attempted to claim a reward after the full amount has already been claimed
    AmountExhausted,

    /// Error code `0xB`
    ///
    /// Attempted to create an NFT record that wasn't created by the account specified in config
    InvalidCreator,
}

impl From<GaiminError> for ProgramError {
    fn from(e: GaiminError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
