use colored::Colorize;
use tmkpr_lib::error::TmkprError;

pub fn print_error(e: &anyhow::Error) {
    if let Some(te) = e.downcast_ref::<TmkprError>() {
        eprintln!("{} {}", "error:".red().bold(), te);
    } else {
        eprintln!("{} {}", "error:".red().bold(), e);
    }
}
