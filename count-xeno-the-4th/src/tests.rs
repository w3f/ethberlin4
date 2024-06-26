use crate::ethereum;
use anyhow::Result;
use ark_bn254::Bn254;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::MontBackend;
use ark_groth16::Groth16;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::fields::nonnative::NonNativeFieldVar;
use ark_r1cs_std::R1CSVar;
use ark_relations::r1cs::ConstraintSystem;
use ark_snark::SNARK;
use ark_std::UniformRand;
use rand::rngs::OsRng;
use rand::Rng;
use snowball::{keys_to_limbs, ApkCircuit};

use ethers::{
    contract::ContractError,
    prelude::abigen,
    providers::{Http, Middleware, Provider},
    utils::Anvil,
};
use std::{convert::TryFrom, sync::Arc};

// We need to implement the conversion from the internal Ethereum types to
// the ones expected by the abigen'd types.
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
    let rng = &mut OsRng;

    let n = 3;
    let keys: Vec<ark_bls12_381::G1Affine> =
        (0..n).map(|_| ark_bls12_381::G1Affine::rand(rng)).collect();
    let bits: Vec<bool> = (0..n).map(|_| rng.gen_bool(0.9)).collect();
    // TODO: seed could be selected for more effiency
    let seed = ark_bls12_381::G1Affine::rand(rng);

    let cs = ConstraintSystem::<ark_bn254::Fr>::new_ref();
    let bit_vars = Vec::<Boolean<ark_bn254::Fr>>::new_constant(cs, bits.clone()).unwrap();
    let packed_bits = Boolean::le_bits_to_fp_var(&bit_vars)
        .unwrap()
        .value()
        .unwrap();
    let apk = keys
        .iter()
        .zip(bits.iter())
        .fold(
            seed.into_group(),
            |acc, (key, bit)| {
                if *bit {
                    acc + key
                } else {
                    acc
                }
            },
        );
    let apk = apk.into_affine();

    let circuit = ApkCircuit::<_, _, NonNativeFieldVar<ark_bls12_381::Fq, ark_bn254::Fr>>::new(
        keys.clone(),
        seed,
        packed_bits,
        apk,
    );

    // TODO: circuit can be empty
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();
    let proof = Groth16::<Bn254>::prove(&pk, circuit.clone(), rng).unwrap();

    let mut pi = keys_to_limbs(&keys);
    pi.push(packed_bits);
    let apk_bits: Vec<ark_ff::Fp<MontBackend<ark_bn254::FrConfig, 4>, 4>> = keys_to_limbs(&[apk]);
    pi.extend(apk_bits);

    // launch a local network
    let anvil = Anvil::new().spawn();
    let acc = anvil.addresses()[0];
    let provider = Provider::<Http>::try_from(anvil.endpoint())?;
    let provider = provider.with_sender(acc);
    let provider = Arc::new(provider);

    // deploy the verifier
    let contract = Groth16Verifier::deploy(provider.clone(), ())?
        .send()
        .await?;

    // check the proof
    let verified = contract.check_proof(proof, vk, pi.as_slice()).await?;

    assert!(verified);

    Ok(())
}
