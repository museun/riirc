use super::*;

pub(crate) fn bind_command(ctx: &Context) -> CommandResult {
    match (ctx.parts.get(0), ctx.parts.get(1)) {
        (None, None) => {
            let keybinds = &ctx.config.borrow().keybinds;
            for (k, v) in keybinds.iter() {
                let output = Output::new()
                    .fg(Color::Yellow)
                    .add(k.to_string())
                    .add(" -> ")
                    .fg(Color::Cyan)
                    .add(v.to_string())
                    .build();
                ctx.status(output)
            }
        }

        (Some(key), None) => {
            let keybinds = &ctx.config.borrow().keybinds;
            let ok = KeyRequest::parse(*key).and_then(|key| {
                keybinds.lookup(key).and_then(|v| {
                    let output = Output::new()
                        .fg(Color::Yellow)
                        .add(key.to_string())
                        .add(" -> ")
                        .fg(Color::Cyan)
                        .add(v.to_string())
                        .build();
                    ctx.status(output);
                    Some(())
                })
            });

            if ok.is_none() {
                let output = Output::new()
                    .fg(Color::Red)
                    .add("error: ")
                    .add("unknown command: ")
                    .fg(Color::Cyan)
                    .add(*key)
                    .build();
                ctx.status(output);
            }
        }

        (Some(key), Some(value)) => {
            let keybinds = &mut ctx.config.borrow_mut().keybinds;
            if let Some(req) = KeyRequest::parse(*key) {
                if let Some(v) = keybinds.lookup(req) {
                    let next = KeyType::from(*value);
                    let output = Output::new()
                        .fg(Color::Yellow)
                        .add(key.to_string())
                        .fg(Color::Cyan)
                        .add(" ")
                        .add(v.to_string())
                        .add(" -> ")
                        .fg(Color::BrightGreen)
                        .add(next.to_string())
                        .build();
                    ctx.status(output);
                    keybinds.insert(next, req);
                }
            } else {
                let output = Output::new()
                    .fg(Color::Red)
                    .add("error: ")
                    .add("unknown command: ")
                    .fg(Color::Cyan)
                    .add(*key)
                    .build();
                ctx.status(output);
            }
        }
        _ => {}
    }

    ctx.config.borrow().save();
    Ok(Response::Nothing)
}
