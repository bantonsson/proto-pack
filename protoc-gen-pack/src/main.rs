use std::{
    env,
    io::{self, Read},
};

use log::{debug, info, LevelFilter};
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

mod args;
use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    // TODO set up logging using the parsed arguments as well?? Or env variables??
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%3f %Z)} - [{l}]: {m}{n}",
        )))
        .append(false)
        .build("log_file.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file_appender", Box::new(file_appender)))
        .build(
            Root::builder()
                .appender("file_appender")
                .build(LevelFilter::Debug),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();

    info!("Starting");

    let mut env_args = env::args();
    debug!("EARG -> {:?} {:?}", env_args, env_args.len());
    // The first arg is always the program name
    if env_args.len() > 1 {
        debug!("EARG !!");
        // TODO Handle the parsed value in some better way in the args package
        let _args = args::try_parse_from(env_args)?;
        return Ok(());
    }

    debug!("Handling!");

    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;

    protoc_gen_pack::execute(
        env_args.next().ok_or(anyhow!("Missing program name"))?,
        buf.as_slice(),
    )
    .unwrap();

    Ok(())
}
