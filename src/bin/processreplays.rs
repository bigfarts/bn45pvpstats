use clap::Parser;
use sha3::Digest;
use tango_dataview::save::Save;

#[derive(serde::Deserialize, Debug)]
struct PatchMetadata {
    pub versions: std::collections::HashMap<String, VersionMetadata>,
}

#[derive(serde::Deserialize, Debug)]
struct VersionMetadata {
    pub netplay_compatibility: String,
}

#[derive(clap::Parser, Clone)]
struct Args {
    #[clap(long, default_value = "pending_replays")]
    pending_replays_dir: std::path::PathBuf,

    #[clap(long, default_value = "hashed_replays")]
    hashed_replays_dir: std::path::PathBuf,

    #[clap(long, default_value = "done_replays")]
    done_replays_dir: std::path::PathBuf,

    #[clap(long, default_value = "rejected_replays")]
    rejected_replays_dir: std::path::PathBuf,

    #[clap(
        long,
        default_value = "postgres://bn45pvpstats@%2Fvar%2Frun%2Fpostgresql/bn45pvpstats"
    )]
    db: String,
}

fn hash_replay(replay: &tango_pvp::replay::Replay) -> Vec<u8> {
    let mut hasher = sha3::Sha3_256::new();
    for ip in replay.input_pairs.iter() {
        hasher.update(
            std::iter::zip(ip.local.packet.iter(), ip.remote.packet.iter())
                .map(|(x, y)| *x ^ *y)
                .collect::<Vec<_>>(),
        );
    }
    hasher.finalize().to_vec()
}

