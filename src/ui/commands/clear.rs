use super::*;

pub(crate) fn clear_command(ctx: &Context) -> CommandResult {
    ctx.request(Request::Clear(true));
    Ok(Response::Nothing)
}
