//! Lazily verified signatures.
//!
//! In the original implementation of `Cert::canonicalize`, all
//! self-signatures were verified.  This has turned out to be very
//! expensive.  Instead, we should only verify the signatures we are
//! actually interested in.
//!
//! To preserve the semantics, every self signature we hand out from
//! the `Cert` API must have been verified first.  However, we can do
//! that lazily.  And, when we reason over the cert (i.e. we are
//! looking for the right self-signature), we can search the
//! signatures without triggering the verification, and only verify
//! the one we are really interested in.

use std::{
    cmp::Ordering,
    mem,
    sync::{Arc, Mutex, OnceLock},
};

use crate::{
    Error,
    Result,
    packet::{
        Key,
        Signature,
        key,
        signature::subpacket::{SubpacketTag, SubpacketValue},
    },
};

/// Lazily verified signatures, similar to a `Vec<Signature>`.
///
/// We use two distinct vectors to store the signatures and their
/// state.  The reason for that is that we need to modify the
/// signature states while the signatures are borrowed.
///
/// We provide a subset of `Vec<Signature>`'s interface to make it
/// (mostly) a drop-in replacement.
///
/// # Invariant
///
/// - There are as many signatures as signature states.
///
/// - If the field `verified_sigs` is used, then there must have been
///   a bad signature (i.e. len(verified_sigs) < len(sigs)).
#[derive(Debug)]
pub struct LazySignatures {
    /// The primary key to verify the signatures with.
    primary_key: Arc<Key<key::PublicParts, key::PrimaryRole>>,

    /// The signatures.
    sigs: Vec<Signature>,

    /// The signature states.
    states: Mutex<Vec<SigState>>,

    /// Verified signatures.
    ///
    /// Because of https://gitlab.com/sequoia-pgp/sequoia/-/issues/638
    /// we have to hand out contiguous slices of verified signatures.
    /// If all signatures are good, we can serve that request from
    /// `sigs`.  Otherwise, we have to clone the verified signatures.
    ///
    /// XXXv2: Remove this field.
    verified_sigs: OnceLock<Vec<Signature>>,
}

impl PartialEq for LazySignatures {
    fn eq(&self, other: &Self) -> bool {
        self.assert_invariant();
        other.assert_invariant();
        self.primary_key == other.primary_key
            && self.sigs == other.sigs
    }
}

impl Clone for LazySignatures {
    fn clone(&self) -> Self {
        self.assert_invariant();
        LazySignatures {
            primary_key: self.primary_key.clone(),
            sigs: self.sigs.clone(),
            // Avoid blocking.  If we fail to get the lock, reset the
            // signature states to unverified in the clone.
            states: if let Ok(states) = self.states.try_lock() {
                states.clone()
            } else {
                vec![SigState::Unverified; self.sigs.len()]
            }.into(),
            verified_sigs: Default::default(),
        }
    }
}

/// Verification state of a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigState {
    /// Not yet verified.
    Unverified,

    /// Verification was successful.
    Good,

    /// Verification failed.
    Bad,
}

impl LazySignatures {
    /// Asserts the invariant.
    fn assert_invariant(&self) {
        debug_assert_eq!(self.sigs.len(), self.states.lock().unwrap().len());
        debug_assert!(
            self.verified_sigs.get().map(|v| v.len() < self.sigs.len())
                .unwrap_or(true));
    }

    /// Creates a vector of lazily verified signatures.
    ///
    /// The provided `primary_key` is used to verify the signatures.
    /// It should be shared across the certificate.
    pub fn new(primary_key: Arc<Key<key::PublicParts, key::PrimaryRole>>)
               -> Self
    {
        LazySignatures {
            primary_key,
            sigs: Default::default(),
            states: Default::default(),
            verified_sigs: Default::default(),
        }
    }

