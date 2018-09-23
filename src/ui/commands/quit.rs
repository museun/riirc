use super::*;

pub(crate) fn quit_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;

    let msg = if ctx.parts.is_empty() {
        None
    } else {
        Some(ctx.parts.join(" "))
    };

    ctx.request(Request::Quit(msg));
    Ok(Response::Nothing)
}
