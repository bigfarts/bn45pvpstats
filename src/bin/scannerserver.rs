use clap::Parser;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use sha3::Digest;
use tango_dataview::save::Save;

#[derive(clap::Parser)]
struct Args {
    #[clap(long, default_value = "pending_replays")]
    pending_replays_dir: std::path::PathBuf,

    #[clap(long, default_value = "hashed_replays")]
    hashed_replays_dir: std::path::PathBuf,

    #[clap(long, default_value = "done_replays")]
    done_replays_dir: std::path::PathBuf,
}

async fn hash_and_move_one(
    args: &Args,
    replay_path: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let hash = {
        let mut f = std::fs::File::open(replay_path)?;
        let replay = tango_pvp::replay::Replay::decode(&mut f)?;

        let mut side_dependent_sha3 = sha3::Sha3_256::new();

        for ip in replay.input_pairs.iter() {
            side_dependent_sha3.update(
                std::iter::zip(ip.local.packet.iter(), ip.remote.packet.iter())
                    .flat_map(|(x, y)| [*x, *y])
                    .collect::<Vec<_>>(),
            );
        }

        hex::encode(side_dependent_sha3.finalize())
    };

    std::fs::rename(
        replay_path,
        args.hashed_replays_dir
            .join(format!("{}.tangoreplay", hash)),
    )?;

    Ok(())
}

async fn run_once(args: &Args) -> Result<(), anyhow::Error> {
    // Hash pending replays.
    for entry in std::fs::read_dir(&args.pending_replays_dir)?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
    {
        let path = entry.path();
        if path.extension() != Some(std::ffi::OsStr::new("tangoreplay")) {
            continue;
        }
        if let Err(err) = hash_and_move_one(args, &path).await {
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

    hashed_replays.into_par_iter().for_each(|entry| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        if let Err(err) = runtime.block_on(process_one(args, &entry.path())) {
            log::error!("process one error for {}: {}", entry.path().display(), err);
        }
    });

    Ok(())
}

async fn process_one(args: &Args, replay_path: &std::path::Path) -> Result<(), anyhow::Error> {
    let replay = tango_pvp::replay::Replay::decode(&mut std::fs::File::open(replay_path)?)?;

    let game_info = replay
        .metadata
        .local_side
        .as_ref()
        .and_then(|side| side.game_info.as_ref())
        .unwrap();

    if game_info.rom_family != "exe45" || game_info.rom_variant != 0 {
        return Err(anyhow::format_err!("bad game: {:?}", game_info));
    }

    let patch_info = if let Some(patch) = game_info.patch.as_ref() {
        patch
    } else {
        return Err(anyhow::anyhow!("no patch info"));
    };

    let local_save = tango_dataview::game::exe45::save::Save::from_wram(replay.local_state.wram())?;
    let (local_navi, local_chips) = {
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
        )
    };

    let remote_save =
        tango_dataview::game::exe45::save::Save::from_wram(replay.remote_state.wram())?;
    let (remote_navi, remote_chips) = {
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
        )
    };

    let rom = std::fs::read("exe45.gba")?;

    let patch = std::fs::read(format!(
        "patches/{}/v{}/BR4J_00.bps",
        patch_info.name, patch_info.version
    ))?;
    let patch = bps::Patch::decode(&patch)?;

    let rom = patch.apply(&rom)?;

    let hooks = tango_pvp::hooks::hooks_for_gamedb_entry(&tango_gamedb::BR4J_00).unwrap();
    let (result, state) = tango_pvp::eval::eval(&replay, &rom, hooks).await?;

    if result.outcome != tango_pvp::stepper::BattleOutcome::Win {
        // Only keep track of wins.
        return Ok(());
    }

    let turns = state.wram()[0x00033018];

    // TODO: Do stuff.

    std::fs::rename(
        replay_path,
        args.done_replays_dir.join(replay_path.file_name().unwrap()),
    )?;

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

    loop {
        if let Err(err) = run_once(&args).await {
            log::error!("run error: {}", err);
        }

        const SLEEP_DURATION: std::time::Duration = std::time::Duration::from_secs(5);
        log::info!("sleeping for {:?}", SLEEP_DURATION);
        tokio::time::sleep(SLEEP_DURATION).await;
    }
}
