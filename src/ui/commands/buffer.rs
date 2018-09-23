use super::*;

pub(crate) fn buffer_command(ctx: &Context) -> CommandResult {
    assume_args(&ctx, "try: /buffer N")?;

    // TODO get rid of this string
    let buf = ctx.parts[0]
        .parse::<usize>()
        .map_err(|_e| Error::InvalidArgument("try: /buffer N (a number this time)".into()))?;

    ctx.request(Request::SwitchBuffer(buf));
    Ok(Response::Nothing)
}
