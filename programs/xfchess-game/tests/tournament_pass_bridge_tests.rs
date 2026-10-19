//! The registrar bridge: `issue_pass_eligibility` attests a profile rating to the compiled
//! `xfchess_pass` program by CPI, and the pass then moves only to attested players. Both programs
//! run as SBF binaries (`cargo build-sbf` the game; put the pass binary built with the same ID as
//! `XFCHESS_PASS_PROGRAM_ID` as `xfchess_pass.so` in `SBF_OUT_DIR`).

mod common;

use anchor_lang::{
    solana_program::instruction::AccountMeta,
    AccountDeserialize, InstructionData, Space, ToAccountMetas,
};
use anchor_spl::token_2022::{
    spl_token_2022::{
        extension::{transfer_hook, ExtensionType, StateWithExtensions},
        instruction as token_instruction,
        state::{Account as TokenAccountState, Mint as MintState},
    },
    ID as TOKEN_2022,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use solana_system_interface::instruction as system_instruction;
use xfchess_game::{
    constants::{PASS_REGISTRAR_SEED, XFCHESS_PASS_PROGRAM_ID},
    errors::GameErrorCode,
    state::{
        PlayerProfile, Tournament, TournamentPassBinding, TournamentPlayersShard,
        TournamentStatus,
    },
    tournament_ix::lifecycle::initialize_escrow::TournamentEscrow,
};

const ID: u64 = 7;
const BLITZ_BASE_SECONDS: u64 = 300;
const WALLET: u64 = 100_000_000;
const PASS_ISSUE: [u8; 8] = [29, 88, 236, 137, 74, 17, 193, 144];
const PASS_INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const PASS_ERROR_UNAUTHORIZED_REGISTRAR: u32 = 0xD00B;

fn tournament_pda(seed: &[u8]) -> Pubkey {
    Pubkey::find_program_address(&[seed, &ID.to_le_bytes()], &xfchess_game::ID).0
}

fn shard_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"tourney_players", &[0u8], &ID.to_le_bytes()], &xfchess_game::ID).0
}

fn binding_pda() -> Pubkey {
    tournament_pda(b"t_pass")
}

fn registrar_pda() -> Pubkey {
    Pubkey::find_program_address(&[PASS_REGISTRAR_SEED], &xfchess_game::ID).0
}

fn pass_pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &XFCHESS_PASS_PROGRAM_ID).0
}

fn config_pda(mint: &Pubkey) -> Pubkey {
    pass_pda(&[b"pass-config", mint.as_ref()])
}

fn record_pda(mint: &Pubkey, player: &Pubkey) -> Pubkey {
    pass_pda(&[b"eligibility", mint.as_ref(), player.as_ref()])
}

fn extra_list_pda(mint: &Pubkey) -> Pubkey {
    pass_pda(&[b"extra-account-metas", mint.as_ref()])
}

fn code(error: GameErrorCode) -> Option<u32> {
    Some(common::ec(error))
}

fn profile(player: &Pubkey, blitz: u32) -> PlayerProfile {
    PlayerProfile {
        authority: *player,
        elo_blitz: f64::from(blitz) * 100.0,
        ..Default::default()
    }
}

struct Fixture {
    authority: Keypair,
    players: Vec<(Keypair, u32)>,
    tournament: Tournament,
}

impl Fixture {
    fn new(blitz_ratings: &[u32]) -> Self {
        let authority = Keypair::new();
        let mut tournament =
            Tournament::try_deserialize_unchecked(&mut &vec![0u8; 8 + Tournament::INIT_SPACE][..])
                .expect("zeroed tournament");
        tournament.tournament_id = ID;
        tournament.authority = authority.pubkey();
        tournament.host_treasury = Pubkey::new_unique();
        tournament.bump =
            Pubkey::find_program_address(&[b"tournament", &ID.to_le_bytes()], &xfchess_game::ID).1;
        tournament.status = TournamentStatus::Registration;
        tournament.max_players = 8;
        tournament.elo_min = 1_400;
        tournament.elo_max = 1_600;
        tournament.base_time_seconds = BLITZ_BASE_SECONDS;
        Self {
            authority,
            players: blitz_ratings.iter().map(|r| (Keypair::new(), *r)).collect(),
            tournament,
        }
    }

