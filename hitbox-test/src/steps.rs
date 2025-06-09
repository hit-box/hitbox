use crate::core::{HitboxWorld, StepExt};

use anyhow::{anyhow, Error};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use hitbox::policy::PolicyConfig;
use hurl::{
    runner::{request::eval_request, VariableSet},
    util::path::ContextDir,
};
use hurl_core::{error::DisplaySourceError, parser::parse_hurl_file, text::Format};

///////////// GIVEN ////////////

#[given(regex = r"hitbox with policy")]
fn hitbox(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    dbg!(step.docstring_content());
    let policy = step
        .docstring_content()
        .as_deref()
        .map(serde_yaml::from_str::<PolicyConfig>)
        .transpose()?
        .unwrap_or_default();
    world.settings.policy = policy;
    Ok(())
}

#[given(expr = "request predicates")]
async fn request_predicates(_world: &mut HitboxWorld) -> Result<(), Error> {
    Ok(())
}

#[given(expr = "key extractor {string}")]
async fn key_extractor(world: &mut HitboxWorld, extractor: String) -> Result<(), Error> {
    Ok(())
}

///////////// WHEN ////////////

#[when(expr = "execute request")]
async fn execute(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let hurl_request = step
        .docstring_content()
        .ok_or_else(|| anyhow!("request not provided"))?;
    let hurl_file = parse_hurl_file(&hurl_request).map_err(|err| {
        anyhow!(
            "hurl request parse error: {}",
            &err.message(&hurl_request.lines().collect::<Vec<_>>())
                .to_string(Format::Ansi)
        )
    })?;
    let variables = VariableSet::new();
    let request = &hurl_file
        .entries
        .first()
        .ok_or_else(|| anyhow!("request not found"))?
        .request;
    let request = eval_request(request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;
    world.execute_request(&request).await?;
    Ok(())
}

///////////// THEN ////////////

#[then(expr = "response status is {int}")]
async fn check_response_status(world: &mut HitboxWorld, status: u16) -> Result<(), Error> {
    match &world.state.response {
        Some(response) => {
            if response.status_code().eq(&status) {
                Ok(())
            } else {
                Err(anyhow!(
                    "expected response status code is {}, received is {}",
                    response.status_code(),
                    status
                ))
            }
        }
        None => Err(anyhow!("request was not executed")),
    }
}

#[then(expr = "cache has records")]
async fn check_cache_backend_state(_world: &mut HitboxWorld) -> Result<(), Error> {
    Ok(())
}
