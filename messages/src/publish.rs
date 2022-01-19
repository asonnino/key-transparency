use crate::ensure;
use crate::error::{MessageError, MessageResult};
use config::Committee;
use crypto::{Digest, KeyPair, PublicKey, Signature};
use ed25519_dalek::Digest as _;
use ed25519_dalek::Sha512;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;

/// Represents a state proof.
pub type Proof = u64;

/// An item committed to the state.
pub type Item = u64;

/// The sequence number of consistent (or reliable) broadcast.
pub type SequenceNumber = u64;

/// A message that can be hashed.
pub trait PublishMessage {
    /// Return a reference to the root commitment.
    fn root(&self) -> &Digest;

    /// Return the sequence number of the message.
    fn sequence_number(&self) -> SequenceNumber;

    /// Compute the hash of the message.
    fn digest(&self) -> Digest {
        let mut hasher = Sha512::new();
        hasher.update(self.root());
        hasher.update(self.sequence_number().to_le_bytes());
        Digest(hasher.finalize().as_slice()[..32].try_into().unwrap())
    }
}

/// An publish notification sent by the IdP to the witnesses to request votes.
#[derive(Serialize, Deserialize)]
pub struct PublishNotification {
    /// The root committing to the new state.
    root: Digest,
    /// The state-transition proof ensuring the state publish is valid.
    proof: Proof,
    /// The item changing the state.
    items: Vec<Item>,
    /// The sequence number unique to this publish notification.
    sequence_number: SequenceNumber,
    /// The hash of the previous fields of this publish.
    id: Digest,
    /// A signature from the IdP authenticating the publish.
    signature: Signature,
}

impl std::fmt::Debug for PublishNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}: N{}({}, {})",
            self.id,
            self.sequence_number,
            self.root,
            self.items
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(",")
        )?;
        Ok(())
    }
}

impl PublishMessage for PublishNotification {
    fn root(&self) -> &Digest {
        &self.root
    }

    fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }
}

impl PublishNotification {
    /// Create a new PublishNotification signed by the IdP.
    pub fn new(
        root: Digest,
        proof: Proof,
        items: Vec<Item>,
        sequence_number: SequenceNumber,
        keypair: &KeyPair,
    ) -> Self {
        let notification = Self {
            root,
            proof,
            items,
            sequence_number,
            id: Digest::default(),
            signature: Signature::default(),
        };
        let id = notification.digest();
        let signature = Signature::new(&id, keypair);
        Self {
            id,
            signature,
            ..notification
        }
    }

    /// Verify a publish notification (very CPU-intensive).
    pub fn verify(&self, committee: &Committee) -> MessageResult<()> {
        // Ensure the id is well formed.
        ensure!(
            self.digest() == self.id,
            MessageError::MalformedNotificationId(self.id.clone())
        );

        // Verify the signature on the publish notification
        self.signature
            .verify(&self.id, &committee.identity_provider)?;

        // Verify the commit proof.
        unimplemented!();
    }
}

/// A vote for a publish notification.
#[derive(Serialize, Deserialize, Clone)]
pub struct PublishVote {
    /// The root commitment of the publish notification.
    root: Digest,
    /// The sequence number of the publish notification.
    sequence_number: SequenceNumber,
    /// The witness creating the vote.
    author: PublicKey,
    /// A signature authenticating the vote.
    signature: Signature,
}

impl std::fmt::Debug for PublishVote {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}: V{}({}, {})",
            self.digest(),
            self.sequence_number,
            self.author,
            self.root
        )
    }
}

impl PublishMessage for PublishVote {
    fn root(&self) -> &Digest {
        &self.root
    }

    fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }
}

impl PublishVote {
    /// Create a new vote for a publish notification (signed by a witness).
    pub fn new(notification: &PublishNotification, keypair: &KeyPair) -> Self {
        let vote = Self {
            root: notification.root.clone(),
            sequence_number: notification.sequence_number,
            author: keypair.public(),
            signature: Signature::default(),
        };
        Self {
            signature: Signature::new(&vote.digest(), keypair),
            ..vote
        }
    }

    /// Verify that the vote is correctly signed.
    pub fn verify(&self, committee: &Committee) -> MessageResult<()> {
        // Ensure the authority has voting rights.
        ensure!(
            committee.voting_power(&self.author) > 0,
            MessageError::UnknownWitness(self.author)
        );

        // Check the signature.
        self.signature
            .verify(&self.digest(), &self.author)
            .map_err(MessageError::from)
    }
}

/// A certificate over a publish notification.
#[derive(Serialize, Deserialize)]
pub struct PublishCertificate {
    /// The root commitment of the certified notification.
    root: Digest,
    /// The sequence number of the publish notification.
    sequence_number: SequenceNumber,
    /// The quorum of votes making the certificate.
    votes: Vec<(PublicKey, Signature)>,
}

impl std::fmt::Debug for PublishCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}: C{}({})",
            self.digest(),
            self.sequence_number,
            self.root
        )
    }
}

impl PublishMessage for PublishCertificate {
    fn root(&self) -> &Digest {
        &self.root
    }

    fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }
}

impl PublishCertificate {
    /// Verify that certificate.
    pub fn verify(&self, committee: &Committee) -> MessageResult<()> {
        // Ensure the certificate has a quorum.
        let mut weight = 0;
        let mut used = HashSet::new();
        for (name, _) in self.votes.iter() {
            ensure!(!used.contains(name), MessageError::WitnessReuse(*name));
            let voting_power = committee.voting_power(name);
            ensure!(voting_power > 0, MessageError::UnknownWitness(*name));
            used.insert(*name);
            weight += voting_power;
        }
        ensure!(
            weight >= committee.quorum_threshold(),
            MessageError::CertificateRequiresQuorum
        );

        // Check the signatures.
        Signature::verify_batch(&self.digest(), &self.votes).map_err(MessageError::from)
    }
}
