//! Canonical script identity and save data structures.
//!
//! Provides SHA-256 based script identification for save integrity.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::EngineState;
use crate::version::{SAVE_BINARY_MAGIC, SAVE_FORMAT_VERSION};

/// Unique identifier for a compiled script, computed as SHA-256 of its binary representation.
pub type ScriptId = [u8; 32];
const AUTH_SAVE_MAGIC: [u8; 4] = *b"VNSA";
type HmacSha256 = Hmac<Sha256>;

/// Computes the canonical script_id from compiled script bytes.
pub fn compute_script_id(compiled_bytes: &[u8]) -> ScriptId {
    let mut hasher = Sha256::new();
    hasher.update(compiled_bytes);
    hasher.finalize().into()
}

/// Save data structure with script identity for integrity validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    /// SHA-256 of the compiled script this save belongs to.
    pub script_id: ScriptId,
    /// The engine state at the time of saving.
    pub state: EngineState,
}

impl SaveData {
    /// Creates a new save data bundle.
    pub fn new(script_id: ScriptId, state: EngineState) -> Self {
        Self { script_id, state }
    }

    /// Serializes save data to binary format with magic bytes and version.
    pub fn to_binary(&self) -> Result<Vec<u8>, SaveError> {
        let payload =
            postcard::to_allocvec(self).map_err(|e| SaveError::Serialization(e.to_string()))?;
        let checksum = crc32fast::hash(&payload);
        let payload_len = u32::try_from(payload.len()).map_err(|_| SaveError::TooLarge)?;

        let mut output = Vec::with_capacity(4 + 2 + 4 + 4 + payload.len());
        output.extend_from_slice(&SAVE_BINARY_MAGIC);
        output.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
        output.extend_from_slice(&checksum.to_le_bytes());
        output.extend_from_slice(&payload_len.to_le_bytes());
        output.extend_from_slice(&payload);
        Ok(output)
    }

    /// Deserializes save data from binary format, validating magic, version, and checksum.
    pub fn from_binary(input: &[u8]) -> Result<Self, SaveError> {
        if input.len() < 14 {
            return Err(SaveError::TooSmall);
        }
        if input[0..4] != SAVE_BINARY_MAGIC {
            return Err(SaveError::InvalidMagic);
        }
        let version = u16::from_le_bytes([input[4], input[5]]);
        if version != SAVE_FORMAT_VERSION {
            return Err(SaveError::IncompatibleVersion {
                found: version,
                expected: SAVE_FORMAT_VERSION,
            });
        }
        let checksum = u32::from_le_bytes([input[6], input[7], input[8], input[9]]);
        let payload_len = u32::from_le_bytes([input[10], input[11], input[12], input[13]]) as usize;
        let payload = input.get(14..).ok_or(SaveError::MissingPayload)?;
        if payload.len() != payload_len {
            return Err(SaveError::LengthMismatch);
        }
        if crc32fast::hash(payload) != checksum {
            return Err(SaveError::ChecksumMismatch);
        }
        postcard::from_bytes(payload).map_err(|e| SaveError::Serialization(e.to_string()))
    }

    /// Serializes save data to authenticated binary format.
    ///
    /// This wraps the regular save payload with a MAC to detect tampering.
    pub fn to_authenticated_binary(&self, key: &[u8]) -> Result<Vec<u8>, SaveError> {
        if key.is_empty() {
            return Err(SaveError::AuthKeyInvalid);
        }
        let payload = self.to_binary()?;
        let tag = compute_hmac_sha256(key, &payload)?;
        let payload_len = u32::try_from(payload.len()).map_err(|_| SaveError::TooLarge)?;

        let mut output = Vec::with_capacity(4 + 2 + 4 + 32 + payload.len());
        output.extend_from_slice(&AUTH_SAVE_MAGIC);
        output.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
        output.extend_from_slice(&payload_len.to_le_bytes());
        output.extend_from_slice(&tag);
        output.extend_from_slice(&payload);
        Ok(output)
    }

