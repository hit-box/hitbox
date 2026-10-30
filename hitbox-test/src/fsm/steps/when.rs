use crate::fsm::world::FsmWorld;
use anyhow::Error;
use cucumber::when;

// =============================================================================
// Request Execution Steps
// =============================================================================

#[when(expr = "{int} request is made with value {int}")]
async fn single_request(world: &mut FsmWorld, _num: usize, value: u32) -> Result<(), Error> {
    world.run_requests(1, value).await;
    Ok(())
}

#[when(expr = "{int} requests are made with value {int}")]
async fn multiple_requests(world: &mut FsmWorld, num: usize, value: u32) -> Result<(), Error> {
    world.run_requests(num, value).await;
    Ok(())
}

// =============================================================================
// Selective Request Execution Steps
// =============================================================================

#[when(expr = "{int} selective request is made with value {int}")]
async fn selective_request(world: &mut FsmWorld, _num: usize, value: u32) -> Result<(), Error> {
    world.run_selective_requests(1, value).await;
    Ok(())
}

#[when(expr = "{int} selective requests are made with value {int}")]
async fn selective_requests(world: &mut FsmWorld, num: usize, value: u32) -> Result<(), Error> {
    world.run_selective_requests(num, value).await;
    Ok(())
}
