use super::*;

pub(crate) fn clear_history_command(ctx: &Context) -> CommandResult {
    let (index, _) = ctx.state.buffers().current();
    ctx.request(Request::ClearHistory(index));
    Ok(Response::Nothing)
}
