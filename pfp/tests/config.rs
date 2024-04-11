// #![cfg(feature = "test-bpf")]

mod utils;

use gaimin_staking::instruction::{ConfigArgs, GaiminInstruction};
use solana_program::{instruction::{Instruction, AccountMeta}, system_program};
use solana_program_test::tokio;
use solana_sdk::{transaction::Transaction, signer::Signer};
use utils::*;

#[tokio::test]
async fn config() {
    let mut context = program_test().start_with_context().await;

    let instruction = GaiminInstruction::Config(ConfigArgs {
        claimable_from: 0,
        total_reward: 40000.0,
        initial_reward_frac: 0.2,
        reward_period_sec: 9000,
    });

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: gaimin_staking::ID,
            accounts: vec![
                AccountMeta::new_readonly(context.payer.pubkey(), true),
                AccountMeta::new(config_pda().0, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction.pack(),
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(transaction).await.unwrap();
}
