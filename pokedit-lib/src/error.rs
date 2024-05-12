use core::fmt;

#[derive(Debug)]
pub enum PkError {
    Load(PkErrorLoad),
    InvalidData(&'static str),
    NotAvailableInGameVersion(&'static str),
    Msg(&'static str),
    Io(std::io::Error),
}

impl fmt::Display for PkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PkError::Load(l) => write!(f, "{l}"),
            PkError::InvalidData(m) => write!(f, "save file contains invalid data: {m}"),
            PkError::NotAvailableInGameVersion(m) => write!(
                f,
                "the requested datum \"{m}\" is not available in the version of the loaded game"
            ),
            PkError::Msg(m) => write!(f, "{m}"),
            PkError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl From<std::io::Error> for PkError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::error::Error for PkError {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PkErrorLoad {
    SaveFileTooSmall {
        expected_size: usize,
        received_size: usize,
    },
    InvalidChecksum {
        section_id: u16,
        expected: u16,
        found: u16,
    },
    InvalidSignature {
        section_id: u16,
        expected: u32,
        found: u32,
    },
    MissingSection(&'static str),
    InvalidSectionId(u16),
    MissmatchedSaveFileIndex(u32, u32),
}

impl fmt::Display for PkErrorLoad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PkErrorLoad::SaveFileTooSmall {
                expected_size,
                received_size,
            } => write!(f, "expected save file with a minimum size of {expected_size} bytes, received file with {received_size} bytes"),
            PkErrorLoad::InvalidChecksum { section_id, expected, found } => write!(f, "section {section_id} has an invalid checksum {found}, expected {expected}"),
            PkErrorLoad::InvalidSignature { section_id, expected, found } => write!(f, "section {section_id} has an invalid signature {found}, expected {expected}"),
            PkErrorLoad::MissingSection(section_name) => write!(f, "save file missing section: {section_name}"),
            PkErrorLoad::InvalidSectionId(id) => write!(f, "save file contains invalid section id: {id}"),
            PkErrorLoad::MissmatchedSaveFileIndex(first, second) => write!(f, "sections contain missmatching save indices: {first} - {second}"),
        }
    }
}

impl std::error::Error for PkErrorLoad {}
