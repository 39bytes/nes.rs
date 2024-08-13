use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub trait ToggleExt {
    fn toggle(&self);
}

impl ToggleExt for Arc<AtomicBool> {
    fn toggle(&self) {
        self.store(!self.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

pub trait ResultExt<T> {
    fn into_anyhow(self) -> anyhow::Result<T>;
}

impl<T> ResultExt<T> for Result<T, String> {
    fn into_anyhow(self) -> anyhow::Result<T> {
        self.map_err(anyhow::Error::msg)
    }
}
