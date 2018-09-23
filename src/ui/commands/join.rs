use super::*;

pub(crate) fn join_command(ctx: &Context) -> CommandResult {
    assume_connected(&ctx)?;
    assume_args(&ctx, "try: /join <chan>")?;

    // TODO make this actually work on multiple channels + keys
    ctx.request(Request::Join(ctx.parts[0].to_owned()));
    Ok(Response::Nothing)
}
