use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use std::future::Future;
use tokio::task;
use std::io;

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub async fn new(socket_path: &str) -> io::Result<Self> {
        tokio::fs::remove_file(socket_path).await.ok();
        let listener = UnixListener::bind(socket_path)?;
        Ok(Self { listener })
    }

    pub async fn start<F, Fut>(&self, handle_request: F) -> io::Result<()>
    where
        F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let handle_request = handle_request.clone();

            task::spawn(async move {
                let mut reader = BufReader::new(stream);
                let mut buffer = String::new();

                if reader.read_line(&mut buffer).await.is_ok() {
                    let response = handle_request(buffer.trim().to_string()).await;
                    if let Err(e) = reader.get_mut().write_all(response.as_bytes()).await {
                        eprintln!("Error writing response: {:?}", e);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_ipc_server() {
        let socket_path = "/tmp/test.sock";
        let server = IpcServer::new(socket_path).await.unwrap();

        tokio::spawn(async move {
            server.start(|msg| async move { format!("Echo: {}", msg) }).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut stream = tokio::net::UnixStream::connect(socket_path).await.unwrap();
        stream.write_all(b"Hello\n").await.unwrap();

        let mut response = String::new();
        let mut reader = BufReader::new(&mut stream);
        reader.read_line(&mut response).await.unwrap();

        assert_eq!(response, "Echo: Hello");
    }
}