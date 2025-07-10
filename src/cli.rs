use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Opts {
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}
