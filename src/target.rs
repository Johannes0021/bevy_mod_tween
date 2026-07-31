use bevy_ecs::entity::Entity;

//==================================================================================================
// TweenTarget
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TweenTarget {
    This,
    Parent,
    Entity(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TweenTargetOptions {
    pub this: Entity,
    pub parent: Option<Entity>,
}

impl TweenTargetOptions {
    pub fn select(&self, target: TweenTarget) -> Option<Entity> {
        match target {
            TweenTarget::This => Some(self.this),
            TweenTarget::Parent => self.parent,
            TweenTarget::Entity(entity) => Some(entity),
        }
    }
}

//==================================================================================================
// TweenKeyTarget
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TweenKeyTarget {
    Derive,
    Custom(TweenTarget),
}