    /// Parses authenticated save payload and validates integrity before decoding.
    pub fn from_authenticated_binary(input: &[u8], key: &[u8]) -> Result<Self, SaveError> {
        if key.is_empty() {
            return Err(SaveError::AuthKeyInvalid);
        }
        if input.len() < 42 {
            return Err(SaveError::TooSmall);
        }
        if input[0..4] != AUTH_SAVE_MAGIC {
            return Err(SaveError::InvalidMagic);
        }

        let version = u16::from_le_bytes([input[4], input[5]]);
        if version != SAVE_FORMAT_VERSION {
            return Err(SaveError::IncompatibleVersion {
                found: version,
                expected: SAVE_FORMAT_VERSION,
            });
        }

        let payload_len = u32::from_le_bytes([input[6], input[7], input[8], input[9]]) as usize;
        let tag = input.get(10..42).ok_or(SaveError::MissingPayload)?;
        let payload = input.get(42..).ok_or(SaveError::MissingPayload)?;
        if payload.len() != payload_len {
            return Err(SaveError::LengthMismatch);
        }

        let expected = compute_hmac_sha256(key, payload)?;
        if expected.as_slice() != tag {
            return Err(SaveError::AuthenticationFailed);
        }

        SaveData::from_binary(payload)
    }

    /// Validates that this save matches the given script_id.
    pub fn validate_script_id(&self, expected: &ScriptId) -> Result<(), SaveError> {
        if &self.script_id != expected {
            return Err(SaveError::ScriptMismatch);
        }
        Ok(())
    }
}