    async fn start(&self) -> ProgramTestContext {
        if std::env::var_os("SBF_OUT_DIR").is_none() {
            std::env::set_var(
                "SBF_OUT_DIR",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy"),
            );
        }
        let mut pt = ProgramTest::new("xfchess_game", xfchess_game::ID, None);
        pt.add_program("xfchess_pass", XFCHESS_PASS_PROGRAM_ID, None);
        pt.add_account(
            tournament_pda(b"tournament"),
            common::program_account(&self.tournament, 8 + Tournament::INIT_SPACE),
        );
        let mut escrow = common::program_account(&TournamentEscrow {}, 8);
        escrow.lamports = Rent::default().minimum_balance(8);
        pt.add_account(tournament_pda(b"t_escrow"), escrow);
        pt.add_account(
            shard_pda(),
            common::program_account(
                &TournamentPlayersShard {
                    tournament_id: ID,
                    shard_id: 0,
                    players: vec![],
                    player_elos: vec![],
                    swiss_standings: vec![],
                },
                8 + TournamentPlayersShard::space_for(),
            ),
        );
        pt.add_account(self.authority.pubkey(), common::system_account(WALLET));
        for (player, blitz) in &self.players {
            pt.add_account(player.pubkey(), common::system_account(WALLET));
            pt.add_account(
                common::profile_pda(&player.pubkey()).0,
                common::program_account(
                    &profile(&player.pubkey(), *blitz),
                    8 + PlayerProfile::INIT_SPACE,
                ),
            );
        }
        pt.start_with_context().await
    }

    fn bind_ix(&self, mint: &Pubkey) -> Instruction {
        Instruction {
            program_id: xfchess_game::ID,
            accounts: xfchess_game::__client_accounts_bind_tournament_pass::BindTournamentPass {
                tournament: tournament_pda(b"tournament"),
                pass_binding: binding_pda(),
                pass_mint: *mint,
                authority: self.authority.pubkey(),
                system_program: solana_system_interface::program::ID,
            }
            .to_account_metas(None),
            data: xfchess_game::instruction::BindTournamentPass { tournament_id: ID }.data(),
        }
    }
}

fn bridge_ix(mint: &Pubkey, player: &Pubkey, payer: &Pubkey) -> Instruction {
    Instruction {
        program_id: xfchess_game::ID,
        accounts: xfchess_game::__client_accounts_issue_pass_eligibility::IssuePassEligibility {
            tournament: tournament_pda(b"tournament"),
            pass_binding: binding_pda(),
            player: *player,
            player_profile: common::profile_pda(player).0,
            pass_mint: *mint,
            pass_config: config_pda(mint),
            eligibility_record: record_pda(mint, player),
            registrar: registrar_pda(),
            payer: *payer,
            pass_program: XFCHESS_PASS_PROGRAM_ID,
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: xfchess_game::instruction::IssuePassEligibility { tournament_id: ID }.data(),
    }
}

fn pass_initialize_ix(mint: &Pubkey, hook_authority: &Pubkey, payer: &Pubkey) -> Instruction {
    let mut data = PASS_INITIALIZE.to_vec();
    data.extend_from_slice(&ID.to_le_bytes());
    data.extend_from_slice(registrar_pda().as_ref());
    data.extend_from_slice(&1_400u32.to_le_bytes());
    data.extend_from_slice(&1_600u32.to_le_bytes());
    data.extend_from_slice(&1_000u64.to_le_bytes());
    Instruction {
        program_id: XFCHESS_PASS_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(config_pda(mint), false),
            AccountMeta::new(extra_list_pda(mint), false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*hook_authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data,
    }
}

async fn send_all(ctx: &mut ProgramTestContext, instructions: &[Instruction], extra: &[&Keypair]) {
    let mut signers: Vec<&Keypair> = vec![&ctx.payer];
    signers.extend_from_slice(extra);
    let blockhash = ctx.banks_client.get_latest_blockhash().await.expect("blockhash");
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&ctx.payer.pubkey()),
        &signers,
        blockhash,
    );
    ctx.banks_client
        .process_transaction(transaction)
        .await
        .expect("setup transaction");
}

