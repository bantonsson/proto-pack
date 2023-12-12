use std::{ffi::OsString, fmt::Debug};

use anyhow::Result;
use clap::Parser;
use log::debug;

/// Simple args for the protoc plugin
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The default package name to use if none is specified in the proto file
    #[arg(short, long)]
    pub default_package_name: Option<String>,
}

pub fn try_parse_from<I, T>(itr: I) -> Result<Args>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone + Debug,
{
    let itr2 = itr.into_iter().map(|t| {
        debug!("I: {:?}", t);
        t
    });
    Ok(Args::try_parse_from(itr2)?)
}
