// Macro b/c easier than a blanket/default impl for futures::stream
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! impl_stream_for {
    ($Prog:ty, $Value:ty) => {
        impl futures::Stream for $Prog {
            type Item = $crate::event::Event<$Value>;
            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                ctx: &mut std::task::Context,
            ) -> std::task::Poll<Option<Self::Item>> {
                let timeout_immediate = std::time::Duration::from_millis(0);
                match self.ev_buf.poll(timeout_immediate) {
                    Ok(()) => match self.rx.try_recv().ok() {
                        Some(ev) => std::task::Poll::Ready(Some(ev)),
                        None => {
                            let waker = ctx.waker().clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                waker.wake();
                            });
                            std::task::Poll::Pending
                        }
                    },
                    Err(e) => {
                        log::error!("Error polling perf buffer: {:?}", e);
                        std::task::Poll::Ready(None)
                    }
                }
            }
        }
    };
}

pub(crate) use impl_stream_for;