async fn create_hooked_mint(ctx: &mut ProgramTestContext) -> Pubkey {
    let mint = Keypair::new();
    let payer = ctx.payer.pubkey();
    let len = ExtensionType::try_calculate_account_len::<MintState>(&[ExtensionType::TransferHook])
        .expect("length");
    let rent = ctx.banks_client.get_rent().await.expect("rent").minimum_balance(len);
    send_all(
        ctx,
        &[
            system_instruction::create_account(&payer, &mint.pubkey(), rent, len as u64, &TOKEN_2022),
            transfer_hook::instruction::initialize(
                &TOKEN_2022,
                &mint.pubkey(),
                Some(payer),
                Some(XFCHESS_PASS_PROGRAM_ID),
            )
            .expect("hook extension"),
            token_instruction::initialize_mint2(&TOKEN_2022, &mint.pubkey(), &payer, None, 0)
                .expect("initialize mint"),
        ],
        &[&mint],
    )
    .await;
    mint.pubkey()
}

async fn create_token_account(
    ctx: &mut ProgramTestContext,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let token_account = Keypair::new();
    let payer = ctx.payer.pubkey();
    let len = ExtensionType::try_calculate_account_len::<TokenAccountState>(&[
        ExtensionType::TransferHookAccount,
    ])
    .expect("length");
    let rent = ctx.banks_client.get_rent().await.expect("rent").minimum_balance(len);
    let mut instructions = vec![
        system_instruction::create_account(
            &payer,
            &token_account.pubkey(),
            rent,
            len as u64,
            &TOKEN_2022,
        ),
        token_instruction::initialize_account3(&TOKEN_2022, &token_account.pubkey(), mint, owner)
            .expect("initialize account"),
    ];
    if amount > 0 {
        instructions.push(
            token_instruction::mint_to(&TOKEN_2022, mint, &token_account.pubkey(), &payer, &[], amount)
                .expect("mint to"),
        );
    }
    send_all(ctx, &instructions, &[&token_account]).await;
    token_account.pubkey()
}

async fn data_of(ctx: &mut ProgramTestContext, key: Pubkey) -> Vec<u8> {
    ctx.banks_client
        .get_account(key)
        .await
        .expect("account lookup")
        .expect("account exists")
        .data
}

async fn token_balance(ctx: &mut ProgramTestContext, token_account: Pubkey) -> u64 {
    let data = data_of(ctx, token_account).await;
    StateWithExtensions::<TokenAccountState>::unpack(&data)
        .expect("token account")
        .base
        .amount
}

/// (verified_elo, verified_at_slot, is_revoked) of an `EligibilityRecord`, after the 8-byte
/// discriminator, bump, mint and player.
async fn record_fields(ctx: &mut ProgramTestContext, mint: &Pubkey, player: &Pubkey) -> (u32, u64, bool) {
    let data = data_of(ctx, record_pda(mint, player)).await;
    assert_eq!(&data[9..41], mint.as_ref());
    assert_eq!(&data[41..73], player.as_ref());
    let elo = u32::from_le_bytes(data[73..77].try_into().expect("elo"));
    let slot = u64::from_le_bytes(data[77..85].try_into().expect("slot"));
    (elo, slot, data[85] != 0)
}

/// A pass mint owned by the payer, hooked by the pass program, initialized for the tournament
/// with the game program's registrar PDA, and bound to the tournament.
async fn bound_pass(fixture: &Fixture, ctx: &mut ProgramTestContext) -> Pubkey {
    let mint = create_hooked_mint(ctx).await;
    let payer = ctx.payer.pubkey();
    send_all(ctx, &[pass_initialize_ix(&mint, &payer, &payer)], &[]).await;
    common::send(ctx, fixture.bind_ix(&mint), &[&fixture.authority])
        .await
        .expect("bind");
    mint
}

