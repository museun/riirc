use super::*;

pub(crate) fn exit_command(ctx: &Context) -> CommandResult {
    use super::irc::IrcClient;
    if let Some(client) = ctx.state.client() {
        client.quit(Some("leaving".into()));
    }
    Err(Error::ForceExit)
}
