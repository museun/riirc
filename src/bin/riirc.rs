#[macro_use]
extern crate log;
extern crate env_logger;

extern crate riirc;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    // TODO use structopt or something like that for this
    let mut args = ::std::env::args();
    if let Some(next) = args.nth(1) {
        match next.as_str() {
            "-c" | "--config" => {
                info!("generating a default toml config");
                let mut stdout = ::std::io::stdout();
                riirc::Config::default().dump(&mut stdout);
                return;
            }
            "-a" | "--attach" => {
                // this'll attach to the daemon
                return;
            }

            "-h" | "--help" | _ => {
                let help = &[
                    "-c, --config: writes a default config to stdout",
                    "-a, --attach: TODO",
                ];

                let help =
                    help.iter()
                        .map(|s| format!("\n\t{}", s))
                        .fold(String::new(), |mut a, c| {
                            a.push_str(&c);
                            a
                        });
                info!("{}", help);
                return;
            }
        }
    }

    let config = riirc::Config::load("riirc.toml")
        .map_err(|e| {
            error!("{}", e);
            ::std::process::exit(2);
        }).unwrap();

    macro_rules! check {
        ($e:expr) => {
            if $e.is_empty() {
                error!(
                    "'{}' field is missing from the config",
                    stringify!($e).split('.').last().unwrap()
                );
                ::std::process::exit(1);
            }
        };
    }

    check!(config.server);
    check!(config.nick);
    check!(config.user);
    check!(config.real);

    riirc::Gui::new(config).run();
}
