pub mod base;
pub(crate) mod lib;
pub(crate) mod stateful;

use anyhow::Result;
use stateful::{ActionErrorKind, StatefulAction};

#[async_trait::async_trait]
pub trait Planner: Send + Sync + std::fmt::Debug + dyn_clone::DynClone {
    async fn plan(&self) -> Result<Vec<StatefulAction<Box<dyn Action>>>>;
}

dyn_clone::clone_trait_object!(Planner);

#[async_trait::async_trait]
pub trait Action: Send + Sync + std::fmt::Debug + dyn_clone::DynClone {
    async fn execute(&self) -> Result<(), ActionErrorKind>;
    async fn revert(&self) -> Result<(), ActionErrorKind>;

    fn stateful(self) -> StatefulAction<Self>
    where
        Self: Sized,
    {
        StatefulAction::uncompleted(self)
    }
}

dyn_clone::clone_trait_object!(Action);

#[async_trait::async_trait]
impl Action for Box<dyn Action + 'static> {
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        self.as_ref().execute().await
    }

    async fn revert(&self) -> Result<(), ActionErrorKind> {
        self.as_ref().revert().await
    }
}
