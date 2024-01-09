use solana_program::program_error::ProgramError;

#[derive(Copy, Clone)]
pub enum GaiminError {
    InvalidInstruction,
    PermissionDenied,
    ClaimingNotAvailable,
    AmountExhausted,
}

impl From<GaiminError> for ProgramError {
    fn from(e: GaiminError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
