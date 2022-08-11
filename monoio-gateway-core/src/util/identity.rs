use crate::service::Layer;

#[derive(Default)]
pub struct Identity {
    _p: (),
}

impl Identity {
    pub fn new() -> Self {
        Self { _p: () }
    }
}

impl<S> Layer<S> for Identity {
    type Service = S;

    fn layer(&self, inner: S) -> Self::Service {
        inner
    }
}
