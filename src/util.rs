// https://doc.rust-lang.org/beta/std/task/trait.Wake.html
pub fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    struct ThreadWaker(std::thread::Thread);

    impl std::task::Wake for ThreadWaker {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }

    let mut fut = std::pin::pin!(future);

    let t = std::thread::current();
    let waker = std::sync::Arc::new(ThreadWaker(t)).into();
    let mut cx = std::task::Context::from_waker(&waker);

    loop {
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(res) => return res,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

pub const fn as_bytes<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref::<T>(p).cast::<u8>(),
            std::mem::size_of::<T>(),
        )
    }
}

pub const fn slice_as_bytes<T: Sized>(p: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p.as_ptr().cast::<u8>(), std::mem::size_of_val(p)) }
}
