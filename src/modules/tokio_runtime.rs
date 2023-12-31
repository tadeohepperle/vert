use crate::Module;

pub struct TokioRuntime {
    rt: tokio::runtime::Runtime,
}

impl std::ops::Deref for TokioRuntime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.rt
    }
}

impl Module for TokioRuntime {
    type Config = ();

    type Dependencies = ();

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self { rt })
    }
}
