use crate::service::Layer;

pub struct Stack<I, O> {
    pub(crate) inner: I,
    pub(crate) outer: O,
}

impl<I, O> Stack<I, O> {
    pub fn new(inner: I, outer: O) -> Stack<I, O> {
        Self { inner, outer }
    }
}

impl<S, I, O> Layer<S> for Stack<I, O>
where
    I: Layer<S>,
    O: Layer<I::Service>,
{
    type Service = O::Service;

    fn layer(&self, service: S) -> Self::Service {
        let inner = self.inner.layer(service);

        self.outer.layer(inner)
    }
}
