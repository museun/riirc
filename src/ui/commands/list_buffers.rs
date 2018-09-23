use super::*;

pub(crate) fn list_buffers_command(ctx: &Context) -> CommandResult {
    let buffers = ctx.state.buffers().buffers(); // nice
    let len = buffers.len() - 1;

    let mut output = Output::new();
    output.fg(Color::White).add("buffers: ");

    for (n, buffer) in buffers.iter().enumerate() {
        output
            .fg(Color::BrightWhite)
            .add(format!("{}", n))
            .fg(Color::Cyan)
            .add(buffer.name());

        if n < len {
            output.add(",");
        }
    }

    ctx.status(output.build());
    Ok(Response::Nothing)
}
