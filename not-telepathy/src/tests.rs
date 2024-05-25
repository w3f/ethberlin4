use crate::ethereum;
use anyhow::Result;
use ark_bn254::{Bn254, G1Affine, G2Affine};
use ark_groth16::Groth16;
use ark_std::rand::thread_rng;

use ethers::{
    contract::ContractError,
    prelude::abigen,
    providers::{Http, Middleware, Provider},
    utils::Anvil,
};
use std::{convert::TryFrom, sync::Arc};

// We need to implement the conversion from the Ark-Circom's internal Ethereum types to
// the ones expected by the abigen'd types. Could we maybe provide a convenience
// macro for these, given that there's room for implementation error?
abigen!(Groth16Verifier, "src/verifier_artifact.json");
use groth_16_verifier::{G1Point, G2Point, Proof, VerifyingKey};
impl From<ethereum::G1> for G1Point {
    fn from(src: ethereum::G1) -> Self {
        Self { x: src.x, y: src.y }
    }
}
impl From<ethereum::G2> for G2Point {
    fn from(src: ethereum::G2) -> Self {
        // We should use the `.as_tuple()` method which handles converting
        // the G2 elements to have the second limb first
        let src = src.as_tuple();
        Self { x: src.0, y: src.1 }
    }
}
impl From<ethereum::Proof> for Proof {
    fn from(src: ethereum::Proof) -> Self {
        Self {
            a: src.a.into(),
            b: src.b.into(),
            c: src.c.into(),
        }
    }
}
impl From<ethereum::VerifyingKey> for VerifyingKey {
    fn from(src: ethereum::VerifyingKey) -> Self {
        Self {
            alfa_1: src.alpha1.into(),
            beta_2: src.beta2.into(),
            gamma_2: src.gamma2.into(),
            delta_2: src.delta2.into(),
            ic: src.ic.into_iter().map(|i| i.into()).collect(),
        }
    }
}

impl<M: Middleware> Groth16Verifier<M> {
    async fn check_proof<
        I: Into<ethereum::Inputs>,
        P: Into<ethereum::Proof>,
        VK: Into<ethereum::VerifyingKey>,
    >(
        &self,
        proof: P,
        vk: VK,
        inputs: I,
    ) -> Result<bool, ContractError<M>> {
        // convert into the expected format by the contract
        let proof = proof.into().into();
        let vk = vk.into().into();
        let inputs = inputs.into().0;

        // query the contract
        let res = self.verify(inputs, proof, vk).call().await?;

        Ok(res)
    }
}

#[tokio::test]
async fn solidity_verifier() -> Result<()> {
    let mut rng = thread_rng();
    // let proof = Groth16::<Bn254>::prove(&params, circom, &mut rng)?;

    // launch the network & compile the verifier
    let anvil = Anvil::new().spawn();
    let acc = anvil.addresses()[0];
    let provider = Provider::<Http>::try_from(anvil.endpoint())?;
    let provider = provider.with_sender(acc);
    let provider = Arc::new(provider);

    // deploy the verifier
    let contract = Groth16Verifier::deploy(provider.clone(), ())?
        .send()
        .await?;

    // // check the proof
    // let verified = contract
    //     .check_proof(proof, params.vk, inputs.as_slice())
    //     .await?;

    // assert!(verified);

    Ok(())
}
