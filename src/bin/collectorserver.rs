use clap::Parser;
use futures::stream::TryStreamExt;
use prost::Message;
use routerify::ext::RequestExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(clap::Parser)]
struct Args {
    #[clap(long, default_value = "[::]:1982")]
    listen_addr: String,

    #[clap(long, default_value = "pending_replays")]
    pending_replays_dir: std::path::PathBuf,
}

const MAX_METADATA_LEN: usize = 10 * 1024; // do not allow metadata to exceed 10KiB

struct State {
    pending_replays_dir: std::path::PathBuf,
}

#[derive(PartialEq)]
struct PatchInfo {
    name: String,
    version: String,
}

async fn handle_submit_request(
    request: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, anyhow::Error> {
    let pending_replays_dir = {
        let state = request.data::<State>().unwrap();
        state.pending_replays_dir.clone()
    };

    let mut reader = tokio_util::compat::FuturesAsyncReadCompatExt::compat(
        TryStreamExt::map_err(request.into_body(), |e| {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })
        .into_async_read(),
    );

    let mut header = [0u8; 4];
    reader.read_exact(&mut header).await?;

    // Read the header.
    if &header != tango_pvp::replay::HEADER {
        return Ok(hyper::Response::builder()
            .status(hyper::http::StatusCode::BAD_REQUEST)
            .body("invalid header".into())?);
    }

    // Read the version: if the version mismatches, it's innocuous.
    let version = reader.read_u8().await?;
    if version != tango_pvp::replay::VERSION {
        return Ok(hyper::Response::builder()
            .status(hyper::http::StatusCode::BAD_REQUEST)
            .body(format!("unsupported version: {:02x}", version).into())?);
    }

    // Read the number of inputs: 0 is incomplete.
    let num_inputs = reader.read_u32_le().await?;
    if num_inputs == 0 {
        return Ok(hyper::Response::builder()
            .status(hyper::http::StatusCode::BAD_REQUEST)
            .body("not complete".into())?);
    }
    log::info!("read num inputs: {}", num_inputs);

    // Read the metadata.
    let metadata_len = reader.read_u32_le().await? as usize;
    if metadata_len == 0 || metadata_len > MAX_METADATA_LEN {
        return Ok(hyper::Response::builder()
            .status(hyper::http::StatusCode::BAD_REQUEST)
            .body(format!("metadata too long: {}", metadata_len).into())?);
    }

    let mut metadata_buf = vec![0u8; metadata_len];
    reader.read_exact(&mut metadata_buf).await?;

    let (_, mut metadata) = tango_pvp::replay::read_metadata(&mut &metadata_buf[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if let Some(side) = metadata.local_side.as_mut() {
        side.nickname = "".to_string();
    }

    if let Some(side) = metadata.remote_side.as_mut() {
        side.nickname = "".to_string();
    }

    let game_info = if let Some(side) = metadata
        .local_side
        .as_ref()
        .and_then(|side| side.game_info.as_ref())
    {
        side
    } else {
        return Ok(hyper::Response::builder()
            .status(hyper::http::StatusCode::BAD_REQUEST)
            .body(hyper::Body::empty())?);
    };

    log::info!("received metadata: {:?}", metadata);

    // We are only collecting ROCKEXE4.5ROBR4J real_bn_gameplay data.
    if game_info.rom_family != "exe45"
        || game_info.rom_variant != 0
        || !(game_info
            .patch
            .as_ref()
            .map(|p| p.name == "exe45_pvp")
            .unwrap_or(false)
            || game_info
                .patch
                .as_ref()
                .map(|p| p.name == "bn45_us_pvp")
                .unwrap_or(false))
    {
        return Ok(hyper::Response::builder().body(hyper::Body::empty())?);
    }

    let filename = uuid::Uuid::new_v4().to_string();
    let path = pending_replays_dir.join(format!("{}.tangoreplayincomplete", filename));

    {
        let mut f = tokio::fs::File::create(path.clone()).await?;

        let metadata_buf = metadata.encode_to_vec();

        f.write_all(tango_pvp::replay::HEADER).await?;
        f.write_u8(tango_pvp::replay::VERSION).await?;
        f.write_u32_le(num_inputs).await?;
        f.write_u32_le(metadata_buf.len() as u32).await?;
        f.write_all(&metadata_buf).await?;
        tokio::io::copy(&mut reader, &mut f).await?;
    }
    tokio::fs::rename(path.clone(), path.with_extension("tangoreplay")).await?;

    Ok(hyper::Response::builder().body(hyper::Body::empty())?)
}

fn router(
    pending_replays_dir: std::path::PathBuf,
) -> routerify::Router<hyper::Body, anyhow::Error> {
    routerify::Router::builder()
        .data(State {
            pending_replays_dir,
        })
        .post("/", handle_submit_request)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter(Some("collectorserver"), log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    std::fs::create_dir_all(&args.pending_replays_dir)?;

    let addr = args.listen_addr.parse()?;
    let router = router(args.pending_replays_dir);
    let service = routerify::RouterService::new(router).unwrap();
    hyper::Server::bind(&addr).serve(service).await?;
    Ok(())
}
