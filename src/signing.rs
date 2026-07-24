//! N5 authenticity evidence.
//!
//! Ed25519 signatures are made over the raw 32-byte value of the already
//! verified canonical-payload SHA-256. The immutable report remains unchanged;
//! the signature is a separate, portable sidecar with explicit lineage.
//!
//! Production private-key bytes are stored only in the macOS data-protection
//! Keychain. Settings and project documents carry a key reference, public key,
//! and fingerprint only. The provider boundary deliberately prevents callers
//! from reading private key material.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use zeroize::Zeroize;

pub const SIGNATURE_SCHEMA: &str = "reyn.evidence-signature.v1";
pub const SIGNATURE_ALGORITHM: &str = "Ed25519";
pub const SIGNATURE_ENCODING: &str = "base64";
pub const SIGNED_MESSAGE_FORMAT: &str =
    "raw 32-byte SHA-256 digest decoded from signed_canonical_payload_sha256";
const KEYCHAIN_SERVICE: &str = "com.reyn-studio.evidence-signing";
const MAX_SIDECAR_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublicKeyRecord {
    pub key_id: String,
    pub algorithm: String,
    pub public_key_base64: String,
    pub key_fingerprint_sha256: String,
}

impl PublicKeyRecord {
    pub fn from_verifying_key(key_id: impl Into<String>, key: &VerifyingKey) -> Self {
        let public_key = key.to_bytes();
        Self {
            key_id: key_id.into(),
            algorithm: SIGNATURE_ALGORITHM.into(),
            public_key_base64: BASE64.encode(public_key),
            key_fingerprint_sha256: sha256_hex(&public_key),
        }
    }

