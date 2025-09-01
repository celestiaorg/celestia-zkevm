#![allow(dead_code)]
use anyhow::Result;
use tonic::{Request, Response, Status};

use crate::config::config::Config;
use crate::proto::celestia::prover::v1::prover_server::Prover;
use crate::proto::celestia::prover::v1::{
    InfoRequest, InfoResponse, ProveStateMembershipRequest, ProveStateMembershipResponse, ProveStateTransitionRequest,
    ProveStateTransitionResponse,
};
use crate::prover::prover::BlockRangeExecProver;

pub struct ProverService {
    block_range_prover: BlockRangeExecProver,
}

impl ProverService {
    pub fn new(_config: Config) -> Result<Self> {
        let block_range_prover = BlockRangeExecProver::new();

        Ok(ProverService { block_range_prover })
    }
}

#[tonic::async_trait]
impl Prover for ProverService {
    async fn info(&self, _request: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        let response = InfoResponse {
            state_membership_verifier_key: "".to_string(),
            state_transition_verifier_key: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn prove_state_transition(
        &self,
        _request: Request<ProveStateTransitionRequest>,
    ) -> Result<Response<ProveStateTransitionResponse>, Status> {
        Err(Status::unimplemented("prove_state_transition is unimplemented"))
    }

    async fn prove_state_membership(
        &self,
        _request: Request<ProveStateMembershipRequest>,
    ) -> Result<Response<ProveStateMembershipResponse>, Status> {
        Err(Status::unimplemented("prove_state_membership is unimplemented"))
    }
}
