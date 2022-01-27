use super::*;
use config::Witness;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub fn keys() -> Vec<(PublicKey, KeyPair)> {
    let mut rng = StdRng::from_seed([0; 32]);
    (0..4)
        .map(|_| KeyPair::generate_keypair(&mut rng))
        .collect()
}

pub fn committee() -> Committee {
    let (identity_provider, _) = keys().pop().unwrap();
    Committee {
        identity_provider,
        witnesses: keys()
            .into_iter()
            .enumerate()
            .map(|(i, (name, _))| {
                (
                    name,
                    Witness {
                        voting_power: 1,
                        address: format!("127.0.0.1:{}", i as u16).parse().unwrap(),
                    },
                )
            })
            .collect(),
    }
}

#[test]
fn verify_notification() {
    let (_, identity_provider) = keys().pop().unwrap();
    let notification = PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ SequenceNumber::default(),
        /* keypair */ &identity_provider,
    );
    assert!(notification.verify(&committee(), &Root::default()).is_ok());
}
