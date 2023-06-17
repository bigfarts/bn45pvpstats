use futures::StreamExt;
use itertools::Itertools;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_default_env()
        .filter(Some("patchessyncserver"), log::LevelFilter::Info)
        .init();

    let root = std::path::Path::new("patches");
    std::fs::create_dir_all(root)?;

    let client = reqwest::Client::new();
    let entries = tokio::time::timeout(
        // 30 second timeout to fetch JSON.
        std::time::Duration::from_secs(30),
        (|| async {
            Ok::<_, anyhow::Error>(
                client
                    .get("https://patches.tango.n1gp.net/index.json")
                    .header("User-Agent", "tango")
                    .send()
                    .await?
                    .json::<tango_filesync::Entries>()
                    .await?,
            )
        })(),
    )
    .await??
    .into_iter()
    .filter(|(k, _)| k == "exe45_pvp" || k == "bn45_us_pvp")
    .collect::<std::collections::HashMap<String, tango_filesync::Entry>>();

    tango_filesync::sync(
        &root,
        &entries,
        {
            let root = root.to_path_buf();
            move |path| {
                let root = root.clone();
                Box::pin(async move {
                    let mut output_file = tokio::fs::File::create(&root.join(path)).await?;
                    let client = reqwest::Client::new();
                    let mut stream = tokio::time::timeout(
                        // 30 second timeout to initiate connection.
                        std::time::Duration::from_secs(30),
                        client
                            .get(format!(
                                "https://patches.tango.n1gp.net/{}",
                                path.components()
                                    .map(|v| v.as_os_str().to_string_lossy())
                                    .join("/")
                            ))
                            .header("User-Agent", "tango")
                            .send(),
                    )
                    .await?
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                    .bytes_stream();
                    while let Some(chunk) = tokio::time::timeout(
                        // 30 second timeout per stream chunk.
                        std::time::Duration::from_secs(30),
                        stream.next(),
                    )
                    .await?
                    {
                        let chunk =
                            chunk.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                        output_file.write_all(&chunk).await?;
                    }
                    log::info!("filesynced: {}", path.display());
                    Ok(())
                })
            }
        },
        4,
    )
    .await?;

    Ok(())
}
