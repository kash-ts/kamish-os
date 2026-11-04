use crate::protocol::IpcMessage;
use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

/// Reads a line-delimited `IpcMessage` from the UnixStream
pub async fn read_message(reader: &mut BufReader<OwnedReadHalf>) -> Result<Option<IpcMessage>> {
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).await?;
    if bytes_read == 0 {
        return Ok(None);
    }
    let msg: IpcMessage = serde_json::from_str(line.trim_end())
        .with_context(|| format!("Failed to parse IPC message: {}", line))?;
    Ok(Some(msg))
}

/// Writes a line-delimited `IpcMessage` into the UnixStream
pub async fn write_message(writer: &mut BufWriter<OwnedWriteHalf>, msg: &IpcMessage) -> Result<()> {
    let mut json = serde_json::to_string(msg)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}
