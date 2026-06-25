use std::{env, path::PathBuf, process::ExitCode};

use whatsvault_proof::{build_report, render_report};

fn main() -> ExitCode {
    let roots = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    match build_report(roots) {
        Ok(report) => {
            print!("{}", render_report(&report));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("WhatsVault proof failed: {error}");
            ExitCode::FAILURE
        }
    }
}
