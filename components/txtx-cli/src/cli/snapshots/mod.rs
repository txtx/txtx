use super::{BeginSnapshot, CommitSnapshot, Context};

pub async fn handle_begin_command(cmd: &BeginSnapshot, _ctx: &Context) -> Result<(), String> {
    // Create a .lock file on a DB file path specified
    // Write state transitions to it
    Ok(())
}

pub async fn handle_commit_command(cmd: &CommitSnapshot, _ctx: &Context) -> Result<(), String> {
    // Check for .lock file
    // Write state transitions accumulated to db file specified
    Ok(())
}
