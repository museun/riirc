use super::*;

pub(crate) fn rehash_command(ctx: &Context) -> CommandResult {
    if let Some(config) = Config::load() {
        ctx.config.replace(config);
        Ok(Response::Nothing)
    } else {
        Err(Error::ReloadConfig)
    }
}