    /// Like [`Vec::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.assert_invariant();
        self.sigs.is_empty()
    }

    /// Like [`std::mem::take`].
    pub fn take(&mut self) -> Vec<Signature> {
        self.assert_invariant();
        self.states.lock().unwrap().clear();
        let r = mem::replace(&mut self.sigs, Vec::new());
        self.verified_sigs.take();
        self.assert_invariant();
        r
    }

    /// Like [`Vec::push`].
    pub fn push(&mut self, s: Signature) {
        self.assert_invariant();
        self.sigs.push(s);
        self.states.lock().unwrap().push(SigState::Unverified);
        self.verified_sigs.take();
        self.assert_invariant();
    }

    /// Like [`Vec::append`].
    pub fn append(&mut self, other: &mut LazySignatures) {
        // XXX check context
        self.assert_invariant();
        other.assert_invariant();
        self.sigs.append(&mut other.sigs);
        self.states.lock().unwrap().append(&mut other.states.lock().unwrap());
        self.verified_sigs.take();
        self.assert_invariant();
    }

    /// Like [`Vec::sort_by`].
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Signature, &Signature) -> Ordering,
    {
        self.assert_invariant();
        self.sigs.sort_by(compare);
        self.states.lock().unwrap().iter_mut().for_each(|p| *p = SigState::Unverified);
        self.verified_sigs.take();
        self.assert_invariant();
    }

    /// Like [`Vec::dedup_by`].
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut Signature, &mut Signature) -> bool,
    {
        self.assert_invariant();
        self.sigs.dedup_by(same_bucket);
        {
            let mut states = self.states.lock().unwrap();
            states.truncate(self.sigs.len());
            states.iter_mut().for_each(|p| *p = SigState::Unverified);
        }
        self.verified_sigs.take();
        self.assert_invariant();
    }

    /// Like [`Vec::iter_mut`], but gives out **potentially
    /// unverified** signatures.
    pub fn iter_mut_unverified(&mut self) -> impl Iterator<Item = &mut Signature> {
        self.assert_invariant();
        self.sigs.iter_mut()
    }

    /// Like [`Vec::into_iter`], but gives out **potentially
    /// unverified** signatures.
    pub fn into_unverified(self) -> impl Iterator<Item = Signature> {
        self.assert_invariant();
        self.sigs.into_iter()
    }

    /// Like [`Vec::as_slice`], but gives out **potentially
    /// unverified** signatures.
    pub fn as_slice_unverified(&self) -> &[Signature] {
        self.sigs.as_slice()
    }

    /// Like [`Vec::iter`], but only gives out verified signatures.
    ///
    /// If this is a subkey binding, `subkey` must be the bundle's
    /// subkey.
    pub fn iter_verified<'a>(&'a self,
                             subkey: Option<&'a Key<key::PublicParts, key::SubordinateRole>>)
                             -> impl Iterator<Item = &'a Signature> + 'a
    {
        self.iter_intern(subkey)
            .filter_map(|(state, s)| match state {
                SigState::Good => Some(s),
                SigState::Bad => None,
                SigState::Unverified => unreachable!(),
            })
    }

    /// Like [`Vec::iter`], but only gives out bad signatures.
    ///
    /// If this is a subkey binding, `subkey` must be the bundle's
    /// subkey.
    pub fn iter_bad<'a>(&'a self,
                        subkey: Option<&'a Key<key::PublicParts, key::SubordinateRole>>)
                        -> impl Iterator<Item = &'a Signature> + 'a
    {
        self.iter_intern(subkey)
            .filter_map(|(state, s)| match state {
                SigState::Good => None,
                SigState::Bad => Some(s),
                SigState::Unverified => unreachable!(),
            })
    }

    /// Like [`Vec::iter`], but lazily verifies the signatures.
    ///
    /// If this is a subkey binding, `subkey` must be the bundle's
    /// subkey.
    fn iter_intern<'a>(&'a self,
                       subkey: Option<&'a Key<key::PublicParts, key::SubordinateRole>>)
                       -> impl Iterator<Item = (SigState, &'a Signature)> + 'a
    {
        self.assert_invariant();
        self.sigs.iter().enumerate()
            .map(move |(i, s)| (self.verify_sig(i, subkey).expect("in bounds"), s))
    }

    /// Verifies the `i`th signature.
    ///
    /// If this is a subkey binding, `subkey` must be the bundle's
    /// subkey.
    ///
    /// Returns an error if `i` is out of bounds.
    pub fn verify_sig(&self, i: usize,
                      subkey: Option<&Key<key::PublicParts, key::SubordinateRole>>)
                      -> Result<SigState>
    {
        self.assert_invariant();
        if ! i < self.sigs.len() {
            return Err(Error::InvalidArgument(format!(
                "signature {} out of bound 0..{}", i, self.sigs.len())).into());
        }

        let state = self.states.lock().unwrap().get(i).cloned();
        match state {
            None => unreachable!("LazySignatures invariant violated"),
            Some(SigState::Unverified) => {
                let s = &self.sigs[i];
                let mut r = s.verify_signature(&self.primary_key);

                if r.is_ok() && subkey.is_some() &&
                    s.key_flags().map(|kf| kf.for_signing())
                    .unwrap_or(false)
                {
                    // The signature is good, but we still
                    // need to verify the back sig.
                    let mut any_backsig = Err(Error::BadSignature(
                        "Primary key binding signature missing".into()).into());

                    for backsig in s.subpackets(SubpacketTag::EmbeddedSignature)
                    {
                        let result =
                            if let SubpacketValue::EmbeddedSignature(sig) =
                            backsig.value()
                        {
                            sig.verify_primary_key_binding(
                                &self.primary_key, subkey.unwrap())
                        } else {
                            unreachable!("subpackets(EmbeddedSignature) \
                                          returns EmbeddedSignatures");
                        };
                        if result.is_ok() {
                            // Mark the subpacket as authenticated by the
                            // embedded signature.
                            backsig.set_authenticated(true);
                        }
                        if any_backsig.is_err() {
                            any_backsig = result;
                        }
                    }
                    r = any_backsig;
                }


                let state = if r.is_ok() {
                    SigState::Good
                } else {
                    SigState::Bad
                };

                // Remember the result.
                self.states.lock().unwrap()[i] = state.clone();
                Ok(state)
            },

            // Already verified, return the result.
            Some(state) => Ok(state),
        }
    }

    /// Returns a slice containing all verified signatures.
    ///
    /// This is used to implement the bundle's signature interface
    /// which gives out slices instead of iterators.  This is quite
    /// expensive to emulate, as it eagerly verifies all signatures,
    /// and if there are bad signatures, we need to copy all
    /// signatures into a new vector so that we can give out a
    /// continuous slice.
    // XXXv2: Remove this function.
    pub fn slice_verified(&self,
                          subkey: Option<&Key<key::PublicParts, key::SubordinateRole>>)
                          -> &[Signature]
    {
        self.assert_invariant();

        // First, verify all signatures.
        let verified = self.iter_verified(subkey).collect::<Vec<_>>();

        // Best case, all signatures are good, can use the vec.
        if self.states.lock().unwrap().iter().all(|s| *s == SigState::Good) {
            self.sigs.as_slice()
        } else {
            // Worst case, we have to make a copy of the good ones.
            self.verified_sigs.get_or_init(
                || verified.into_iter().cloned().collect())
        }
    }
}