/// Errors that can occur during save/load operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    TooSmall,
    TooLarge,
    InvalidMagic,
    IncompatibleVersion { found: u16, expected: u16 },
    ChecksumMismatch,
    LengthMismatch,
    MissingPayload,
    ScriptMismatch,
    AuthKeyInvalid,
    AuthenticationFailed,
    Serialization(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall => write!(f, "save data too small"),
            Self::TooLarge => write!(f, "save data too large"),
            Self::InvalidMagic => write!(f, "invalid save file magic bytes"),
            Self::IncompatibleVersion { found, expected } => {
                write!(
                    f,
                    "incompatible save version: found {found}, expected {expected}"
                )
            }
            Self::ChecksumMismatch => write!(f, "save file checksum mismatch"),
            Self::LengthMismatch => write!(f, "save file length mismatch"),
            Self::MissingPayload => write!(f, "save file missing payload"),
            Self::ScriptMismatch => write!(f, "save does not match current script"),
            Self::AuthKeyInvalid => write!(f, "authentication key is empty or invalid"),
            Self::AuthenticationFailed => write!(f, "save authentication failed"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for SaveError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlotMetadata {
    pub slot_id: u16,
    pub quick: bool,
    pub updated_unix_ms: u64,
    pub script_id_hex: String,
    pub position: u32,
    pub flags_words: usize,
    pub vars_count: usize,
    #[serde(default)]
    pub chapter_label: Option<String>,
    #[serde(default)]
    pub summary_line: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveSlotEntry {
    pub metadata: SaveSlotMetadata,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct SaveSlotStore {
    root: PathBuf,
}

#[derive(Debug)]
pub enum SaveStoreError {
    Io(std::io::Error),
    Save(SaveError),
    RecoveryFailed {
        primary: SaveError,
        backup: Option<SaveError>,
    },
}

impl std::fmt::Display for SaveStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveStoreError::Io(err) => write!(f, "save store io error: {err}"),
            SaveStoreError::Save(err) => write!(f, "save store serialization error: {err}"),
            SaveStoreError::RecoveryFailed { primary, backup } => match backup {
                Some(backup) => write!(
                    f,
                    "save store recovery failed (primary: {primary}, backup: {backup})"
                ),
                None => write!(
                    f,
                    "save store recovery failed (primary: {primary}, backup missing)"
                ),
            },
        }
    }
}

impl std::error::Error for SaveStoreError {}

impl From<std::io::Error> for SaveStoreError {
    fn from(value: std::io::Error) -> Self {
        SaveStoreError::Io(value)
    }
}

impl From<SaveError> for SaveStoreError {
    fn from(value: SaveError) -> Self {
        SaveStoreError::Save(value)
    }
}

impl SaveSlotStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_layout(&self) -> Result<(), SaveStoreError> {
        fs::create_dir_all(self.root.join("slots"))?;
        fs::create_dir_all(self.root.join("meta"))?;
        Ok(())
    }

    pub fn save_slot(
        &self,
        slot_id: u16,
        save: &SaveData,
    ) -> Result<SaveSlotEntry, SaveStoreError> {
        self.ensure_layout()?;
        let slot_path = self.slot_path(slot_id, false);
        let metadata_path = self.metadata_path(slot_id, false);
        self.atomic_write_binary(&slot_path, &save.to_binary()?)?;
        let metadata = self.build_metadata(slot_id, false, save);
        self.atomic_write_binary(
            &metadata_path,
            serde_json::to_vec_pretty(&metadata)
                .map_err(|err| SaveError::Serialization(err.to_string()))?
                .as_slice(),
        )?;
        Ok(SaveSlotEntry {
            metadata,
            path: slot_path,
        })
    }

    pub fn load_slot(&self, slot_id: u16) -> Result<SaveData, SaveStoreError> {
        let slot_path = self.slot_path(slot_id, false);
        let backup_path = backup_path(&slot_path);
        self.load_binary_with_recovery(&slot_path, &backup_path)
    }

    pub fn remove_slot(&self, slot_id: u16) -> Result<(), SaveStoreError> {
        let slot_path = self.slot_path(slot_id, false);
        let metadata_path = self.metadata_path(slot_id, false);
        if slot_path.exists() {
            fs::remove_file(slot_path)?;
        }
        if metadata_path.exists() {
            fs::remove_file(metadata_path)?;
        }
        Ok(())
    }

    pub fn quicksave(&self, save: &SaveData) -> Result<SaveSlotEntry, SaveStoreError> {
        self.ensure_layout()?;
        let slot_path = self.slot_path(0, true);
        let metadata_path = self.metadata_path(0, true);
        self.atomic_write_binary(&slot_path, &save.to_binary()?)?;
        let metadata = self.build_metadata(0, true, save);
        self.atomic_write_binary(
            &metadata_path,
            serde_json::to_vec_pretty(&metadata)
                .map_err(|err| SaveError::Serialization(err.to_string()))?
                .as_slice(),
        )?;
        Ok(SaveSlotEntry {
            metadata,
            path: slot_path,
        })
    }

    pub fn quickload(&self) -> Result<SaveData, SaveStoreError> {
        let slot_path = self.slot_path(0, true);
        let backup_path = backup_path(&slot_path);
        self.load_binary_with_recovery(&slot_path, &backup_path)
    }

    pub fn list_slots(&self) -> Result<Vec<SaveSlotEntry>, SaveStoreError> {
        self.ensure_layout()?;
        let mut entries = Vec::new();

        let meta_dir = self.root.join("meta");
        if !meta_dir.exists() {
            return Ok(entries);
        }

        for entry in fs::read_dir(meta_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let bytes = fs::read(&path)?;
            let metadata: SaveSlotMetadata = serde_json::from_slice(&bytes)
                .map_err(|err| SaveError::Serialization(err.to_string()))?;
            let slot_path = self.slot_path(metadata.slot_id, metadata.quick);
            if slot_path.exists() {
                entries.push(SaveSlotEntry {
                    metadata,
                    path: slot_path,
                });
            }
        }

        entries.sort_by(|a, b| b.metadata.updated_unix_ms.cmp(&a.metadata.updated_unix_ms));
        Ok(entries)
    }

    fn build_metadata(&self, slot_id: u16, quick: bool, save: &SaveData) -> SaveSlotMetadata {
        SaveSlotMetadata {
            slot_id,
            quick,
            updated_unix_ms: now_unix_ms(),
            script_id_hex: script_id_hex(&save.script_id),
            position: save.state.position,
            flags_words: save.state.flags.len(),
            vars_count: save.state.vars.len(),
            chapter_label: chapter_label_hint(save),
            summary_line: summary_line_hint(save),
        }
    }

    fn load_binary_with_recovery(
        &self,
        primary_path: &Path,
        backup_path: &Path,
    ) -> Result<SaveData, SaveStoreError> {
        let primary_bytes = fs::read(primary_path)?;
        match SaveData::from_binary(&primary_bytes) {
            Ok(save) => Ok(save),
            Err(primary_err) => match fs::read(backup_path) {
                Ok(backup_bytes) => match SaveData::from_binary(&backup_bytes) {
                    Ok(save) => Ok(save),
                    Err(backup_err) => Err(SaveStoreError::RecoveryFailed {
                        primary: primary_err,
                        backup: Some(backup_err),
                    }),
                },
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    Err(SaveStoreError::RecoveryFailed {
                        primary: primary_err,
                        backup: None,
                    })
                }
                Err(err) => Err(SaveStoreError::Io(err)),
            },
        }
    }

    fn atomic_write_binary(&self, path: &Path, bytes: &[u8]) -> Result<(), SaveStoreError> {
        let parent = path.parent().ok_or_else(|| {
            SaveStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "target path has no parent",
            ))
        })?;
        fs::create_dir_all(parent)?;
        if path.exists() {
            let backup = backup_path(path);
            fs::copy(path, backup)?;
        }
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, bytes)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    fn slot_path(&self, slot_id: u16, quick: bool) -> PathBuf {
        if quick {
            self.root.join("slots").join("quicksave.vnsav")
        } else {
            self.root
                .join("slots")
                .join(format!("slot_{slot_id:03}.vnsav"))
        }
    }

    fn metadata_path(&self, slot_id: u16, quick: bool) -> PathBuf {
        if quick {
            self.root.join("meta").join("quicksave.json")
        } else {
            self.root
                .join("meta")
                .join(format!("slot_{slot_id:03}.json"))
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn script_id_hex(script_id: &ScriptId) -> String {
    let mut output = String::with_capacity(script_id.len() * 2);
    for byte in script_id {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn chapter_label_hint(save: &SaveData) -> Option<String> {
    let background = save.state.visual.background.as_ref()?;
    let stem = Path::new(background.as_ref()).file_stem()?.to_str()?;
    let cleaned = stem.replace(['_', '-'], " ").trim().to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn summary_line_hint(save: &SaveData) -> Option<String> {
    let dialogue = save.state.history.back()?;
    let speaker = dialogue.speaker.as_ref().trim();
    let text = dialogue.text.as_ref().trim();
    if text.is_empty() {
        return None;
    }
    let mut line = if speaker.is_empty() {
        text.to_string()
    } else {
        format!("{speaker}: {text}")
    };
    const MAX_CHARS: usize = 96;
    if line.chars().count() > MAX_CHARS {
        let mut truncated = line
            .chars()
            .take(MAX_CHARS.saturating_sub(3))
            .collect::<String>();
        truncated.push_str("...");
        line = truncated;
    }
    Some(line)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut output = path.as_os_str().to_os_string();
    output.push(".bak");
    PathBuf::from(output)
}

fn compute_hmac_sha256(key: &[u8], payload: &[u8]) -> Result<[u8; 32], SaveError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| SaveError::AuthKeyInvalid)?;
    mac.update(payload);
    let bytes = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes.as_slice());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_save(position: u32) -> SaveData {
        let mut state = EngineState::new(position, 8);
        state.set_flag(2, true);
        state.set_var(1, 42);
        SaveData::new([1u8; 32], state)
    }

    #[test]
    fn slot_store_roundtrip_and_list() {
        let root = std::env::temp_dir().join(format!("vn_slot_store_{}", now_unix_ms()));
        let store = SaveSlotStore::new(root.clone());

        let save = sample_save(7);
        let entry = store.save_slot(1, &save).expect("slot save should succeed");
        assert_eq!(entry.metadata.slot_id, 1);
        assert!(!entry.metadata.quick);
        assert!(entry.path.exists());
        assert_eq!(entry.metadata.chapter_label.as_deref(), None);
        assert_eq!(entry.metadata.summary_line.as_deref(), None);

        let loaded = store.load_slot(1).expect("slot load should succeed");
        assert_eq!(loaded.state.position, 7);
        assert!(loaded.state.get_flag(2));
        assert_eq!(loaded.state.get_var(1), 42);

        let slots = store.list_slots().expect("list slots should succeed");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].metadata.slot_id, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quicksave_roundtrip() {
        let root = std::env::temp_dir().join(format!("vn_quicksave_store_{}", now_unix_ms()));
        let store = SaveSlotStore::new(root.clone());
        let save = sample_save(11);

        let entry = store.quicksave(&save).expect("quicksave should succeed");
        assert!(entry.metadata.quick);
        assert_eq!(entry.metadata.slot_id, 0);

        let loaded = store.quickload().expect("quickload should succeed");
        assert_eq!(loaded.state.position, 11);
        assert_eq!(loaded.state.get_var(1), 42);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn authenticated_save_roundtrip_and_tamper_detection() {
        let key = b"phase10-auth-key";
        let save = sample_save(21);
        let encoded = save
            .to_authenticated_binary(key)
            .expect("authenticated save should encode");
        let decoded = SaveData::from_authenticated_binary(&encoded, key)
            .expect("authenticated save should decode");
        assert_eq!(decoded.state.position, 21);

        let mut tampered = encoded.clone();
        let idx = tampered.len() - 1;
        tampered[idx] ^= 0xFF;
        let err = SaveData::from_authenticated_binary(&tampered, key)
            .expect_err("tampered save must fail auth");
        assert_eq!(err, SaveError::AuthenticationFailed);
    }

    #[test]
    fn slot_load_recovers_from_corrupted_primary() {
        let root = std::env::temp_dir().join(format!("vn_slot_recovery_{}", now_unix_ms()));
        let store = SaveSlotStore::new(root.clone());

        let save_old = sample_save(3);
        let save_new = sample_save(9);
        store
            .save_slot(7, &save_old)
            .expect("first save should succeed");
        store
            .save_slot(7, &save_new)
            .expect("second save should succeed");

        let primary_path = root.join("slots").join("slot_007.vnsav");
        fs::write(&primary_path, [0u8, 1, 2, 3]).expect("corrupt primary");

        let recovered = store
            .load_slot(7)
            .expect("loader should recover from backup");
        assert_eq!(recovered.state.position, 3);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quickload_reports_recovery_failure_when_no_backup() {
        let root = std::env::temp_dir().join(format!("vn_quick_recovery_fail_{}", now_unix_ms()));
        let store = SaveSlotStore::new(root.clone());
        store.ensure_layout().expect("layout");
        let primary_path = root.join("slots").join("quicksave.vnsav");
        fs::write(&primary_path, [9u8, 9, 9]).expect("write invalid quicksave");

        let err = store
            .quickload()
            .expect_err("invalid quicksave without backup must fail");
        assert!(matches!(
            err,
            SaveStoreError::RecoveryFailed { backup: None, .. }
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn list_slots_accepts_legacy_metadata_without_new_fields() {
        let root = std::env::temp_dir().join(format!("vn_slot_legacy_meta_{}", now_unix_ms()));
        let store = SaveSlotStore::new(root.clone());
        store.ensure_layout().expect("layout");

        let save = sample_save(13);
        let encoded = save.to_binary().expect("encode save");
        fs::write(root.join("slots").join("slot_003.vnsav"), encoded).expect("write save");
        let legacy_meta = serde_json::json!({
            "slot_id": 3,
            "quick": false,
            "updated_unix_ms": 123,
            "script_id_hex": "11",
            "position": 13,
            "flags_words": 1,
            "vars_count": 2
        });
        fs::write(
            root.join("meta").join("slot_003.json"),
            serde_json::to_vec(&legacy_meta).expect("serialize legacy metadata"),
        )
        .expect("write metadata");

        let slots = store.list_slots().expect("list slots");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].metadata.slot_id, 3);
        assert_eq!(slots[0].metadata.chapter_label, None);
        assert_eq!(slots[0].metadata.summary_line, None);

        let _ = fs::remove_dir_all(root);
    }
}
