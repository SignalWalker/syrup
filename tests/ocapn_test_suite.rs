use arti_client::TorClientConfig;
use common::{initialize_tracing, LogFormat};
use ed25519_dalek::VerifyingKey;
use rexa::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::{msg::DescImport, object::Object, AbstractCapTpSession, BootstrapEvent, CapTpSession},
    locator::SturdyRefLocator,
    netlayer::{onion::OnionNetlayer, Netlayer},
};
use std::{
    env,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::Arc,
};
use tokio::{process::Command, runtime::Runtime, sync::Notify, task::JoinSet};
use tor_hsservice::{config::OnionServiceConfigBuilder, HsNickname};

mod common;

const ENLIVENER_SWISS: &'static [u8] = b"gi02I1qghIwPiKGKleCQAOhpy3ZtYRpB";

fn initialize(
    log_format: LogFormat,
) -> Result<Runtime, Box<dyn std::error::Error + Send + Sync + 'static>> {
    initialize_tracing(log_format)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(Into::into)
}

#[test]
fn ocapn_test_suite() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let rt = initialize(LogFormat::Pretty)?;

    // prepare test runner command
    let suite_dir = env::var("REXA_OCAPN_TEST_SUITE_DIR")
        .map(PathBuf::from)?
        .canonicalize()?;
    let runner = suite_dir.join("test_runner.py");

    let mut python = Command::new(env::var("REXA_PYTHON_PATH").unwrap_or(String::from("python")));
    python
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    rt.block_on(async move {
        tracing::debug!("bootstrapping netlayer");

        let netlayer = OnionNetlayer::new_bootstrapped(
            tor_rtcompat::PreferredRuntime::current()?,
            TorClientConfig::default(),
            OnionServiceConfigBuilder::default()
                .nickname(HsNickname::new("rexa_ocapn_test_suite_rs".to_owned()).unwrap())
                .build()?,
        )
        .await?;

        let locator = netlayer.locator::<String, String>();
        let uri = locator.to_string();

        let modules = [
            "tests.op_start_session",
            "tests.op_abort",
            "tests.op_delivers",
        ];
        let mut module_args = Vec::new();
        for module in modules {
            module_args.push("--test-module");
            module_args.push(module);
        }

        python
            .args([runner.to_str().unwrap(), "--verbose"])
            .args(module_args)
            .arg(&uri);

        let (end_notifier, mut end_flag) = tokio::sync::watch::channel(false);
        end_flag.mark_unchanged();

        let ready_notif = Arc::new(Notify::new());
        let nl_task = tokio::spawn(manage_netlayer(netlayer, ready_notif.clone(), end_flag));

        ready_notif.notified().await;

        tracing::debug!(local_uri = %uri, cmd = ?python, "starting test runner");

        let mut proc = python.spawn()?;

        assert!(proc.wait().await?.success());

        end_notifier.send(true)?;

        nl_task.await?;

        Ok(())
    })
}

#[tracing::instrument]
async fn manage_netlayer<Nl: std::fmt::Debug + Netlayer>(
    nl: Nl,
    ready_notif: Arc<tokio::sync::Notify>,
    mut end_flag: tokio::sync::watch::Receiver<bool>,
) where
    Nl::Reader: AsyncRead + Unpin + Send + 'static,
    Nl::Writer: AsyncWrite + Unpin + Send + 'static,
    Nl::Error: std::error::Error,
{
    let mut session_tasks = JoinSet::new();
    ready_notif.notify_one();
    loop {
        tracing::trace!("awaiting session");
        let session = tokio::select! {
            session = nl.accept() => session.unwrap(),
            _ = end_flag.changed() => break
        };
        tracing::info!(?session, "accepted session");
        session_tasks.spawn(manage_session(session));
    }
    tracing::debug!("ending netlayer task");
    session_tasks.abort_all();
    while let Some(res) = session_tasks.join_next().await {
        res.unwrap().unwrap();
    }
}

#[tracing::instrument]
async fn manage_session<Reader, Writer>(
    session: CapTpSession<Reader, Writer>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    Reader: AsyncRead + Unpin + Send + 'static,
    Writer: AsyncWrite + Unpin + Send + 'static,
{
    loop {
        tracing::trace!("awaiting event");
        let event = session.recv_event().await?;
        tracing::debug!(?event, "received captp event");
        match event {
            rexa::captp::Event::Bootstrap(BootstrapEvent::Fetch { swiss, resolver }) => {
                match swiss.as_slice() {
                    ENLIVENER_SWISS => {
                        todo!()
                    }
                    swiss => {
                        return Err(format!(
                            "tried to fetch unrecognized swiss: {}",
                            String::from_utf8_lossy(swiss)
                        )
                        .into())
                    }
                }
            }
            rexa::captp::Event::Abort(reason) => {
                tracing::info!(%reason, "session aborted");
                break;
            }
        }
    }
    Ok(())
}

struct Enlivener;
impl Enlivener {
    // TODO :: figure out what kind of thing this is supposed to return
    async fn enliven<HKey, HVal>(
        &self,
        locator: &SturdyRefLocator<HKey, HVal>,
    ) -> Result<syrup::Item, Box<dyn std::error::Error + Send + Sync + 'static>> {
        todo!()
    }
}
impl Object for Enlivener {
    fn deliver_only(
        &self,
        session: &(dyn AbstractCapTpSession + Sync),
        _: Vec<syrup::Item>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Err("enlivener doesn't accept op:deliver-only".into())
    }

    fn deliver<'s>(
        &'s self,
        session: &(dyn AbstractCapTpSession + Sync),
        args: Vec<syrup::Item>,
        resolver: rexa::captp::GenericResolver,
    ) -> futures::future::BoxFuture<
        's,
        Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>,
    > {
        use futures::FutureExt;
        use syrup::Item;
        let mut args = args.into_iter();
        async move {
            match args.next() {
                Some(arg) => {
                    // FIX :: Probably shouldn't be using strings as the key here
                    match <SturdyRefLocator<String, Item> as syrup::FromSyrupItem>::from_syrup_item(
                        arg,
                    ) {
                        Ok(locator) => {
                            match self.enliven(&locator).await {
                                Ok(obj) => {
                                    resolver
                                        // TODO :: find out whether the answer_pos and resolve_me_desc matter here
                                        .fulfill(&[obj], None, DescImport::Object(0.into()))
                                        .await
                                        .map_err(From::from)
                                }
                                Err(e) => {
                                    return resolver
                                        .break_promise(e.to_string())
                                        .await
                                        .map_err(From::from)
                                }
                            }
                        }
                        Err(arg) => resolver
                            .break_promise(format!(
                                "invalid sturdyref: {}",
                                syrup::ser::to_pretty(&arg)?
                            ))
                            .await
                            .map_err(From::from),
                    }
                }
                None => resolver
                    .break_promise("missing sturdyref argument")
                    .await
                    .map_err(From::from),
            }
        }
        .boxed()
    }
}
