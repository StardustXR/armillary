use asteroids::{
    custom::{ElementTrait, FnWrapper},
    ValidState,
};
use stardust_xr_fusion::{core::schemas::zbus::Connection, spatial::SpatialRef};

pub struct RunOnCreate<State: ValidState> {
    on_create: FnWrapper<dyn Fn(&mut State) + Send + Sync + 'static>,
}
impl<State: ValidState> std::fmt::Debug for RunOnCreate<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOnCreate").finish()
    }
}
impl<State: ValidState> RunOnCreate<State> {
    pub fn new<F: Fn(&mut State) + Send + Sync + 'static>(f: F) -> Self {
        Self {
            on_create: FnWrapper(Box::new(f)),
        }
    }
}
impl<State: ValidState> ElementTrait<State> for RunOnCreate<State> {
    type Inner = RunOnCreateInner;
    type Resource = ();
    type Error = String;

    fn create_inner(
        &self,
        parent_space: &SpatialRef,
        dbus_connection: &Connection,
        resource: &mut Self::Resource,
    ) -> Result<Self::Inner, Self::Error> {
        Ok(RunOnCreateInner {
            spatial: parent_space.clone(),
            run: false,
        })
    }

    fn update(
        &self,
        old_decl: &Self,
        state: &mut State,
        inner: &mut Self::Inner,
        resource: &mut Self::Resource,
    ) {
        if !inner.run {
            inner.run = true;
            (self.on_create.0)(state);
        }
    }

    fn spatial_aspect(&self, inner: &Self::Inner) -> SpatialRef {
        inner.spatial.clone()
    }
}

pub struct RunOnCreateInner {
    spatial: SpatialRef,
    run: bool,
}
