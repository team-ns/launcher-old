use anyhow::{Context, Error, Result};
use futures::future::join_all;
use hyper::body::HttpBody;
use hyper::{Body, Client, Request, Uri};
use hyper_tls::HttpsConnector;
use std::fs;
use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct RemoteFile {
    pub name: String,
    pub size: u64,
}

const SMALL_SIZE: u64 = 1048576;
const CHUNK_SIZE: u64 = 512000;

pub async fn download(files: Vec<RemoteFile>, file_server: String) -> Result<(), Error> {
    let tasks = files
        .into_iter()
        .map(|file| {
            let file_server = file_server.clone();
            tokio::spawn(async move {
                let uri = format!("{}/{}", file_server, file.name).parse()?;
                if file.size >= SMALL_SIZE {
                    concurrent_download(file, uri).await
                } else {
                    single_thread_download(file, uri).await
                }
            })
        })
        .collect::<Vec<_>>();
    join_tasks(tasks).await
}

fn get_chunks(file: &RemoteFile) -> Vec<(u64, u64)> {
    let mut chunks = Vec::new();
    let chunk_num = file.size / CHUNK_SIZE;

    for chunk in 0..chunk_num {
        let size = if chunk == chunk_num - 1 {
            file.size
        } else {
            ((chunk + 1) * CHUNK_SIZE) - 1
        };
        chunks.push((chunk * CHUNK_SIZE, size));
    }
    chunks
}

async fn concurrent_download(hashed_file: RemoteFile, uri: Uri) -> Result<(), Error> {
    let (sender, mut receiver) = mpsc::channel(100);
    let total_size = hashed_file.size.clone();
    let mut file = create_file(Path::new(&hashed_file.name)).await?;
    let mut tasks = get_chunks(&hashed_file)
        .into_iter()
        .map(|chunk| {
            let mut sender = sender.clone();
            let uri = uri.clone();
            tokio::spawn(async move {
                let byte_range = format!("bytes={}-{}", chunk.0, chunk.1);
                let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
                let req = Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("Range", byte_range)
                    .header("Connection", "keep-alive")
                    .body(Body::empty())?;

                let mut resp = client.request(req).await?;
                let mut start_offset = chunk.0;
                while let Some(chunk) = resp.data().await {
                    let chunk = chunk?;
                    sender.send((start_offset, chunk.clone())).await?;
                    start_offset += chunk.len() as u64;
                }
                Ok::<(), Error>(())
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
        }
    }));
    join_tasks(tasks).await
}

async fn single_thread_download(hashed_file: RemoteFile, uri: Uri) -> Result<(), Error> {
    tokio::spawn(async move {
        let mut file = create_file(Path::new(&hashed_file.name)).await?;
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        let mut res = client.get(uri).await?;
        while let Some(chunk) = res.data().await {
            file.write_all(&chunk?).await?;
        }
        Ok::<(), Error>(())
    })
    .await
    .with_context(|| "Async task error!")?
}

async fn join_tasks(tasks: Vec<JoinHandle<Result<()>>>) -> Result<(), Error> {
    join_all(tasks)
        .await
        .into_iter()
        .map(|result| result.with_context(|| "Async task error!")?)
        .find(|result| result.is_err())
        .unwrap_or_else(|| Ok(()))
}

async fn create_file(path: &Path) -> Result<File, Error> {
    fs::create_dir_all(path.parent().unwrap())?;
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .read(true)
        .open(path)
        .await?;
    Ok(file)
}
