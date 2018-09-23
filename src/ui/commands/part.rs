use super::*;

pub(crate) fn part_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;

    let (_, buf) = ctx.state.buffers().current();
    if buf.is_status() {
        // TODO get rid of this string
        return Err(Error::InvalidBuffer("cannot /part in a *window".into()));
    };

    let ch = if ctx.parts.is_empty() {
        buf.name().to_string()
    } else {
        ctx.parts[0].to_string()
    };

    ctx.request(Request::Part(ch));
    Ok(Response::Nothing)
}