async fn hash_and_move_one(
    args: &Args,
    replay_path: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let (hash, local_player_index) = {
        let mut f = std::fs::File::open(replay_path)?;
        let replay = tango_pvp::replay::Replay::decode(&mut f)?;
        (hex::encode(hash_replay(&replay)), replay.local_player_index)
    };

    let new_replay_path =
        args.hashed_replays_dir
            .join(format!("{}-p{}.tangoreplay", hash, local_player_index + 1));

    log::info!(
        "hash: {} -> {}",
        replay_path.display(),
        new_replay_path.display()
    );
    std::fs::rename(replay_path, new_replay_path)?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum ProcessError {
    #[error("non-retriable: {0}")]
    NonRetriable(anyhow::Error),

    #[error("retriable: {0}")]
    Retriable(anyhow::Error),
}

async fn process_one(
    _args: &Args,
    replay_path: &std::path::Path,
    db_pool: sqlx::Pool<sqlx::postgres::Postgres>,
) -> Result<(), ProcessError> {
    log::info!("processing {}", replay_path.display());

    let replay = tango_pvp::replay::Replay::decode(
        &mut std::fs::File::open(replay_path).map_err(|e| ProcessError::NonRetriable(e.into()))?,
    )
    .map_err(|e| ProcessError::NonRetriable(e.into()))?;
    let hash = hash_replay(&replay);

    let ts = sqlx::types::time::OffsetDateTime::from(
        std::time::UNIX_EPOCH + std::time::Duration::from_millis(replay.metadata.ts),
    );

    let game_info = replay
        .metadata
        .local_side
        .as_ref()
        .and_then(|side| side.game_info.as_ref())
        .unwrap();

    if game_info.rom_family != "exe45" || game_info.rom_variant != 0 {
        return Err(ProcessError::NonRetriable(
            anyhow::format_err!("bad game: {:?}", game_info).into(),
        ));
    }

    let patch_info = if let Some(patch) = game_info.patch.as_ref() {
        patch
    } else {
        return Err(ProcessError::NonRetriable(
            anyhow::anyhow!("no patch info").into(),
        ));
    };

    let local_save = tango_dataview::game::exe45::save::Save::from_wram(replay.local_state.wram())
        .map_err(|e| ProcessError::NonRetriable(e.into()))?;
    let (local_navi, local_chips, local_regchip) = {
        let link_navi_view = match local_save.view_navi().unwrap() {
            tango_dataview::save::NaviView::LinkNavi(view) => view,
            _ => unreachable!(),
        };
        let chips_view = local_save.view_chips().unwrap();
        (
            link_navi_view.navi(),
            (0..30)
                .map(|i| {
                    let chip = chips_view.chip(0, i).unwrap();
                    (chip.id, chip.code)
                })
                .collect::<Vec<_>>(),
            chips_view.regular_chip_index(0),
        )
    };

    let remote_save =
        tango_dataview::game::exe45::save::Save::from_wram(replay.remote_state.wram())
            .map_err(|e| ProcessError::NonRetriable(e.into()))?;
    let (remote_navi, remote_chips, remote_regchip) = {
        let link_navi_view = match remote_save.view_navi().unwrap() {
            tango_dataview::save::NaviView::LinkNavi(view) => view,
            _ => unreachable!(),
        };
        let chips_view = remote_save.view_chips().unwrap();
        (
            link_navi_view.navi(),
            (0..30)
                .map(|i| {
                    let chip = chips_view.chip(0, i).unwrap();
                    (chip.id, chip.code)
                })
                .collect::<Vec<_>>(),
            chips_view.regular_chip_index(0),
        )
    };

    let rom = std::fs::read("exe45.gba").map_err(|e| ProcessError::Retriable(e.into()))?;

    let patch = std::fs::read(format!(
        "patches/{}/v{}/BR4J_00.bps",
        patch_info.name, patch_info.version
    ))
    .map_err(|e| ProcessError::Retriable(e.into()))?;
    let patch_metadata = toml::from_slice::<PatchMetadata>(
        &std::fs::read(&format!("patches/{}/info.toml", patch_info.name))
            .map_err(|e| ProcessError::Retriable(e.into()))?,
    )
    .map_err(|e| ProcessError::Retriable(e.into()))?;
    let netplay_compatibility = patch_metadata
        .versions
        .get(&patch_info.version)
        .map(|v| v.netplay_compatibility.clone())
        .ok_or_else(|| ProcessError::Retriable(anyhow::anyhow!("invalid version")))?;
    let patch = bps::Patch::decode(&patch).map_err(|e| ProcessError::NonRetriable(e.into()))?;

    let rom = patch
        .apply(&rom)
        .map_err(|e| ProcessError::NonRetriable(e.into()))?;

    let hooks = tango_pvp::hooks::hooks_for_gamedb_entry(&tango_gamedb::BR4J_00).unwrap();
    let (result, state) = tango_pvp::eval::eval(&replay, &rom, hooks)
        .await
        .map_err(|e| ProcessError::NonRetriable(e.into()))?;

    if result.outcome != tango_pvp::stepper::BattleOutcome::Win {
        // Only keep track of wins.
        return Err(ProcessError::NonRetriable(
            anyhow::anyhow!("is loss").into(),
        ));
    }

    log::info!("replay {} accepted", replay_path.display());

    let turns = state.wram()[0x00033018];

    let mut tx = db_pool
        .begin()
        .await
        .map_err(|e| ProcessError::Retriable(e.into()))?;
    sqlx::query!(
        "
        insert into rounds (hash, ts, turns, winner, loser, netplay_compatibility)
        values ($1, $2, $3, $4, $5, $6)
        on conflict (hash) do nothing
        ",
        hash,
        ts,
        turns as i32,
        local_navi as i32,
        remote_navi as i32,
        netplay_compatibility
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ProcessError::Retriable(e.into()))?;
    for (is_winner, chips, regchip) in [
        (true, local_chips, local_regchip),
        (false, remote_chips, remote_regchip),
    ] {
        for (i, (chip_id, chip_code)) in chips.iter().enumerate() {
            sqlx::query!(
                "
                insert into folder_chips (rounds_hash, is_winner, idx, chip_id, chip_code, is_regchip)
                values ($1, $2, $3, $4, $5, $6)
                on conflict (rounds_hash, is_winner, idx) do nothing
                ",
                hash,
                is_winner,
                i as i32,
                *chip_id as i32,
                chip_code.to_string(),
                regchip == Some(i),
            )
            .execute(&mut *tx)
            .await.map_err(|e| ProcessError::Retriable(e.into()))?;
        }
    }
    tx.commit()
        .await
        .map_err(|e| ProcessError::Retriable(e.into()))?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter(Some("scannerserver"), log::LevelFilter::Info)
        .init();
    mgba::log::init();

    let args = Args::parse();

    std::fs::create_dir_all(&args.hashed_replays_dir)?;
    std::fs::create_dir_all(&args.done_replays_dir)?;
    std::fs::create_dir_all(&args.rejected_replays_dir)?;

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&args.db)
        .await?;

    // Hash pending replays.
    for entry in std::fs::read_dir(&args.pending_replays_dir)?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
    {
        let path = entry.path();
        if path.extension() != Some(std::ffi::OsStr::new("tangoreplay")) {
            continue;
        }
        if let Err(err) = hash_and_move_one(&args, &path).await {
            log::error!("hash and move one error for {}: {}", path.display(), err);
        }
    }

    // Process hashed replays.
    let hashed_replays = std::fs::read_dir(&args.hashed_replays_dir)?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| entry.path().extension() == Some(std::ffi::OsStr::new("tangoreplay")))
        .collect::<Vec<_>>();

    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(num_cpus::get()));

    futures_util::future::join_all(hashed_replays.into_iter().map(|entry| {
        let args = args.clone();
        let db_pool = db_pool.clone();
        let sem = sem.clone();
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let replay_path = entry.path();

            if let Err(err) = process_one(&args, &replay_path, db_pool).await {
                log::error!("process one error for {}: {}", replay_path.display(), err);
                if matches!(err, ProcessError::NonRetriable(_)) {
                    std::fs::rename(
                        &replay_path,
                        args.rejected_replays_dir
                            .join(replay_path.file_name().unwrap()),
                    )
                    .unwrap();
                }
            } else {
                log::info!("process one done for {}", replay_path.display());
                std::fs::rename(
                    &replay_path,
                    args.done_replays_dir.join(replay_path.file_name().unwrap()),
                )
                .unwrap();
            }
        })
    }))
    .await;

    Ok(())
}
