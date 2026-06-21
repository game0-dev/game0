use std::future::Future;

use super::runtime::{AppHandle, TaskOptions};
use super::{AppCx, Application};

pub(crate) struct AsyncRuntime<A: Application> {
    tokio: tokio::runtime::Runtime,
    app: AppHandle<A>,
}

impl<A: Application> AsyncRuntime<A> {
    pub(crate) fn new(options: &TaskOptions, app: AppHandle<A>) -> crate::Result<Self> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.enable_all();
        if let Some(worker_threads) = options.worker_threads {
            builder.worker_threads(worker_threads);
        }
        Ok(Self {
            tokio: builder.build()?,
            app,
        })
    }

    pub(crate) fn spawn_io<Fut, T, Then>(&self, fut: Fut, then: Then)
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        Then: FnOnce(&mut A, &mut AppCx<A>, T) + Send + 'static,
    {
        let app = self.app.clone();
        self.tokio.spawn(async move {
            let value = fut.await;
            app.run_on_ui(move |app, cx| {
                then(app, cx, value);
            });
        });
    }

    pub(crate) fn spawn_blocking<F, T, Then>(&self, job: F, then: Then)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
        Then: FnOnce(&mut A, &mut AppCx<A>, T) + Send + 'static,
    {
        let app = self.app.clone();
        self.tokio.spawn_blocking(move || {
            let value = job();
            app.run_on_ui(move |app, cx| {
                then(app, cx, value);
            });
        });
    }
}
