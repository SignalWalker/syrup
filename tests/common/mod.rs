pub mod netlayers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFormat {
    Compact,
    Full,
    Pretty,
    Json,
}

pub fn initialize_tracing(
    log_format: LogFormat,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let log_filter = std::env::var("REXA_LOG_FILTER").unwrap_or("warn,rexa=info".to_owned());

    let tsub = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
            time::UtcOffset::current_local_offset().unwrap_or_else(|e| {
                tracing::warn!("couldn't get local time offset: {:?}", e);
                time::UtcOffset::UTC
            }),
            time::macros::format_description!("[hour]:[minute]:[second]"),
        ))
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_env_filter(log_filter);

    match log_format {
        LogFormat::Compact => tsub.compact().try_init(),
        LogFormat::Full => tsub.try_init(),
        LogFormat::Pretty => tsub.pretty().try_init(),
        LogFormat::Json => tsub.json().try_init(),
    }
}

#[cfg(feature = "tokio")]
pub fn initialize(
    log_format: LogFormat,
) -> Result<tokio::runtime::Runtime, Box<dyn std::error::Error + Send + Sync + 'static>> {
    initialize_tracing(log_format)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(From::from)
}

#[cfg(feature = "tokio")]
pub async fn connect_nodes<Nl: rexa::netlayer::Netlayer + Send + 'static>(
    a: Nl,
    b: Nl,
) -> Result<
    (
        rexa::captp::CapTpSession<Nl::Reader, Nl::Writer>,
        rexa::captp::CapTpSession<Nl::Reader, Nl::Writer>,
    ),
    netlayers::BoxError,
>
where
    Nl::Reader: Send,
    Nl::Writer: Send,
    Nl::Error: std::error::Error + Send + Sync + 'static,
{
    let locator_a = a.locator::<String, String>();

    tracing::debug!(a = %locator_a, b = %b.locator::<String, String>(), "connecting nodes");

    let ready_send = std::sync::Arc::new(tokio::sync::Notify::new());
    let ready_recv = ready_send.clone();
    let session_ab = tokio::spawn(async move {
        ready_send.notify_one();
        a.accept().await
    });
    let session_ba = tokio::spawn(async move {
        ready_recv.notified().await;
        tracing::trace!(local = %b.locator::<String, String>(), remote = %locator_a, "connecting...");
        b.connect(&locator_a).await
    });

    Ok((session_ab.await??, session_ba.await??))
}
