extern crate clap;

use clap::Shell;

include!("src/cli.rs");

fn main() {
    let outdir = match std::env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
    let mut app = build_cli();
    app.gen_completions("yablo", Shell::Bash, outdir.clone());
    app.gen_completions("yablo", Shell::Fish, outdir.clone());
    app.gen_completions("yablo", Shell::Zsh, outdir.clone());
}
