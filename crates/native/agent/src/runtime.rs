//! The agent's own async runtime (docs/adr/8056 §4), as the cloud client has one: HTTP needs tokio and the desktop runs
//! its own executor, where tokio's network types would not work. Every call runs here, on two threads started with the
//! first call and kept for the life of the program; its result comes back through a channel any executor can wait on.
//! Dropping the waiting future stops the call.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;

use crate::failure::{Failure, FailureKind};

/// The runtime, started on first use.
pub(crate) fn handle() -> Result<&'static Runtime, Failure> {
    static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("kentos-ajan")
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| {
            Failure::new(
                FailureKind::Network,
                format!("Yapay zekâ bağlantısı başlatılamadı: {e}"),
            )
        })
}

/// Runs `work` on the runtime; the returned future, awaited on any executor, gives its result. Dropped before that, it
/// stops the work.
pub(crate) fn run<T: Send + 'static>(
    work: impl Future<Output = Result<T, Failure>> + Send + 'static,
) -> impl Future<Output = Result<T, Failure>> + Send + 'static {
    let (mut tx, rx) = oneshot::channel();
    let started = handle().map(|rt| {
        rt.spawn(async move {
            let done = tokio::select! {
                result = work => Some(result),
                () = tx.closed() => None,
            };
            if let Some(result) = done {
                let _ = tx.send(result);
            }
        });
    });
    async move {
        started?;
        rx.await.unwrap_or_else(|_| {
            Err(Failure::new(
                FailureKind::Network,
                "Yapay zekâ isteği yarıda kaldı.",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// A plain executor with no tokio in it, as the desktop's is.
    fn block_on<F: Future>(f: F) -> F::Output {
        use std::pin::pin;
        use std::task::{Context, Poll, Wake, Waker};
        struct Thread(std::thread::Thread);
        impl Wake for Thread {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(Thread(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut f = pin!(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
            std::thread::park();
        }
    }

    #[test]
    fn work_runs_on_the_agents_runtime_whoever_waits_and_stops_when_dropped() {
        let answer = block_on(run(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(std::thread::current().name().map(str::to_owned))
        }));
        assert_eq!(answer, Ok(Some("kentos-ajan".to_owned())));
        let finished = Arc::new(AtomicBool::new(false));
        let flag = finished.clone();
        let waiting = run(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            flag.store(true, Ordering::SeqCst);
            Ok(())
        });
        drop(waiting);
        std::thread::sleep(Duration::from_millis(600));
        assert!(!finished.load(Ordering::SeqCst));
    }
}
