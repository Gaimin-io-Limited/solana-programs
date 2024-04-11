use gaimin_staking::processor::CONFIG_PDA_SEED;
use solana_program_test::*;

use solana_program::pubkey::Pubkey;

pub fn program_test() -> ProgramTest {
    ProgramTest::new("gaimin_staking", gaimin_staking::ID, None)
}

pub fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_PDA_SEED], &gaimin_staking::ID)
}
