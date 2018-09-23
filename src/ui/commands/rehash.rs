use super::*;

pub(crate) fn rehash_command(ctx: &Context) -> CommandResult {
    if let Ok(config) = Config::load("riirc.toml") {
        ctx.config.replace(config);
        Ok(Response::Nothing)
    } else {
        Err(Error::ReloadConfig)
    }
}
