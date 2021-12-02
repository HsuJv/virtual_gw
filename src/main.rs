use clap::clap_app;
use env_logger::Env;

mod client;
mod common;
mod config;
mod server;
mod tunnel;

pub type AsyncReturn<T> = Result<T, Box<dyn std::error::Error>>;

fn init_log(log_level: &str) {
    // actuall we don't want people to set the env
    // just put some base32 encode of "MY_LOG"
    let env = Env::default().filter_or("JRHUOX2MIVLEKTA=", log_level);
    env_logger::init_from_env(env);
}

fn parse_args() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap_app!(myapp =>
        (version: "0.1")
        (author: "Jovi Hsu <jv.hsu@outlook.com>")
        (about: "A virtual gateway")
        (@arg config: -c --config +takes_value "Sets a config file")
        (@arg debug: -d --debug "Sets log_level to debug")
        (@arg log_level: -l --log_level +takes_value "One of (error[default], warn, info, debug, trace)\
        Note this value will overwrite -d settings")
        // (@subcommand test =>
        //     (about: "controls testing features")
        //     (version: "0.1")
        //     (author: "Jovi Hsu <jv.hsu@outlook.com>")
        //     (@arg verbose: -v --verbose "Print test information verbosely")
        // )
    )
    .get_matches();

    let config = matches.value_of("config").unwrap_or("conf.json");
    println!("Config file: {}", config);

    let log_level = {
        let debug = matches.is_present("debug");
        let log_level =
            matches
                .value_of("log_level")
                .unwrap_or_else(|| if debug { "debug" } else { "error" });
        match log_level {
            level @ "error"
            | level @ "warn"
            | level @ "info"
            | level @ "debug"
            | level @ "trace" => {
                println!("Log level {}", level);
                level
            }
            _ => "error",
        }
    };
    init_log(log_level);
    config::init_from_file(config)
}

#[tokio::main]
async fn main() -> AsyncReturn<()> {
    parse_args()?;
    if config::is_server() {
        server::start().await?;
    } else {
        client::start().await?;
    }
    Ok(())
}
