use super::*;

pub(crate) fn echo_command(ctx: &Context) -> CommandResult {
    for part in ctx.parts {
        let output = Output::new().add(*part).build();
        ctx.status(output);
    }
    Ok(Response::Nothing)
}