async fn bridge_error(
    ctx: &mut ProgramTestContext,
    instruction: Instruction,
    extra: &[&Keypair],
) -> Option<u32> {
    let error: TransactionError = common::send(ctx, instruction, extra)
        .await
        .expect_err("the bridge must refuse");
    common::custom_code(&error)
}

#[tokio::test]
async fn the_bridge_issues_then_refreshes_eligibility_from_the_profile() {
    let fixture = Fixture::new(&[1_500]);
    let mut ctx = fixture.start().await;
    let mint = bound_pass(&fixture, &mut ctx).await;
    let player = fixture.players[0].0.pubkey();
    let stranger = Keypair::new();
    ctx.set_account(&stranger.pubkey(), &common::system_account(WALLET).into());

    common::send(&mut ctx, bridge_ix(&mint, &player, &stranger.pubkey()), &[&stranger])
        .await
        .expect("anyone may pay to attest a profile rating");
    let (elo, slot, revoked) = record_fields(&mut ctx, &mint, &player).await;
    assert_eq!((elo, revoked), (1_500, false));

    ctx.set_account(
        &common::profile_pda(&player).0,
        &common::program_account(&profile(&player, 1_550), 8 + PlayerProfile::INIT_SPACE).into(),
    );
    ctx.warp_to_slot(slot + 5).expect("warp");
    common::send(&mut ctx, bridge_ix(&mint, &player, &stranger.pubkey()), &[&stranger])
        .await
        .expect("a second call refreshes the record");
    let (elo, refreshed_slot, _) = record_fields(&mut ctx, &mint, &player).await;
    assert_eq!(elo, 1_550);
    assert!(refreshed_slot > slot);
}

#[tokio::test]
async fn the_bridge_refuses_ratings_outside_the_tournament_range() {
    let fixture = Fixture::new(&[1_700]);
    let mut ctx = fixture.start().await;
    let mint = bound_pass(&fixture, &mut ctx).await;
    let player = fixture.players[0].0.pubkey();
    let payer = ctx.payer.insecure_clone();

    assert_eq!(
        bridge_error(&mut ctx, bridge_ix(&mint, &player, &payer.pubkey()), &[]).await,
        code(GameErrorCode::EloOutOfRange)
    );
    assert!(ctx
        .banks_client
        .get_account(record_pda(&mint, &player))
        .await
        .expect("lookup")
        .is_none());
}

#[tokio::test]
async fn the_bridge_refuses_unbound_tournaments_wrong_mints_and_wrong_programs() {
    let fixture = Fixture::new(&[1_500]);
    let mut ctx = fixture.start().await;
    let player = fixture.players[0].0.pubkey();
    let payer = ctx.payer.pubkey();
    let mint = create_hooked_mint(&mut ctx).await;
    send_all(&mut ctx, &[pass_initialize_ix(&mint, &payer, &payer)], &[]).await;

    assert!(
        bridge_error(&mut ctx, bridge_ix(&mint, &player, &payer), &[])
            .await
            .is_some(),
        "no binding"
    );

    common::send(&mut ctx, fixture.bind_ix(&mint), &[&fixture.authority])
        .await
        .expect("bind");
    let other_mint = create_hooked_mint(&mut ctx).await;
    assert_eq!(
        bridge_error(&mut ctx, bridge_ix(&other_mint, &player, &payer), &[]).await,
        code(GameErrorCode::PassMintMismatch)
    );

    let mut wrong_program = bridge_ix(&mint, &player, &payer);
    let slot = wrong_program
        .accounts
        .iter()
        .position(|meta| meta.pubkey == XFCHESS_PASS_PROGRAM_ID)
        .expect("pass program account");
    wrong_program.accounts[slot].pubkey = solana_system_interface::program::ID;
    assert_eq!(
        bridge_error(&mut ctx, wrong_program, &[]).await,
        code(GameErrorCode::InvalidAccountOwner)
    );
}

