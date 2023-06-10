use clap::Parser;
use sha3::Digest;
use tango_dataview::save::Save;

#[derive(clap::Subcommand)]
enum Command {
    Summary,
    Eval,
}

#[derive(clap::Parser)]
struct Args {
    replay_path: std::path::PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let mut f = std::fs::File::open(&args.replay_path)?;
    let replay = tango_pvp::replay::Replay::decode(&mut f)?;

    match args.command {
        Command::Summary => cmd_summary(replay).await,
        Command::Eval => cmd_eval(replay).await,
    }
}

async fn cmd_eval(replay: tango_pvp::replay::Replay) -> Result<(), anyhow::Error> {
    mgba::log::init();

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
    {
        let link_navi_view = match local_save.view_navi().unwrap() {
            tango_dataview::save::NaviView::LinkNavi(view) => view,
            _ => unreachable!(),
        };
        let chips_view = local_save.view_chips().unwrap();
        println!(
            "{};{}",
            link_navi_view.navi(),
            (0..30)
                .map(|i| {
                    let chip = chips_view.chip(0, i).unwrap();
                    format!("{}:{}", chip.id, chip.code)
                })
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    let remote_save =
        tango_dataview::game::exe45::save::Save::from_wram(replay.remote_state.wram())?;
    {
        let link_navi_view = match remote_save.view_navi().unwrap() {
            tango_dataview::save::NaviView::LinkNavi(view) => view,
            _ => unreachable!(),
        };
        let chips_view = remote_save.view_chips().unwrap();
        println!(
            "{};{}",
            link_navi_view.navi(),
            (0..30)
                .map(|i| {
                    let chip = chips_view.chip(0, i).unwrap();
                    format!("{}:{}", chip.id, chip.code)
                })
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    let rom = std::fs::read("exe45.gba")?;

    let patch = std::fs::read(format!(
        "patches/{}/v{}/BR4J_00.bps",
        patch_info.name, patch_info.version
    ))?;
    let patch = bps::Patch::decode(&patch)?;

    let rom = patch.apply(&rom)?;

    let hooks = tango_pvp::hooks::hooks_for_gamedb_entry(&tango_gamedb::BR4J_00).unwrap();
    let (result, state) = tango_pvp::eval::eval(&replay, &rom, hooks).await?;

    println!("{} {}", state.wram()[0x00033018], result.outcome as u8);

    Ok(())
}

async fn cmd_summary(replay: tango_pvp::replay::Replay) -> Result<(), anyhow::Error> {
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

    println!("{} {}", patch_info.name, patch_info.version);

    let mut side_dependent_sha3 = sha3::Sha3_256::new();
    let mut side_independent_sha3 = sha3::Sha3_256::new();

    for ip in replay.input_pairs.iter() {
        side_dependent_sha3.update(
            std::iter::zip(ip.local.packet.iter(), ip.remote.packet.iter())
                .flat_map(|(x, y)| [*x, *y])
                .collect::<Vec<_>>(),
        );
        side_independent_sha3.update(
            std::iter::zip(ip.local.packet.iter(), ip.remote.packet.iter())
                .map(|(x, y)| *x ^ *y)
                .collect::<Vec<_>>(),
        );
    }

    println!(
        "{} {}",
        hex::encode(side_dependent_sha3.finalize()),
        hex::encode(side_independent_sha3.finalize())
    );

    Ok(())
}
