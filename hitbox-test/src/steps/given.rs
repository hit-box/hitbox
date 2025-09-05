use crate::core::{HitboxWorld, StepExt};
use anyhow::Error;
use cucumber::gherkin::Step;
use cucumber::given;
use hitbox::policy::PolicyConfig;

#[given(regex = r"hitbox with policy")]
fn hitbox_with_policy(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let policy = step
        .docstring_content()
        .as_deref()
        .map(serde_yaml::from_str::<PolicyConfig>)
        .transpose()?
        .unwrap_or_default();
    world.settings.policy = policy;
    Ok(())
}
