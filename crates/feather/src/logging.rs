use tracing_subscriber::{
    EnvFilter,
    fmt::{format::FmtSpan, time::ChronoLocal},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::new("info")
        .add_directive("reqwest=warn".parse()?)
        .add_directive("hyper=warn".parse()?)
        .add_directive("rustls=warn".parse()?)
        .add_directive("h2=warn".parse()?)
        .add_directive("tokio=warn".parse()?)
        .add_directive("tracing=warn".parse()?)
        .add_directive("feather=info".parse()?);

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
                .with_target(true)
                .with_thread_names(false)
                .with_ansi(true)
                .with_span_events(FmtSpan::CLOSE)
                .event_format(
                    tracing_subscriber::fmt::format()
                        .with_level(true)
                        .with_target(true)
                        .compact(),
                ),
        )
        .init();

    Ok(())
}