    pub fn validate(&self) -> Result<VerifyingKey, SigningError> {
        if self.key_id.trim().is_empty() || self.key_id.len() > 128 {
            return Err(SigningError::MalformedKey(
                "key ID must contain 1–128 characters".into(),
            ));
        }
        if self.algorithm != SIGNATURE_ALGORITHM {
            return Err(SigningError::UnsupportedAlgorithm(self.algorithm.clone()));
        }
        if !is_sha256(&self.key_fingerprint_sha256) {
            return Err(SigningError::MalformedKey(
                "key fingerprint must be a lowercase SHA-256 digest".into(),
            ));
        }
        let bytes = BASE64
            .decode(&self.public_key_base64)
            .map_err(|_| SigningError::MalformedKey("public key is not valid base64".into()))?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| {
            SigningError::MalformedKey("Ed25519 public key must contain exactly 32 bytes".into())
        })?;
        if sha256_hex(&bytes) != self.key_fingerprint_sha256 {
            return Err(SigningError::KeyMismatch(
                "public key does not match its recorded fingerprint".into(),
            ));
        }
        VerifyingKey::from_bytes(&bytes)
            .map_err(|_| SigningError::MalformedKey("invalid Ed25519 public key".into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SigningLineage {
    pub run_id: String,
    pub report_schema: String,
    pub canonical_report_sha256: String,
    pub canonical_payload_sha256: String,
    pub created_utc_unix: u64,
}

impl SigningLineage {
    fn validate(&self) -> Result<[u8; 32], SigningError> {
        if uuid::Uuid::parse_str(&self.run_id).is_err() {
            return Err(SigningError::InvalidLineage(
                "source run ID must be a UUID".into(),
            ));
        }
        if self.report_schema.trim().is_empty() {
            return Err(SigningError::InvalidLineage(
                "source report schema is required".into(),
            ));
        }
        if !is_sha256(&self.canonical_report_sha256) || !is_sha256(&self.canonical_payload_sha256) {
            return Err(SigningError::InvalidLineage(
                "source report and canonical payload hashes must be lowercase SHA-256 digests"
                    .into(),
            ));
        }
        decode_sha256(&self.canonical_payload_sha256)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedSource {
    pub run_id: String,
    pub report_schema: String,
    pub canonical_report_sha256: String,
    pub canonical_payload_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthenticityRecord {
    pub status: String,
    pub algorithm: String,
    pub key_id: String,
    pub key_fingerprint_sha256: String,
    pub public_key_base64: String,
    pub signature_encoding: String,
    pub signature_bytes: String,
    pub signed_canonical_payload_sha256: String,
    pub signed_message_format: String,
    pub verification_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VerificationInstructions {
    pub offline_command: String,
    pub trust_instruction: String,
    pub revocation_instruction: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedEvidenceArtifact {
    pub signature_schema: String,
    pub created_utc_unix: u64,
    pub source: SignedSource,
    pub authenticity: AuthenticityRecord,
    pub verification: VerificationInstructions,
}

impl SignedEvidenceArtifact {
    pub fn to_json(&self) -> Result<String, SigningError> {
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| SigningError::Json(error.to_string()))?;
        json.push('\n');
        Ok(json)
    }

    pub fn from_json(json: &str) -> Result<Self, SigningError> {
        if json.len() > MAX_SIDECAR_BYTES {
            return Err(SigningError::Json(
                "signature sidecar exceeds the 256 KiB limit".into(),
            ));
        }
        serde_json::from_str(json).map_err(|error| SigningError::Json(error.to_string()))
    }

    pub fn content_sha256(&self) -> Result<String, SigningError> {
        Ok(sha256_hex(self.to_json()?.as_bytes()))
    }

    pub fn public_key_record(&self) -> PublicKeyRecord {
        PublicKeyRecord {
            key_id: self.authenticity.key_id.clone(),
            algorithm: self.authenticity.algorithm.clone(),
            public_key_base64: self.authenticity.public_key_base64.clone(),
            key_fingerprint_sha256: self.authenticity.key_fingerprint_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderSignature {
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
}

/// The private-key boundary. Implementations return signatures and public key
/// bytes only; they never expose a private key to the application or project.
pub trait SigningKeyProvider {
    fn sign_hash(
        &self,
        key_reference: &str,
        canonical_payload_hash: &[u8; 32],
    ) -> Result<ProviderSignature, ProviderError>;
}

pub trait LocalSigningKeyStore: SigningKeyProvider {
    fn create_key(&self) -> Result<PublicKeyRecord, ProviderError>;
    fn delete_key(&self, key_reference: &str) -> Result<(), ProviderError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeKeychainProvider;

#[cfg(target_os = "macos")]
impl NativeKeychainProvider {
    fn query(key_reference: &str) -> security_framework::passwords::PasswordOptions {
        let mut options = security_framework::passwords::PasswordOptions::new_generic_password(
            KEYCHAIN_SERVICE,
            key_reference,
        );
        options.set_access_synchronized(Some(false));
        options.use_protected_keychain();
        options
    }
}

#[cfg(target_os = "macos")]
impl SigningKeyProvider for NativeKeychainProvider {
    fn sign_hash(
        &self,
        key_reference: &str,
        canonical_payload_hash: &[u8; 32],
    ) -> Result<ProviderSignature, ProviderError> {
        if key_reference.trim().is_empty() {
            return Err(ProviderError::MissingKey);
        }
        let mut secret = security_framework::passwords::generic_password(Self::query(
            key_reference,
        ))
        .map_err(|error| {
            ProviderError::Unavailable(format!(
                "macOS Keychain could not release the signing key (status {})",
                error.code()
            ))
        })?;
        if secret.len() != 32 {
            secret.zeroize();
            return Err(ProviderError::MalformedKey);
        }
        let mut seed = [0_u8; 32];
        seed.copy_from_slice(&secret);
        secret.zeroize();
        let signing_key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        let signature = signing_key.sign(canonical_payload_hash);
        Ok(ProviderSignature {
            public_key: signing_key.verifying_key().to_bytes(),
            signature: signature.to_bytes(),
        })
    }
}

#[cfg(target_os = "macos")]
impl LocalSigningKeyStore for NativeKeychainProvider {
    fn create_key(&self) -> Result<PublicKeyRecord, ProviderError> {
        use security_framework::access_control::{ProtectionMode, SecAccessControl};
        use security_framework::passwords::AccessControlOptions;

        let mut seed = [0_u8; 32];
        getrandom::fill(&mut seed)
            .map_err(|_| ProviderError::Unavailable("secure random generation failed".into()))?;
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let fingerprint = sha256_hex(&verifying_key.to_bytes());
        let key_id = format!("reyn-ed25519-{}", &fingerprint[..16]);
        let mut options = Self::query(&key_id);
        options.set_label("Reyn Studio evidence signing key");
        options.set_description(
            "Ed25519 private key seed for local evidence signing; this item must not synchronize",
        );
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            AccessControlOptions::USER_PRESENCE.bits(),
        )
        .map_err(|error| {
            ProviderError::Unavailable(format!(
                "macOS Keychain access policy failed (status {})",
                error.code()
            ))
        })?;
        options.set_access_control(access);
        let result = security_framework::passwords::set_generic_password_options(&seed, options)
            .map_err(|error| {
                ProviderError::Unavailable(format!(
                    "macOS Keychain could not store the signing key (status {})",
                    error.code()
                ))
            });
        seed.zeroize();
        result?;
        Ok(PublicKeyRecord::from_verifying_key(key_id, &verifying_key))
    }

    fn delete_key(&self, key_reference: &str) -> Result<(), ProviderError> {
        security_framework::passwords::delete_generic_password_options(Self::query(key_reference))
            .map_err(|error| {
                ProviderError::Unavailable(format!(
                    "macOS Keychain could not delete the signing key (status {})",
                    error.code()
                ))
            })
    }
}

#[cfg(not(target_os = "macos"))]
impl SigningKeyProvider for NativeKeychainProvider {
    fn sign_hash(
        &self,
        _key_reference: &str,
        _canonical_payload_hash: &[u8; 32],
    ) -> Result<ProviderSignature, ProviderError> {
        Err(ProviderError::UnsupportedPlatform)
    }
}

#[cfg(not(target_os = "macos"))]
impl LocalSigningKeyStore for NativeKeychainProvider {
    fn create_key(&self) -> Result<PublicKeyRecord, ProviderError> {
        Err(ProviderError::UnsupportedPlatform)
    }

    fn delete_key(&self, _key_reference: &str) -> Result<(), ProviderError> {
        Err(ProviderError::UnsupportedPlatform)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    MissingKey,
    MalformedKey,
    #[cfg(not(target_os = "macos"))]
    UnsupportedPlatform,
    Unavailable(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(formatter, "the configured key reference is missing"),
            Self::MalformedKey => write!(formatter, "the configured key material is malformed"),
            #[cfg(not(target_os = "macos"))]
            Self::UnsupportedPlatform => {
                write!(
                    formatter,
                    "native signing-key storage is unavailable on this platform"
                )
            }
            Self::Unavailable(detail) => write!(formatter, "{detail}"),
        }
    }
}

impl std::error::Error for ProviderError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SigningError {
    Provider(ProviderError),
    RevokedKey,
    UnsupportedAlgorithm(String),
    MalformedKey(String),
    KeyMismatch(String),
    InvalidLineage(String),
    SelfVerificationFailed,
    Json(String),
}

impl fmt::Display for SigningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => write!(formatter, "signing provider: {error}"),
            Self::RevokedKey => write!(formatter, "the configured signing key is revoked"),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(formatter, "unsupported signature algorithm {algorithm}")
            }
            Self::MalformedKey(detail) => write!(formatter, "malformed signing key: {detail}"),
            Self::KeyMismatch(detail) => write!(formatter, "signing key mismatch: {detail}"),
            Self::InvalidLineage(detail) => write!(formatter, "invalid signing lineage: {detail}"),
            Self::SelfVerificationFailed => {
                write!(
                    formatter,
                    "provider signature failed Ed25519 self-verification"
                )
            }
            Self::Json(detail) => write!(formatter, "signature sidecar JSON: {detail}"),
        }
    }
}

impl std::error::Error for SigningError {}

impl From<ProviderError> for SigningError {
    fn from(error: ProviderError) -> Self {
        Self::Provider(error)
    }
}

pub fn sign_canonical_payload(
    provider: &dyn SigningKeyProvider,
    configured_key: &PublicKeyRecord,
    key_is_revoked: bool,
    lineage: &SigningLineage,
) -> Result<SignedEvidenceArtifact, SigningError> {
    if key_is_revoked {
        return Err(SigningError::RevokedKey);
    }
    let expected_public_key = configured_key.validate()?;
    let payload_hash = lineage.validate()?;
    let provider_signature = provider.sign_hash(&configured_key.key_id, &payload_hash)?;
    if provider_signature.public_key != expected_public_key.to_bytes() {
        return Err(SigningError::KeyMismatch(
            "provider returned a different public key than the configured fingerprint".into(),
        ));
    }
    let signature = Signature::from_bytes(&provider_signature.signature);
    expected_public_key
        .verify_strict(&payload_hash, &signature)
        .map_err(|_| SigningError::SelfVerificationFailed)?;

    let artifact = SignedEvidenceArtifact {
        signature_schema: SIGNATURE_SCHEMA.into(),
        created_utc_unix: lineage.created_utc_unix,
        source: SignedSource {
            run_id: lineage.run_id.clone(),
            report_schema: lineage.report_schema.clone(),
            canonical_report_sha256: lineage.canonical_report_sha256.clone(),
            canonical_payload_sha256: lineage.canonical_payload_sha256.clone(),
        },
        authenticity: AuthenticityRecord {
            status: "SIGNED".into(),
            algorithm: SIGNATURE_ALGORITHM.into(),
            key_id: configured_key.key_id.clone(),
            key_fingerprint_sha256: configured_key.key_fingerprint_sha256.clone(),
            public_key_base64: configured_key.public_key_base64.clone(),
            signature_encoding: SIGNATURE_ENCODING.into(),
            signature_bytes: BASE64.encode(provider_signature.signature),
            signed_canonical_payload_sha256: lineage.canonical_payload_sha256.clone(),
            signed_message_format: SIGNED_MESSAGE_FORMAT.into(),
            verification_status: "VALID_AT_CREATION".into(),
        },
        verification: VerificationInstructions {
            offline_command:
                "reyn-studio verify-signature --report <report.json> --signature <report.sig.json> --trusted-fingerprint <sha256>"
                    .into(),
            trust_instruction:
                "Compare key_fingerprint_sha256 with the organization fingerprint received through an independent trusted channel."
                    .into(),
            revocation_instruction:
                "Pass every current revoked fingerprint with --revoked-fingerprint; a revoked key is never accepted even when its Ed25519 bytes verify."
                    .into(),
        },
    };
    let policy = VerificationPolicy::new(
        [configured_key.key_fingerprint_sha256.clone()],
        std::iter::empty::<String>(),
    );
    let outcome = verify_signed_hash(
        &lineage.canonical_payload_sha256,
        &lineage.canonical_report_sha256,
        &artifact,
        &policy,
    );
    if outcome.status != VerificationStatus::VerifiedTrustedKey {
        return Err(SigningError::SelfVerificationFailed);
    }
    Ok(artifact)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerificationPolicy {
    trusted_fingerprints: BTreeSet<String>,
    revoked_fingerprints: BTreeSet<String>,
}

impl VerificationPolicy {
    pub fn new(
        trusted: impl IntoIterator<Item = String>,
        revoked: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            trusted_fingerprints: trusted
                .into_iter()
                .map(|fingerprint| fingerprint.to_ascii_lowercase())
                .collect(),
            revoked_fingerprints: revoked
                .into_iter()
                .map(|fingerprint| fingerprint.to_ascii_lowercase())
                .collect(),
        }
    }

    pub fn portable_untrusted() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    VerifiedTrustedKey,
    ValidUntrustedKey,
    RevokedKey,
    InvalidSignature,
    HashMismatch,
    MissingPublicKey,
    KeyMismatch,
    Malformed,
}

impl VerificationStatus {
    pub fn is_cryptographically_valid(self) -> bool {
        matches!(self, Self::VerifiedTrustedKey | Self::ValidUntrustedKey)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationOutcome {
    pub status: VerificationStatus,
    pub key_id: Option<String>,
    pub key_fingerprint_sha256: Option<String>,
    pub detail: String,
}

impl VerificationOutcome {
    fn new(
        status: VerificationStatus,
        artifact: &SignedEvidenceArtifact,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status,
            key_id: Some(artifact.authenticity.key_id.clone()),
            key_fingerprint_sha256: Some(artifact.authenticity.key_fingerprint_sha256.clone()),
            detail: detail.into(),
        }
    }
}

pub fn verify_signed_hash(
    expected_canonical_payload_sha256: &str,
    expected_canonical_report_sha256: &str,
    artifact: &SignedEvidenceArtifact,
    policy: &VerificationPolicy,
) -> VerificationOutcome {
    if artifact.signature_schema != SIGNATURE_SCHEMA
        || artifact.authenticity.status != "SIGNED"
        || artifact.authenticity.algorithm != SIGNATURE_ALGORITHM
        || artifact.authenticity.signature_encoding != SIGNATURE_ENCODING
        || artifact.authenticity.signed_message_format != SIGNED_MESSAGE_FORMAT
        || artifact.authenticity.verification_status != "VALID_AT_CREATION"
        || artifact.source.canonical_payload_sha256
            != artifact.authenticity.signed_canonical_payload_sha256
    {
        return VerificationOutcome::new(
            VerificationStatus::Malformed,
            artifact,
            "signature sidecar contains unsupported or inconsistent authenticity fields",
        );
    }
    if !is_sha256(expected_canonical_payload_sha256)
        || !is_sha256(expected_canonical_report_sha256)
        || artifact.source.canonical_payload_sha256 != expected_canonical_payload_sha256
        || artifact.source.canonical_report_sha256 != expected_canonical_report_sha256
    {
        return VerificationOutcome::new(
            VerificationStatus::HashMismatch,
            artifact,
            "signature lineage does not match the supplied canonical report",
        );
    }
    if artifact.authenticity.public_key_base64.trim().is_empty() {
        return VerificationOutcome::new(
            VerificationStatus::MissingPublicKey,
            artifact,
            "signature sidecar does not contain its portable public key",
        );
    }
    let public_record = artifact.public_key_record();
    let verifying_key = match public_record.validate() {
        Ok(key) => key,
        Err(SigningError::KeyMismatch(_)) => {
            return VerificationOutcome::new(
                VerificationStatus::KeyMismatch,
                artifact,
                "public key bytes do not match key_fingerprint_sha256",
            )
        }
        Err(_) => {
            return VerificationOutcome::new(
                VerificationStatus::Malformed,
                artifact,
                "signature sidecar contains a malformed Ed25519 public key",
            )
        }
    };
    let fingerprint = artifact
        .authenticity
        .key_fingerprint_sha256
        .to_ascii_lowercase();
    if policy.revoked_fingerprints.contains(&fingerprint) {
        return VerificationOutcome::new(
            VerificationStatus::RevokedKey,
            artifact,
            "the signing-key fingerprint is present in the supplied revocation set",
        );
    }
    let signature_bytes = match BASE64.decode(&artifact.authenticity.signature_bytes) {
        Ok(bytes) => match <Vec<u8> as TryInto<[u8; 64]>>::try_into(bytes) {
            Ok(bytes) => bytes,
            Err(_) => {
                return VerificationOutcome::new(
                    VerificationStatus::Malformed,
                    artifact,
                    "Ed25519 signature must contain exactly 64 bytes",
                )
            }
        },
        Err(_) => {
            return VerificationOutcome::new(
                VerificationStatus::Malformed,
                artifact,
                "signature_bytes is not valid base64",
            )
        }
    };
    let payload_hash = match decode_sha256(expected_canonical_payload_sha256) {
        Ok(hash) => hash,
        Err(_) => {
            return VerificationOutcome::new(
                VerificationStatus::Malformed,
                artifact,
                "canonical payload SHA-256 could not be decoded",
            )
        }
    };
    let signature = Signature::from_bytes(&signature_bytes);
    if verifying_key
        .verify_strict(&payload_hash, &signature)
        .is_err()
    {
        return VerificationOutcome::new(
            VerificationStatus::InvalidSignature,
            artifact,
            "Ed25519 verification failed",
        );
    }
    if policy.trusted_fingerprints.contains(&fingerprint) {
        VerificationOutcome::new(
            VerificationStatus::VerifiedTrustedKey,
            artifact,
            "Ed25519 signature is valid and the fingerprint is in the supplied trust set",
        )
    } else {
        VerificationOutcome::new(
            VerificationStatus::ValidUntrustedKey,
            artifact,
            "Ed25519 signature is valid; organization identity remains untrusted until the fingerprint is compared through an independent channel",
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyCliRequest {
    pub report: PathBuf,
    pub signature: PathBuf,
    pub trusted_fingerprints: Vec<String>,
    pub revoked_fingerprints: Vec<String>,
}

pub fn parse_verify_cli(
    args: impl IntoIterator<Item = String>,
) -> Option<Result<VerifyCliRequest, String>> {
    let mut args = args.into_iter();
    let _binary = args.next();
    if args.next().as_deref() != Some("verify-signature") {
        return None;
    }
    let mut report = None;
    let mut signature = None;
    let mut trusted = Vec::new();
    let mut revoked = Vec::new();
    while let Some(flag) = args.next() {
        let Some(value) = args.next() else {
            return Some(Err(format!("missing value for {flag}")));
        };
        match flag.as_str() {
            "--report" if report.is_none() => report = Some(PathBuf::from(value)),
            "--signature" if signature.is_none() => signature = Some(PathBuf::from(value)),
            "--trusted-fingerprint" if is_sha256(&value) => trusted.push(value),
            "--revoked-fingerprint" if is_sha256(&value) => revoked.push(value),
            "--trusted-fingerprint" | "--revoked-fingerprint" => {
                return Some(Err(format!(
                    "{flag} must be a 64-character lowercase SHA-256 digest"
                )))
            }
            _ => return Some(Err(format!("unknown or duplicate option {flag}"))),
        }
    }
    Some(
        report
            .zip(signature)
            .map(|(report, signature)| VerifyCliRequest {
                report,
                signature,
                trusted_fingerprints: trusted,
                revoked_fingerprints: revoked,
            })
            .ok_or_else(|| {
                "verify-signature requires --report <report.json> and --signature <report.sig.json>"
                    .into()
            }),
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], SigningError> {
    if !is_sha256(value) {
        return Err(SigningError::InvalidLineage(
            "canonical payload hash is not lowercase SHA-256".into(),
        ));
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(decoded)
}

fn hex_nibble(byte: u8) -> Result<u8, SigningError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(SigningError::InvalidLineage(
            "canonical payload hash contains invalid hexadecimal".into(),
        )),
    }
}

#[cfg(test)]
pub struct DeterministicTestProvider {
    key_id: String,
    signing_key: SigningKey,
    unavailable: bool,
}

#[cfg(test)]
impl DeterministicTestProvider {
    /// Deterministic and explicitly non-secret. The signing seed is derived at
    /// runtime from a public label; no private key is embedded in a fixture.
    pub fn new(label: &str) -> Self {
        let domain = format!("reyn.test-only.ed25519.v1\0{label}");
        let seed: [u8; 32] = Sha256::digest(domain.as_bytes()).into();
        Self {
            key_id: format!("test-{label}"),
            signing_key: SigningKey::from_bytes(&seed),
            unavailable: false,
        }
    }

    pub fn unavailable(label: &str) -> Self {
        let mut provider = Self::new(label);
        provider.unavailable = true;
        provider
    }

    pub fn public_key_record(&self) -> PublicKeyRecord {
        PublicKeyRecord::from_verifying_key(self.key_id.clone(), &self.signing_key.verifying_key())
    }
}

#[cfg(test)]
impl SigningKeyProvider for DeterministicTestProvider {
    fn sign_hash(
        &self,
        key_reference: &str,
        canonical_payload_hash: &[u8; 32],
    ) -> Result<ProviderSignature, ProviderError> {
        if self.unavailable || key_reference != self.key_id {
            return Err(ProviderError::MissingKey);
        }
        let signature = self.signing_key.sign(canonical_payload_hash);
        Ok(ProviderSignature {
            public_key: self.signing_key.verifying_key().to_bytes(),
            signature: signature.to_bytes(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lineage() -> SigningLineage {
        SigningLineage {
            run_id: "61f596e7-8414-488e-b764-0a1dfe671d1a".into(),
            report_schema: "reyn.benchmark-report-card.v1".into(),
            canonical_report_sha256:
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            canonical_payload_sha256:
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            created_utc_unix: 42,
        }
    }

    #[test]
    fn deterministic_ed25519_vector_is_stable_and_trusted() {
        let provider = DeterministicTestProvider::new("vector-a");
        let key = provider.public_key_record();
        let artifact = sign_canonical_payload(&provider, &key, false, &lineage()).unwrap();
        assert_eq!(artifact.authenticity.algorithm, "Ed25519");
        assert_eq!(
            artifact.authenticity.signature_bytes,
            "++IRI62UXAVuZ4C5x2nJ3WLPED7OjlNLVEUnw2/jUog993Yvnt+zaI0dwFoIclBM3J/D7mvwylK0KtLVeqNgBQ=="
        );
        let policy = VerificationPolicy::new(
            [key.key_fingerprint_sha256.clone()],
            std::iter::empty::<String>(),
        );
        assert_eq!(
            verify_signed_hash(
                &lineage().canonical_payload_sha256,
                &lineage().canonical_report_sha256,
                &artifact,
                &policy,
            )
            .status,
            VerificationStatus::VerifiedTrustedKey
        );
    }

    #[test]
    fn mutation_wrong_missing_and_revoked_keys_never_verify_as_signed() {
        let provider = DeterministicTestProvider::new("primary");
        let key = provider.public_key_record();
        let mut wrong_provider = DeterministicTestProvider::new("wrong");
        wrong_provider.key_id = key.key_id.clone();
        let artifact = sign_canonical_payload(&provider, &key, false, &lineage()).unwrap();

        let mutated_hash = "1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            verify_signed_hash(
                mutated_hash,
                &lineage().canonical_report_sha256,
                &artifact,
                &VerificationPolicy::portable_untrusted(),
            )
            .status,
            VerificationStatus::HashMismatch
        );
        assert!(matches!(
            sign_canonical_payload(&wrong_provider, &key, false, &lineage()),
            Err(SigningError::KeyMismatch(_))
        ));
        assert!(matches!(
            sign_canonical_payload(
                &DeterministicTestProvider::unavailable("primary"),
                &key,
                false,
                &lineage()
            ),
            Err(SigningError::Provider(ProviderError::MissingKey))
        ));
        assert!(matches!(
            sign_canonical_payload(&provider, &key, true, &lineage()),
            Err(SigningError::RevokedKey)
        ));
        let revoked = VerificationPolicy::new(
            std::iter::empty::<String>(),
            [key.key_fingerprint_sha256.clone()],
        );
        assert_eq!(
            verify_signed_hash(
                &lineage().canonical_payload_sha256,
                &lineage().canonical_report_sha256,
                &artifact,
                &revoked,
            )
            .status,
            VerificationStatus::RevokedKey
        );
    }

    #[test]
    fn malformed_or_missing_public_key_is_explicit() {
        let provider = DeterministicTestProvider::new("malformed");
        let key = provider.public_key_record();
        let mut artifact = sign_canonical_payload(&provider, &key, false, &lineage()).unwrap();
        artifact.authenticity.public_key_base64.clear();
        assert_eq!(
            verify_signed_hash(
                &lineage().canonical_payload_sha256,
                &lineage().canonical_report_sha256,
                &artifact,
                &VerificationPolicy::portable_untrusted(),
            )
            .status,
            VerificationStatus::MissingPublicKey
        );
        artifact.authenticity.public_key_base64 = "not-base64".into();
        assert_eq!(
            verify_signed_hash(
                &lineage().canonical_payload_sha256,
                &lineage().canonical_report_sha256,
                &artifact,
                &VerificationPolicy::portable_untrusted(),
            )
            .status,
            VerificationStatus::Malformed
        );
    }

    #[test]
    fn cli_requires_report_signature_and_valid_fingerprints() {
        let request = parse_verify_cli([
            "reyn-studio".into(),
            "verify-signature".into(),
            "--report".into(),
            "report.json".into(),
            "--signature".into(),
            "report.sig.json".into(),
            "--trusted-fingerprint".into(),
            "a".repeat(64),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(request.report, PathBuf::from("report.json"));
        assert_eq!(request.trusted_fingerprints, vec!["a".repeat(64)]);
        assert!(parse_verify_cli([
            "reyn-studio".into(),
            "verify-signature".into(),
            "--trusted-fingerprint".into(),
            "bad".into(),
        ])
        .unwrap()
        .is_err());
    }
}
