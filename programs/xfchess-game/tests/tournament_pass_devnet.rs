//! Live Devnet run of the tournament pass flow against the deployed `xfchess_game` and
//! `xfchess_pass` programs: create a tournament, bind a hooked pass mint, attest a player through
//! `issue_pass_eligibility`, move the pass, and register by burning it.
//!
//! Needs funded Devnet keypairs, so it is ignored by default:
//! `VPS_KEYPAIR=<vps_authority.json> PAYER_KEYPAIR=<funded throwaway.json> cargo test -p
//! xfchess-game --test tournament_pass_devnet -- --ignored --nocapture`

use std::time::{SystemTime, UNIX_EPOCH};

use anchor_lang::{
    solana_program::instruction::AccountMeta,
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::token_2022::{
    spl_token_2022::{
        extension::{transfer_hook, ExtensionType, StateWithExtensions},
        instruction as token_instruction,
        state::{Account as TokenAccountState, Mint as MintState},
    },
    ID as TOKEN_2022,
};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use solana_system_interface::instruction as system_instruction;
use xfchess_game::{
    constants::{PASS_REGISTRAR_SEED, XFCHESS_PASS_PROGRAM_ID},
    state::{Tournament, TournamentPlayersShard, TournamentType},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DEVNET: &str = "https://api.devnet.solana.com";
const PASS_INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const PASS_ERROR_NOT_ELIGIBLE: u32 = 0xD007;
const ENTRY_FEE: u64 = 20_000_000;
const PRIZE: u64 = 50_000_000;

fn game_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &xfchess_game::ID).0
}

fn pass_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &XFCHESS_PASS_PROGRAM_ID).0
}

fn keypair_from_env(name: &str) -> TestResult<Keypair> {
    let path = std::env::var(name).map_err(|_| format!("set {name} to a keypair file"))?;
    read_keypair_file(&path).map_err(|error| format!("cannot read {name}: {error}").into())
}

fn send(rpc: &RpcClient, label: &str, instructions: &[Instruction], payer: &Keypair, extra: &[&Keypair]) -> TestResult {
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        rpc.get_latest_blockhash()?,
    );
    let signature = rpc
        .send_and_confirm_transaction(&transaction)
        .map_err(|error| format!("{label}: {error}"))?;
    println!("  ok   {label}: {signature}");
    Ok(())
}

/// Simulates a transaction that must be refused and returns its error.
fn refused(rpc: &RpcClient, label: &str, instructions: &[Instruction], payer: &Keypair) -> TestResult<TransactionError> {
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &[payer],
        rpc.get_latest_blockhash()?,
    );
    let error = rpc
        .simulate_transaction(&transaction)?
        .value
        .err
        .ok_or_else(|| format!("{label}: expected a refusal"))?;
    println!("  deny {label}: {error:?}");
    Ok(error.into())
}

fn custom(error: &TransactionError) -> Option<u32> {
    match error {
        TransactionError::InstructionError(_, solana_sdk::instruction::InstructionError::Custom(code)) => Some(*code),
        _ => None,
    }
}

fn account_data(rpc: &RpcClient, key: &Pubkey) -> TestResult<Vec<u8>> {
    Ok(rpc.get_account_with_commitment(key, CommitmentConfig::confirmed())?.value.ok_or("account missing")?.data)
}

fn token_balance(rpc: &RpcClient, token_account: &Pubkey) -> TestResult<u64> {
    let data = account_data(rpc, token_account)?;
    Ok(StateWithExtensions::<TokenAccountState>::unpack(&data)?.base.amount)
}

fn pass_transfer_ix(mint: &Pubkey, source: &Pubkey, destination: &Pubkey, owner: &Pubkey, destination_owner: &Pubkey) -> TestResult<Instruction> {
    let mut instruction =
        token_instruction::transfer_checked(&TOKEN_2022, source, mint, destination, owner, &[], 1, 0)?;
    instruction.accounts.extend([
        AccountMeta::new_readonly(pass_pda(&[b"pass-config", mint.as_ref()]), false),
        AccountMeta::new_readonly(pass_pda(&[b"eligibility", mint.as_ref(), destination_owner.as_ref()]), false),
        AccountMeta::new_readonly(XFCHESS_PASS_PROGRAM_ID, false),
        AccountMeta::new_readonly(pass_pda(&[b"extra-account-metas", mint.as_ref()]), false),
    ]);
    Ok(instruction)
}

fn token_account_ixs(rpc: &RpcClient, payer: &Pubkey, account: &Keypair, mint: &Pubkey, owner: &Pubkey) -> TestResult<Vec<Instruction>> {
    let len = ExtensionType::try_calculate_account_len::<TokenAccountState>(&[ExtensionType::TransferHookAccount])?;
    Ok(vec![
        system_instruction::create_account(payer, &account.pubkey(), rpc.get_minimum_balance_for_rent_exemption(len)?, len as u64, &TOKEN_2022),
        token_instruction::initialize_account3(&TOKEN_2022, &account.pubkey(), mint, owner)?,
    ])
}

