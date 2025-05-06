use colored::Colorize;
use log::Level;

pub fn init() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| match record.level() {
            Level::Error => {
                out.finish(format_args!(
                    "{}{} {}",
                    "error".red().bold(),
                    ":".bold(),
                    message
                ));
            }
            Level::Warn => {
                out.finish(format_args!(
                    "{}{} {}",
                    "warning".yellow().bold(),
                    ":".bold(),
                    message
                ));
            }
            Level::Info | Level::Debug | Level::Trace => {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    jiff::Zoned::now().strftime("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ));
            }
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;
    
    Ok(())
}
