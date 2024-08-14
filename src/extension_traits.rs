pub trait ResultExt<T> {
    fn into_anyhow(self) -> anyhow::Result<T>;
}

impl<T> ResultExt<T> for Result<T, String> {
    fn into_anyhow(self) -> anyhow::Result<T> {
        self.map_err(anyhow::Error::msg)
    }
}