#[tokio::test]
async fn only_the_game_registrar_can_attest_to_the_pass_program() {
    let fixture = Fixture::new(&[1_500]);
    let mut ctx = fixture.start().await;
    let mint = bound_pass(&fixture, &mut ctx).await;
    let player = fixture.players[0].0.pubkey();
    let payer = ctx.payer.pubkey();

    let mut data = PASS_ISSUE.to_vec();
    data.extend_from_slice(&1_500u32.to_le_bytes());
    let forged = Instruction {
        program_id: XFCHESS_PASS_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(config_pda(&mint), false),
            AccountMeta::new(record_pda(&mint, &player), false),
            AccountMeta::new_readonly(player, false),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
        data,
    };
    assert_eq!(
        bridge_error(&mut ctx, forged, &[]).await,
        Some(PASS_ERROR_UNAUTHORIZED_REGISTRAR)
    );
}

fn transfer_pass_ix(
    mint: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    destination_owner: &Pubkey,
) -> Instruction {
    let mut instruction = token_instruction::transfer_checked(
        &TOKEN_2022,
        source,
        mint,
        destination,
        owner,
        &[],
        1,
        0,
    )
    .expect("transfer");
    instruction.accounts.extend([
        AccountMeta::new_readonly(config_pda(mint), false),
        AccountMeta::new_readonly(record_pda(mint, destination_owner), false),
        AccountMeta::new_readonly(XFCHESS_PASS_PROGRAM_ID, false),
        AccountMeta::new_readonly(extra_list_pda(mint), false),
    ]);
    instruction
}

#[tokio::test]
async fn a_pass_reaches_only_players_the_bridge_attested() {
    let fixture = Fixture::new(&[1_500, 1_500]);
    let mut ctx = fixture.start().await;
    let mint = bound_pass(&fixture, &mut ctx).await;
    let payer = ctx.payer.insecure_clone();
    let attested = fixture.players[0].0.pubkey();
    let unattested = fixture.players[1].0.pubkey();
    let treasury = create_token_account(&mut ctx, &mint, &payer.pubkey(), 2).await;
    let attested_account = create_token_account(&mut ctx, &mint, &attested, 0).await;
    let unattested_account = create_token_account(&mut ctx, &mint, &unattested, 0).await;

    let refused: TransactionError = common::send(
        &mut ctx,
        transfer_pass_ix(&mint, &treasury, &unattested_account, &payer.pubkey(), &unattested),
        &[],
    )
    .await
    .expect_err("an unattested player cannot receive a pass");
    assert!(matches!(refused, TransactionError::InstructionError(..)), "{refused:?}");
    assert_eq!(token_balance(&mut ctx, unattested_account).await, 0);

    common::send(&mut ctx, bridge_ix(&mint, &attested, &payer.pubkey()), &[])
        .await
        .expect("attest");
    common::send(
        &mut ctx,
        transfer_pass_ix(&mint, &treasury, &attested_account, &payer.pubkey(), &attested),
        &[],
    )
    .await
    .expect("an attested player receives the pass");
    assert_eq!(token_balance(&mut ctx, attested_account).await, 1);
}

#[tokio::test]
async fn the_bridge_only_attests_while_the_tournament_is_registering() {
    let mut fixture = Fixture::new(&[1_500]);
    fixture.tournament.status = TournamentStatus::Active;
    let mut ctx = fixture.start().await;
    let player = fixture.players[0].0.pubkey();
    let payer = ctx.payer.pubkey();
    let mint = create_hooked_mint(&mut ctx).await;
    send_all(&mut ctx, &[pass_initialize_ix(&mint, &payer, &payer)], &[]).await;
    ctx.set_account(
        &binding_pda(),
        &common::program_account(
            &TournamentPassBinding {
                tournament_id: ID,
                pass_mint: mint,
                bump: Pubkey::find_program_address(&[b"t_pass", &ID.to_le_bytes()], &xfchess_game::ID).1,
            },
            8 + TournamentPassBinding::INIT_SPACE,
        )
        .into(),
    );
    assert_eq!(
        bridge_error(&mut ctx, bridge_ix(&mint, &player, &payer), &[]).await,
        code(GameErrorCode::TournamentNotInRegistration)
    );
}
