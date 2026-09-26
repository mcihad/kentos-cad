//! The cloud's own async runtime. HTTP needs tokio; the desktop runs Iced's
//! executor (a thread pool), where tokio's network types would not work. So
//! every request runs here, on two threads started with the first request
//! and kept for the life of the program, and its result comes back through
//! a channel any executor can wait on. Dropping the waiting future cancels
//! the request.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;

use crate::failure::ApiFailure;

fn runtime() -> Result<&'static Runtime, ApiFailure> {
    static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("kentos-bulut")
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| ApiFailure::local(format!("Bulut bağlantısı başlatılamadı: {e}")))
}

/// Runs `work` on the cloud's runtime; the returned future, awaited on any
/// executor, gives its result. Dropped before that, it stops the work.
pub(crate) fn run<T: Send + 'static>(
    work: impl Future<Output = Result<T, ApiFailure>> + Send + 'static,
) -> impl Future<Output = Result<T, ApiFailure>> + Send + 'static {
    let (mut tx, rx) = oneshot::channel();
    let started = runtime().map(|rt| {
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
        rx.await
            .unwrap_or_else(|_| Err(ApiFailure::local("Bulut isteği yarıda kaldı.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// A plain executor with no tokio in it, as Iced's thread pool is.
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
    fn work_runs_on_its_own_runtime_whoever_waits() {
        let answer = block_on(run(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(42)
        }));
        assert_eq!(answer, Ok(42));
    }

    #[test]
    fn dropping_the_wait_stops_the_work() {
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
