use anyhow::{Context, Result};

use server::Server;

fn main() -> Result<()> {
  let server: Server = Server::new("target/database.sql")
    .with_context(|| "Failed to initialize server")?;
  let tracks = server.get_all_tracks()
    .with_context(|| "Failed to get all tracks")?;
  for track in tracks {
    dbg!(track);
  }
  Ok(())
}
