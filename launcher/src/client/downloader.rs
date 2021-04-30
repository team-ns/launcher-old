use anyhow::{Context, Error, Result};
use futures::future::join_all;
use hyper::body::HttpBody;
use hyper::{Body, Client, Request, Uri};
use hyper_tls::HttpsConnector;

use crate::runtime::webview::{EventProxy, WebviewEvent};
use futures::Future;
use launcher_api::validation::RemoteFile;
use std::fs;
use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinError;

const SMALL_SIZE: u64 = 1048576;
const CHUNK_SIZE: u64 = 512000;

pub async fn download(files: Vec<(String, RemoteFile)>, handler: EventProxy) -> Result<()> {
    let (progress_sender, mut receiver) = mpsc::unbounded_channel::<u64>();
    let total_size = files.iter().map(|file| file.1.size).sum::<u64>();
    type Download = (Vec<(String, RemoteFile)>, Vec<(String, RemoteFile)>);
    let (concurrent, single): Download = files
        .into_iter()
        .partition(|file| file.1.size <= SMALL_SIZE);

    let mut tasks = concurrent
        .into_iter()
        .map(|file| {
            let progress_sender = progress_sender.clone();
            tokio::spawn(
                async move { single_thread_download(file.1, file.0, progress_sender).await },
            )
        })
        .peekable();

    tokio::spawn(
        #[allow(unused_must_use)]
        async move {
            let mut receive_size = 0;
            handler.send_event(WebviewEvent::DispatchScript(format!(
                "app.backend.download.setTotalSize('{}')",
                total_size
            )));
            loop {
                if total_size == receive_size {
                    handler.send_event(WebviewEvent::DispatchScript(
                        "app.backend.download.wait()".to_string(),
                    ));
                    return;
                }
                match receiver
                    .recv()
                    .await
                    .with_context(|| "Incorrect downloaded size!")
                {
                    Ok(size) => {
                        receive_size += size;
                        handler.send_event(WebviewEvent::DispatchScript(format!(
                            "app.backend.download.updateSize('{}')",
                            receive_size
                        )));
                    }
                    Err(error) => {
                        handler.send_event(WebviewEvent::DispatchScript(format!(
                            "app.backend.error('{}')",
                            error
                        )));
                        return;
                    }
                }
            }
        },
    );

    while tasks.peek().is_some() {
        join_tasks(tasks.by_ref().take(100)).await?;
    }

    for file in single {
        concurrent_download(file.1, file.0, progress_sender.clone()).await?;
    }
    Ok(())
}

fn get_chunks(file_size: u64) -> Vec<(u64, u64)> {
    let mut chunks = Vec::new();
    let chunk_num = file_size / CHUNK_SIZE;

    for chunk in 0..chunk_num {
        let size = if chunk == chunk_num - 1 {
            file_size
        } else {
            ((chunk + 1) * CHUNK_SIZE) - 1
        };
        chunks.push((chunk * CHUNK_SIZE, size));
    }
    chunks
}

pub async fn concurrent_download(
    remote_file: RemoteFile,
    path: String,
    progress_sender: UnboundedSender<u64>,
) -> Result<()> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let total_size = remote_file.size;
    let mut file = create_file(Path::new(&path)).await?;
    let uri: Uri = remote_file.uri.parse()?;
    let mut tasks = get_chunks(remote_file.size - 1)
        .into_iter()
        .map(|chunk| {
            let sender = sender.clone();
            let uri = uri.clone();
            tokio::spawn(async move {
                let byte_range = format!("bytes={}-{}", chunk.0, chunk.1);
                let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
                let req = Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("Range", byte_range.clone())
                    .header("Connection", "keep-alive")
                    .body(Body::empty())?;

                let mut resp = client.request(req).await?;
                if resp.status().is_success() {
                    let mut start_offset = chunk.0;
                    while let Some(chunk) = resp.data().await {
                        let chunk = chunk?;
                        sender.send((start_offset, chunk.clone()))?;
                        start_offset += chunk.len() as u64;
                    }
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Can't download file, status code: {}",
                        resp.status()
                    ))
                }
            })
        })
        .collect::<Vec<_>>();

    tasks.push(tokio::spawn(async move {
        let mut receive_size = 0;
        loop {
            if total_size == receive_size {
                return Ok(());
            }
            let chunk = receiver.recv().await.with_context(|| "Incorrect chunk")?;
            file.seek(SeekFrom::Start(chunk.0)).await?;
            file.write_all(&chunk.1).await?;
            receive_size += chunk.1.len() as u64;
            progress_sender.send(chunk.1.len() as u64)?;
        }
    }));
    join_tasks(tasks).await
}

async fn single_thread_download(
    remote_file: RemoteFile,
    path: String,
    progress_sender: UnboundedSender<u64>,
) -> Result<()> {
    tokio::spawn(async move {
        let mut file = create_file(Path::new(&path)).await?;
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let mut resp = client.get(remote_file.uri.parse()?).await?;
        if resp.status().is_success() {
            while let Some(chunk) = resp.data().await {
                let bytes = &chunk?;
                file.write_all(bytes).await?;
                progress_sender.send(bytes.len() as u64)?;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Can't download file, status code: {}",
                resp.status()
            ))
        }
    })
    .await
    .with_context(|| "Async task error!")?
}

async fn join_tasks<I>(tasks: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: Future<Output = Result<Result<()>, JoinError>>,
{
    join_all(tasks)
        .await
        .into_iter()
        .map(|result| result.with_context(|| "Async task error!")?)
        .find(|result| result.is_err())
        .unwrap_or(Ok(()))
}

async fn create_file(path: &Path) -> Result<File, Error> {
    fs::create_dir_all(path.parent().unwrap())?;
    if path.is_file() {
        fs::remove_file(path)?;
    }
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(path)
        .await?;
    Ok(file)
}
