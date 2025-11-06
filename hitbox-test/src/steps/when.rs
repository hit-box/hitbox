use crate::core::{HitboxWorld, StepExt};
use anyhow::{Error, anyhow};
use cucumber::gherkin::Step;
use cucumber::when;
use hurl::{
    runner::{VariableSet, request::eval_request},
    util::path::ContextDir,
};
use hurl_core::{error::DisplaySourceError, parser::parse_hurl_file, text::Format};

#[when(expr = "execute request")]
async fn execute_request(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
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
    let parsed_request = &hurl_file
        .entries
        .first()
        .ok_or_else(|| anyhow!("request not found"))?
        .request;

    let request = eval_request(parsed_request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;

    world.execute_request(&request).await?;
    Ok(())
}

#[when(expr = "sleep {int}")]
async fn sleep(world: &mut HitboxWorld, secs: u16) -> Result<(), Error> {
    // If mock time is available, advance it instead of actually sleeping
    if let Some(mock_time) = &world.time_state.mock_time {
        mock_time.advance_secs(secs.into());
    } else {
        // Fall back to actual sleep if no mock time is set
        tokio::time::sleep(tokio::time::Duration::from_secs(secs.into())).await;
    }
    Ok(())
}
