#![allow(dead_code)]

use crate::map::World;

#[derive(Clone, Debug)]
pub struct BehaviorContext {
    pub focus_world: World,
}

impl BehaviorContext {
    pub const fn new(focus_world: World) -> Self {
        Self { focus_world }
    }
}