#[test]
#[ignore = "spends Devnet SOL; needs VPS_KEYPAIR and PAYER_KEYPAIR"]
fn the_deployed_programs_attest_move_and_burn_a_pass() -> TestResult {
    let rpc = RpcClient::new_with_commitment(DEVNET.to_string(), CommitmentConfig::confirmed());
    let vps = keypair_from_env("VPS_KEYPAIR")?;
    let payer = keypair_from_env("PAYER_KEYPAIR")?;
    let player = Keypair::new();
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let registrar = game_pda(&[PASS_REGISTRAR_SEED]);
    let tournament = game_pda(&[b"tournament", &id.to_le_bytes()]);
    let escrow = game_pda(&[b"t_escrow", &id.to_le_bytes()]);
    let shard = game_pda(&[b"tourney_players", &[0u8], &id.to_le_bytes()]);
    let binding = game_pda(&[b"t_pass", &id.to_le_bytes()]);
    let profile = game_pda(&[b"profile", player.pubkey().as_ref()]);
    println!("tournament {id}, game {}, pass program {XFCHESS_PASS_PROGRAM_ID}", xfchess_game::ID);
    for program in [xfchess_game::ID, XFCHESS_PASS_PROGRAM_ID] {
        assert!(rpc.get_account(&program)?.executable, "{program} is not deployed on Devnet");
    }

    // Tournament: blitz, 2 players, rating range 1000..=1400 (a new profile counts as 1200).
    let init = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_initialize_tournament::InitializeTournament {
            tournament,
            usdc_prize_escrow_authority: game_pda(&[b"t_usdc_prize", &id.to_le_bytes()]),
            usdc_prize_escrow: None,
            usdc_mint: None,
            authority: vps.pubkey(),
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::InitializeTournament {
            tournament_id: id,
            name: "Pass bridge Devnet".to_string(),
            entry_fee: ENTRY_FEE,
            max_players: 2,
            tournament_type: TournamentType::SingleElimination,
            elo_min: 1_000,
            elo_max: 1_400,
            min_players: 2,
            prize_shares: [7000, 3000, 0, 0, 0, 0, 0, 0, 0, 0],
            platform_fee: 0,
            winner_takes_all: false,
            host_treasury: vps.pubkey(),
            usdc_mint: None,
            base_time_seconds: 300,
            increment_seconds: 0,
        }
        .data(),
    };
    let escrow_ix = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_initialize_tournament_escrow::InitializeTournamentEscrow {
            tournament,
            escrow_pda: escrow,
            authority: vps.pubkey(),
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::InitializeTournamentEscrow { tournament_id: id }.data(),
    };
    let shards_ix = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_initialize_shards_small::InitializeShardsSmall {
            tournament,
            tournament_players_shard_0: shard,
            authority: vps.pubkey(),
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::InitializeShardsSmall { tournament_id: id }.data(),
    };
    send(&rpc, "initialize tournament, escrow and shards", &[init, escrow_ix, shards_ix], &vps, &[])?;
    let fund_prize = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_fund_sol_prize::FundSolPrize {
            tournament,
            escrow_pda: escrow,
            operator: vps.pubkey(),
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::FundSolPrize { tournament_id: id, amount: PRIZE }.data(),
    };
    send(&rpc, "fund the guaranteed prize", &[fund_prize], &vps, &[])?;

    // Player wallet and profile.
    send(&rpc, "fund the player", &[system_instruction::transfer(&payer.pubkey(), &player.pubkey(), 100_000_000)], &payer, &[])?;
    let username = format!("pass{}", id % 100_000_000);
    let init_profile = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_init_profile::InitProfile {
            player_profile: profile,
            username_record: game_pda(&[b"username", username.as_bytes()]),
            player: player.pubkey(),
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::InitProfile {
            username,
            country: "GB".to_string(),
            date_of_birth: 631_152_000,
        }
        .data(),
    };
    send(&rpc, "create the player profile", &[init_profile], &player, &[])?;

    // Pass mint hooked by the deployed pass program; this program's registrar PDA attests.
    let mint = Keypair::new();
    let len = ExtensionType::try_calculate_account_len::<MintState>(&[ExtensionType::TransferHook])?;
    send(
        &rpc,
        "create the hooked pass mint",
        &[
            system_instruction::create_account(&payer.pubkey(), &mint.pubkey(), rpc.get_minimum_balance_for_rent_exemption(len)?, len as u64, &TOKEN_2022),
            transfer_hook::instruction::initialize(&TOKEN_2022, &mint.pubkey(), Some(payer.pubkey()), Some(XFCHESS_PASS_PROGRAM_ID))?,
            token_instruction::initialize_mint2(&TOKEN_2022, &mint.pubkey(), &payer.pubkey(), None, 0)?,
        ],
        &payer,
        &[&mint],
    )?;
    let mint = mint.pubkey();
    let mut data = PASS_INITIALIZE.to_vec();
    data.extend_from_slice(&id.to_le_bytes());
    data.extend_from_slice(registrar.as_ref());
    data.extend_from_slice(&1_000u32.to_le_bytes());
    data.extend_from_slice(&1_400u32.to_le_bytes());
    data.extend_from_slice(&1_000_000u64.to_le_bytes());
    let pass_initialize = Instruction {
        program_id: XFCHESS_PASS_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(pass_pda(&[b"pass-config", mint.as_ref()]), false),
            AccountMeta::new(pass_pda(&[b"extra-account-metas", mint.as_ref()]), false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data,
    };
    send(&rpc, "initialize the pass for the tournament", &[pass_initialize], &payer, &[])?;
    let bind = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_bind_tournament_pass::BindTournamentPass {
            tournament,
            pass_binding: binding,
            pass_mint: mint,
            authority: vps.pubkey(),
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::BindTournamentPass { tournament_id: id }.data(),
    };
    send(&rpc, "bind the pass to the tournament", &[bind], &vps, &[])?;

    // Pass token accounts: the organizer holds the supply, the player starts empty.
    let (treasury, player_pass) = (Keypair::new(), Keypair::new());
    let mut ixs = token_account_ixs(&rpc, &payer.pubkey(), &treasury, &mint, &payer.pubkey())?;
    ixs.push(token_instruction::mint_to(&TOKEN_2022, &mint, &treasury.pubkey(), &payer.pubkey(), &[], 1)?);
    ixs.extend(token_account_ixs(&rpc, &payer.pubkey(), &player_pass, &mint, &player.pubkey())?);
    send(&rpc, "create the pass accounts and mint one pass", &ixs, &payer, &[&treasury, &player_pass])?;

    // Before attestation the hook refuses the transfer.
    let transfer = pass_transfer_ix(&mint, &treasury.pubkey(), &player_pass.pubkey(), &payer.pubkey(), &player.pubkey())?;
    let error = refused(&rpc, "pass transfer before attestation", &[transfer.clone()], &payer)?;
    assert_eq!(custom(&error), Some(PASS_ERROR_NOT_ELIGIBLE));

    // The bridge attests the profile rating; anyone may pay for it.
    let bridge = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_issue_pass_eligibility::IssuePassEligibility {
            tournament,
            pass_binding: binding,
            player: player.pubkey(),
            player_profile: profile,
            pass_mint: mint,
            pass_config: pass_pda(&[b"pass-config", mint.as_ref()]),
            eligibility_record: pass_pda(&[b"eligibility", mint.as_ref(), player.pubkey().as_ref()]),
            registrar,
            payer: payer.pubkey(),
            pass_program: XFCHESS_PASS_PROGRAM_ID,
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::IssuePassEligibility { tournament_id: id }.data(),
    };
    send(&rpc, "issue_pass_eligibility through the game program", &[bridge], &payer, &[])?;
    let record = account_data(&rpc, &pass_pda(&[b"eligibility", mint.as_ref(), player.pubkey().as_ref()]))?;
    assert_eq!(&record[9..41], mint.as_ref());
    assert_eq!(&record[41..73], player.pubkey().as_ref());
    assert_eq!(u32::from_le_bytes(record[73..77].try_into()?), 1_200, "the profile's rating");
    assert_eq!(record[85], 0, "not revoked");

    send(&rpc, "transfer the pass to the attested player", &[transfer], &payer, &[])?;
    assert_eq!(token_balance(&rpc, &player_pass.pubkey())?, 1);

    // Registration burns the pass.
    let register = Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_register_player::RegisterPlayer {
            tournament,
            player_profile: profile,
            player: player.pubkey(),
            escrow_pda: escrow,
            tournament_players_shard_0: shard,
            tournament_players_shard_1: None,
            tournament_players_shard_2: None,
            tournament_players_shard_3: None,
            host_treasury: vps.pubkey(),
            system_program: solana_system_interface::program::ID,
            pass_binding: binding,
            pass_mint: Some(mint),
            player_pass_account: Some(player_pass.pubkey()),
            token_program_2022: Some(TOKEN_2022),
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::RegisterPlayer { tournament_id: id }.data(),
    };
    send(&rpc, "register_player burning the pass", &[register], &player, &[])?;
    assert_eq!(token_balance(&rpc, &player_pass.pubkey())?, 0, "the pass is burned");
    let supply = StateWithExtensions::<MintState>::unpack(&account_data(&rpc, &mint)?)?.base.supply;
    assert_eq!(supply, 0, "the only pass was burned");
    let shard_state = TournamentPlayersShard::try_deserialize(&mut account_data(&rpc, &shard)?.as_slice())?;
    assert_eq!(shard_state.players, vec![player.pubkey()]);
    assert_eq!(shard_state.player_elos, vec![1_200]);
    let tournament_state = Tournament::try_deserialize(&mut account_data(&rpc, &tournament)?.as_slice())?;
    assert_eq!(tournament_state.num_registered_players, 1);
    println!("\nmint {mint}\nplayer {}\ntournament {id}", player.pubkey());
    Ok(())
}
