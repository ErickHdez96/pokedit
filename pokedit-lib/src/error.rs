use core::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PkError {
    Load(PkErrorLoad),
    InvalidData(&'static str),
    Msg(&'static str),
}

impl fmt::Display for PkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PkError::Load(l) => write!(f, "{l}"),
            PkError::InvalidData(m) => write!(f, "save file contains invalid data: {m}"),
            PkError::Msg(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for PkError {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PkErrorLoad {
    SaveFileTooSmall {
        expected_size: usize,
        received_size: usize,
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
            PkErrorLoad::MissingSection(section_name) => write!(f, "save file missing section: {section_name}"),
            PkErrorLoad::InvalidSectionId(id) => write!(f, "save file contains invalid section id: {id}"),
            PkErrorLoad::MissmatchedSaveFileIndex(first, second) => write!(f, "sections contain missmatching save indices: {first} - {second}"),
        }
    }
}

impl std::error::Error for PkErrorLoad {}
