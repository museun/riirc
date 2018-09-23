use super::*;

pub(crate) fn connect_command(ctx: &Context) -> CommandResult {
    use super::irc::IrcClient;

    if ctx.state.client().is_some() {
        Err(Error::AlreadyConnected)?;
    };

    let output = Output::new()
        .fg(Color::Green)
        .add("connecting to ")
        .fg(Color::Cyan)
        .add(&ctx.config.server())
        .build();

    let client = irc::Client::connect(ctx.config.server().clone()).map_err(Error::ClientError)?;
    if !&ctx.config.pass().is_empty() {
        client.pass(&ctx.config.pass())
    }
    client.nick(&ctx.config.nick());
    client.user(&ctx.config.user(), &ctx.config.real());
    ctx.state.set_client(client);

    Ok(Response::Output(output))
}
