use rexa::netlayer::Netlayer;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub trait NlFuture<Nl>: std::future::Future<Output = Result<Nl, BoxError>> {}
impl<Nl, F: std::future::Future<Output = Result<Nl, BoxError>>> NlFuture<Nl> for F {}

#[macro_export]
macro_rules! test_nl {
    ($make_nl:expr => { $($test:ident: $name:ident),* $(,)? }) => {
        $(
            #[test]
            fn $name() -> Result<(), BoxError> {
                $test($make_nl)
            }
        )*
    };
}

#[cfg(feature = "netlayer-mock")]
pub async fn make_mock_netlayer(
    test_name: &'static str,
    index: usize,
) -> Result<std::sync::Arc<rexa::netlayer::mock::MockNetlayer>, BoxError> {
    rexa::netlayer::mock::MockNetlayer::bind(format!("{test_name}-{index}")).map_err(From::from)
}

#[cfg(feature = "netlayer-datastream")]
pub async fn make_tcp_netlayer(
    _: &'static str,
    _: usize,
) -> Result<rexa::netlayer::datastream::TcpIpNetlayer, BoxError> {
    rexa::netlayer::datastream::TcpIpNetlayer::bind(&std::net::SocketAddr::from((
        [127, 0, 0, 1],
        0,
    )))
    .await
    .map_err(From::from)
}

#[cfg(target_family = "unix")]
pub async fn mktemp(dir: bool, prefix: impl AsRef<str>) -> Result<std::path::PathBuf, BoxError> {
    use std::ffi::OsStr;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::ffi::OsStringExt;
    let output = {
        let mut cmd = tokio::process::Command::new("mktemp");
        cmd.args(&["--tmpdir"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if dir {
            cmd.arg("--directory");
        }
        cmd.arg(format!("{}.XXXXXXXXX", prefix.as_ref()));
        cmd
    }
    .spawn()?
    .wait_with_output()
    .await?;
    if !output.status.success() {
        return Err(OsStr::from_bytes(&output.stderr)
            .to_string_lossy()
            .into_owned()
            .into());
    }
    let res = std::path::PathBuf::from(
        OsString::from_vec(output.stdout)
            .into_string()
            .unwrap()
            .trim(),
    );

    tracing::trace!(path = ?res, "mktemp");

    Ok(res)
}

#[cfg(all(target_family = "unix", feature = "netlayer-datastream"))]
pub async fn make_unix_netlayer(
    test_name: &'static str,
    index: usize,
) -> Result<rexa::netlayer::datastream::UnixNetlayer, BoxError> {
    let addr = if cfg!(target_os = "linux") {
        // use std::os::linux::net::SocketAddrExt;
        // FIX :: tokio unix socketaddr does not support as_abstract_namespace
        // std::os::unix::net::SocketAddr::from_abstract_name(format!("rexa-{test_name}-{index}"))?
        //format!("rexa-{test_name}-{index}.XXXXXXXXX")
        std::os::unix::net::SocketAddr::from_pathname(
            mktemp(false, format!("rexa-{test_name}.socket.{index}")).await?,
        )?
    } else {
        std::os::unix::net::SocketAddr::from_pathname(
            mktemp(false, format!("rexa-{test_name}.socket.{index}")).await?,
        )?
    };
    rexa::netlayer::datastream::UnixNetlayer::bind(&addr)
        .await
        .map_err(From::from)
}

#[cfg(feature = "netlayer-onion")]
pub async fn make_onion_netlayer(
    test_name: &'static str,
    index: usize,
) -> Result<rexa::netlayer::onion::OnionNetlayer<tor_rtcompat::PreferredRuntime>, BoxError> {
    use futures::StreamExt;
    use std::path::PathBuf;
    use tor_hsservice::{config::OnionServiceConfigBuilder, HsNickname};

    let (state_dir, cache_dir) = {
        (
            match std::env::var_os("REXA_TESTS_ARTI_STATE_DIR") {
                Some(d) => PathBuf::from(d),
                _ => mktemp(true, format!("rexa-{test_name}-{index}-arti_state")).await?,
            },
            match std::env::var_os("REXA_TESTS_ARTI_CACHE_DIR") {
                Some(d) => PathBuf::from(d),
                _ => mktemp(true, format!("rexa-{test_name}-{index}-arti_cache")).await?,
            },
        )
    };

    tracing::trace!(?state_dir, ?cache_dir, "making OnionNetlayer");

    let res = rexa::netlayer::onion::OnionNetlayer::new_bootstrapped(
        tor_rtcompat::PreferredRuntime::current()?,
        {
            let mut cfg =
                arti_client::config::TorClientConfigBuilder::from_directories(state_dir, cache_dir);
            cfg.address_filter().allow_onion_addrs(true);
            cfg.build()?
        },
        OnionServiceConfigBuilder::default()
            .nickname(HsNickname::new(format!("rexa_{test_name}-{index}"))?)
            .build()?,
    )
    .await
    .map_err(BoxError::from)?;
    let mut status_stream = res.service().status_events();
    let mut seen_shutdown = false;
    loop {
        let status = status_stream.next().await.unwrap();
        match status.state() {
            tor_hsservice::status::State::Bootstrapping => {}
            tor_hsservice::status::State::Running => {
                if let Some(time) = status.provisioned_key_expiration() {
                    tracing::warn!(node = %res.locator::<String, String>(), expiration_time = ?time, "onion service keys may expire");
                }
                break Ok(res);
            }
            tor_hsservice::status::State::Shutdown => {
                tracing::error!(node = %res.locator::<String, String>(), problem = ?status.current_problem(), "onion service unexpectedly shut down");
                match seen_shutdown {
                    true => {
                        break Err(format!(
                            "onion service unexpectedly shut down, problem: {:?}",
                            status.current_problem()
                        )
                        .into())
                    }
                    false => seen_shutdown = true,
                }
            }
            tor_hsservice::status::State::Degraded => {
                tracing::warn!(node = %res.locator::<String, String>(), problem = ?status.current_problem(), "onion service degraded, may not be reachable");
            }
            tor_hsservice::status::State::Recovering => {
                tracing::warn!(node = %res.locator::<String, String>(), problem = ?status.current_problem(), "onion service recovering, may not be reachable");
            }
            tor_hsservice::status::State::Broken => {
                tracing::error!(node = %res.locator::<String, String>(), problem = ?status.current_problem(), "onion service broken, may not be reachable");
                break Ok(res);
            }
            state => {
                tracing::warn!(node = %res.locator::<String, String>(), ?state, problem = ?status.current_problem(), "unrecognized onion service state")
            }
        }
    }
}
