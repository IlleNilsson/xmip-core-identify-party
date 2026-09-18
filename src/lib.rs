#![forbid(unsafe_code)]

//! Identify by party: whatever arrives here is from the Party the Location's
//! configuration names.
//!
//! A Party is the estate's one place to say who a partner is (ADR-0019
//! clause 4), and a Location that belongs to a single partner — their drop
//! folder, their mailbox, their leased line — can simply name that Party. This
//! identifier is built from that name and presents it for every Stream that
//! arrives on the Location, under [`xcore::mechanism::party`]. The value is
//! the name the Party carries under the `party` mechanism in the registry, or
//! the Party's id in its canonical form, so the second gate's registry lookup
//! resolves the claim to the Party the operator meant and to no other.
//!
//! The claim is **inferred**, never passed: the configuration said it and the
//! sender said nothing. It is presented however the Stream arrived, because a
//! scheduled pickup from a partner's server is as much that partner's as a
//! connection from it, and can be identified no other way. Its sibling
//! `endpoint` presents a free identity the same way; this one presents a name
//! the registry is expected to know. Naming a Party here grants nothing: a
//! Party is recognized, a role is granted, and the third gate still decides.
//!
//! Evidence this technology writes: `party.location`, the URI the Stream came
//! from. It reads no property and attaches no proof.

use identify::{IdentifyError, Presented, StreamArrival, TransportIdentifier};
use xcore::{Mechanism, PartyId};

/// The evidence name the arrival's source URI rides under.
pub const LOCATION: &str = "party.location";

/// Presents the Party one Location's configuration names.
#[derive(Clone, Debug)]
pub struct ConfiguredParty {
    party: String,
}

impl ConfiguredParty {
    /// Whatever arrives on the Location is from the Party the registry knows
    /// by this name.
    ///
    /// # Errors
    ///
    /// Where the configuration names no Party: an empty name would put a
    /// claim with no claimant on every record, so it is refused when the
    /// identifier is built.
    pub fn named(party: &str) -> Result<Self, IdentifyError> {
        let party = party.trim();
        if party.is_empty() {
            return Err(IdentifyError::new(
                "the Location's configuration names no Party",
            ));
        }

        Ok(Self {
            party: party.to_string(),
        })
    }

    /// Whatever arrives on the Location is from the Party with this id,
    /// presented in its canonical 8-4-4-4-12 form.
    #[must_use]
    pub fn identified(party_id: PartyId) -> Self {
        Self {
            party: party_id.to_string(),
        }
    }

    /// The Party the configuration names, as it is presented.
    #[must_use]
    pub fn party(&self) -> &str {
        &self.party
    }
}

impl TransportIdentifier for ConfiguredParty {
    fn mechanism(&self) -> Mechanism {
        xcore::mechanism::party()
    }

    fn identify(&self, arrival: &StreamArrival<'_>) -> Result<Option<Presented>, IdentifyError> {
        Ok(Some(
            Presented::inferred(self.mechanism(), &self.party)
                .with_evidence(LOCATION, arrival.source_uri()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stream::Stream;
    use xcore::{Arriving, Established, Layer, StreamId};

    fn stream() -> Stream {
        Stream::new(StreamId::new(1), b"<order/>".to_vec(), None)
    }

    #[test]
    fn what_arrives_on_the_location_is_from_the_party_the_configuration_names() {
        let stream = stream();
        let arrival = StreamArrival::new(&stream, Arriving::Detected, "file:///in/partner-x", &[]);

        let claim = ConfiguredParty::named("partner-x")
            .expect("a Party")
            .identify(&arrival)
            .expect("read")
            .expect("a claim");

        assert_eq!(claim.mechanism.name(), "party");
        assert_eq!(claim.value, "partner-x");
        assert_eq!(claim.established, Established::Inferred);
        assert_eq!(claim.layer(), Layer::Transport);
        assert_eq!(
            claim.evidence,
            vec![(LOCATION.to_string(), "file:///in/partner-x".to_string())]
        );
    }

    #[test]
    fn a_party_named_by_its_id_is_presented_in_the_canonical_form() {
        let stream = stream();
        let arrival = StreamArrival::new(&stream, Arriving::Scheduled, "sftp://partner/out", &[]);

        let claim = ConfiguredParty::identified(PartyId::new(0x2a))
            .identify(&arrival)
            .expect("read")
            .expect("a claim");

        assert_eq!(claim.value, "00000000-0000-0000-0000-00000000002a");
        assert_eq!(claim.established, Established::Inferred);
    }

    #[test]
    fn the_party_is_inferred_however_the_stream_arrived_and_whatever_the_sender_says() {
        let stream = stream();
        let facts = [("party".to_string(), "mallory".to_string())];
        let party = ConfiguredParty::named("partner-x").expect("a Party");

        for arriving in [Arriving::Pushed, Arriving::Detected, Arriving::Scheduled] {
            let arrival = StreamArrival::new(&stream, arriving, "https://xmip/in", &facts);

            let claim = party.identify(&arrival).expect("read").expect("a claim");

            assert_eq!(claim.value, "partner-x");
            assert_eq!(claim.established, Established::Inferred);
        }
    }

    #[test]
    fn a_configuration_naming_no_party_is_refused_when_the_identifier_is_built() {
        let failure = ConfiguredParty::named("").expect_err("no Party");

        assert_eq!(
            failure.to_string(),
            "the Location's configuration names no Party"
        );
    }

    #[test]
    fn naming_a_party_proves_nothing_and_attaches_no_proof() {
        let stream = stream();
        let arrival = StreamArrival::new(&stream, Arriving::Detected, "file:///in/x", &[]);
        let party = ConfiguredParty::named(" partner-x ").expect("a Party");

        let claim = party.identify(&arrival).expect("read").expect("a claim");

        assert_eq!(party.party(), "partner-x");
        assert!(!claim.mechanism.authenticates());
        assert!(format!("{claim:?}").contains("proof: []"));
    }
}
