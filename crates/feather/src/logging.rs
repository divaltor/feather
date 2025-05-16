use colored::Colorize;
use log::Level;
use tracing_subscriber::{
    fmt::{format::FmtSpan, time::ChronoLocal, writer::BoxMakeWriter},
    layer::SubscriberExt,
};

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

    let is_trace_level = true;
    let logger = BoxMakeWriter::new(std::io::stderr);

    let subscriber = tracing_subscriber::Registry::default().with(
        tracing_subscriber::fmt::layer()
            .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S.%f".to_string()))
            .with_thread_names(is_trace_level)
            .with_target(is_trace_level)
            .with_ansi(false)
            .with_writer(logger)
            .with_span_events(FmtSpan::ENTER),
    );

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global default");

    Ok(())
}
