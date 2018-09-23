use super::*;

pub(crate) fn connect_command(ctx: &Context) -> CommandResult {
    use super::irc::IrcClient;

    if ctx.state.client().is_some() {
        Err(Error::AlreadyConnected)?;
    };

    let config = &ctx.config.borrow();

    let output = Output::new()
        .fg(Color::Green)
        .add("connecting to ")
        .fg(Color::Cyan)
        .add(&config.server)
        .build();

    let client = irc::Client::connect(config.server.clone()).map_err(Error::ClientError)?;
    if !&config.pass.is_empty() {
        client.pass(&config.pass)
    }
    client.nick(&config.nick);
    client.user(&config.user, &config.real);
    ctx.state.set_client(client);

    Ok(Response::Output(output))
}
