use core::fmt;
use std::marker::PhantomData;

use log::{debug, error};

use crate::{
    error::{PkError, PkErrorLoad},
    mem::le as mem,
    PkResult,
};

pub use crate::common::Gender;

/// A Gen 3 Game loads as little information from the game as possible, instead keeping a reference
/// to the underlying data and reading from it on demand.
///
/// # Data
///
/// The contents of a Gen 3 Game are as follows
///
/// | Offset | Size | Contents |
/// |--------|------|----------|
/// | 0x0000 | 57344 | Game Save A |
/// | 0xE000 | 57344 | Game Save B |
/// | 0x1C000 | 8192 | Hall of Fame |
/// | 0x1E000 | 4096 | Mystery Gift/e-Reader |
/// | 0x1F000 | 4096 | Recorded Battle |
#[derive(Debug)]
pub struct Game<'d> {
    data: &'d mut [u8],
    current_save_slot_info: SaveSlotInfo,
    backup_save_slot_info: SaveSlotInfo,
    version: GameVersion,
}

impl<'d> Game<'d> {
    /// 128KiB
    const SAVE_FILE_MIN_SIZE: usize = 128 * 1024;

    pub fn new(bytes: &'d mut [u8]) -> PkResult<Self> {
        Self::try_from(bytes)
    }

    pub fn validate(&self) -> Result<(), PkError> {
        Ok(())
    }

    pub fn trainer(&self) -> Data<TrainerSection> {
        Data::from_offset(self.data, self.current_save_slot_info.trainer)
    }
}

impl<'d> TryFrom<&'d mut [u8]> for Game<'d> {
    type Error = PkError;

    fn try_from(bytes: &'d mut [u8]) -> Result<Self, Self::Error> {
        debug!("Loading Gen 3 game with size: {}", bytes.len());
        let offset = emulator_intro_length(bytes);
        if offset > 0 {
            debug!("Skipping {offset} bytes from emulator intro");
        }

        if bytes.len() < Self::SAVE_FILE_MIN_SIZE {
            Err(PkError::Load(PkErrorLoad::SaveFileTooSmall {
                expected_size: Self::SAVE_FILE_MIN_SIZE,
                received_size: bytes.len(),
            }))
        } else {
            let data = &mut bytes[offset..];
            let (current_save_slot_data, backup_save_slot_data, version) = {
                let ((current_offset, current_save_slot), (backup_offset, backup_save_slot)) =
                    SaveSlot::save_slots(data);
                current_save_slot.validate()?;
                backup_save_slot.validate()?;

                let trainer_section: GameVersion =
                    current_save_slot.to_sections()?.trainer.game_code().into();

                (
                    current_save_slot.to_info(current_offset),
                    backup_save_slot.to_info(backup_offset),
                    trainer_section,
                )
            };

            debug!("Gen 3 game {} loaded", version);

            Ok(Self {
                data,
                current_save_slot_info: current_save_slot_data,
                backup_save_slot_info: backup_save_slot_data,
                version,
            })
        }
    }
}

pub trait DataView {
    const SIZE: usize;
}

pub struct DataMut<'d, D> {
    data: &'d mut [u8],
    _phantom: PhantomData<D>,
}

#[derive(Debug, Clone, Copy)]
pub struct Data<'d, D>
where
    D: fmt::Debug + Clone + Copy,
{
    data: &'d [u8],
    _phantom: PhantomData<D>,
}

impl<'d, D> Data<'d, D>
where
    D: DataView + fmt::Debug + Clone + Copy,
{
    fn new(data: &'d [u8]) -> Self {
        debug_assert!(
            data.len() >= D::SIZE,
            "DataView expects {} bytes, got {}",
            SaveSlot::SIZE,
            data.len()
        );

        Self {
            data: &data[..D::SIZE],
            _phantom: PhantomData,
        }
    }

    fn from_offset(data: &'d [u8], offset: usize) -> Self {
        let data = &data[offset..];
        debug_assert!(
            data.len() >= D::SIZE,
            "DataView expects {} bytes, got {}",
            SaveSlot::SIZE,
            data.len()
        );

        Self {
            data: &data[..D::SIZE],
            _phantom: PhantomData,
        }
    }
}

impl<'d, D: DataView> DataMut<'d, D> {
    fn new(data: &'d mut [u8]) -> Self {
        debug_assert!(
            data.len() >= D::SIZE,
            "Save slot expects {} bytes, got {}",
            SaveSlot::SIZE,
            data.len()
        );

        Self {
            data: &mut data[0..D::SIZE],
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SaveSlotInfo {
    offset: usize,
    trainer: usize,
    team_items: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SaveSlot;

impl<'d> Data<'d, SaveSlot> {
    pub fn save_index(&self) -> u32 {
        Data::<'d, Section>::new(self.data).save_index()
    }

    fn validate(&self) -> PkResult<()> {
        Ok(())
    }

    pub fn to_sections(self) -> PkResult<Sections<'d>> {
        let mut trainer = None;
        let mut team_items = None;

        for section in self.sections() {
            match section.id() {
                TrainerSection::ID => {
                    trainer = Some(Data::new(section.data));
                }
                TeamItemsSection::ID => {
                    team_items = Some(Data::new(section.data));
                }
                2..=13 => {}
                id => {
                    error!("found invalid section id: {id}");
                    return Err(PkError::Load(PkErrorLoad::InvalidSectionId(id)));
                }
            }
        }

        macro_rules! valid {
            ($section:expr, $section_name:expr) => {
                $section.ok_or_else(|| PkError::Load(PkErrorLoad::MissingSection($section_name)))
            };
        }

        Ok(Sections {
            trainer: valid!(trainer, "Trainer")?,
            team_items: valid!(team_items, "Team/Items")?,
        })
    }

    pub fn to_info(&self, current_offset: usize) -> SaveSlotInfo {
        let mut info = SaveSlotInfo {
            offset: current_offset,
            trainer: 0,
            team_items: 0,
        };

        for (i, section) in self.sections().enumerate() {
            match section.id() {
                TrainerSection::ID => {
                    info.trainer = Section::SIZE * i;
                }
                TeamItemsSection::ID => {
                    info.team_items = Section::SIZE * i;
                }
                2..=13 => {}
                id => {
                    panic!("unexpected id {id}, save slot wasn't validated");
                }
            }
        }

        info
    }

    pub fn sections(&self) -> impl Iterator<Item = Data<'d, Section>> {
        self.data.chunks_exact(Section::SIZE).map(Data::new)
    }
}

impl SaveSlot {
    const SAVE_SLOT_A_OFFSET: usize = 0;
    const SAVE_SLOT_B_OFFSET: usize = Self::SIZE;
    const SECTION_COUNT: usize = 14;

    fn save_slots(data: &[u8]) -> ((usize, Data<Self>), (usize, Data<Self>)) {
        let save_slot_a = Data::<Self>::new(data);
        let a_index = save_slot_a.save_index();
        let save_slot_b = Data::<Self>::from_offset(data, Self::SAVE_SLOT_B_OFFSET);
        let b_index = save_slot_b.save_index();

        debug!(
            "Save indices {{a = 0x{a_index:08X}, b = 0x{b_index:08X}}} - using save index {}",
            if a_index > b_index { 'a' } else { 'b' }
        );
        if save_slot_a.save_index() > save_slot_b.save_index() {
            (
                (Self::SAVE_SLOT_A_OFFSET, save_slot_a),
                (Self::SAVE_SLOT_B_OFFSET, save_slot_b),
            )
        } else {
            (
                (Self::SAVE_SLOT_B_OFFSET, save_slot_b),
                (Self::SAVE_SLOT_A_OFFSET, save_slot_a),
            )
        }
    }

    //    pub fn validate<'s>(&mut self) -> PkResult<()> {
    //        let mut sections = 0;
    //        let expected_save_index = Self::save_index(self.data);
    //        for section in self.data.chunks_exact_mut(Section::SIZE).map(Section::new) {
    //            sections += 1;
    //            if section.save_index() != expected_save_index {
    //                error!(
    //                    "missmatched save index - expected {expected_save_index}, found: {}",
    //                    section.save_index(),
    //                );
    //                return Err(PkError::Load(PkErrorLoad::MissmatchedSaveFileIndex(
    //                    expected_save_index,
    //                    section.save_index(),
    //                )));
    //            }
    //        }
    //        if sections != Self::SECTION_COUNT {
    //            error!(
    //                "wrong number of sections, expected: {}, found {sections}",
    //                Self::SECTION_COUNT
    //            );
    //            return Err(PkError::Msg("wrong number of sections in save slot"));
    //        }
    //
    //        {
    //            let sections = SaveSlot::new(&mut self.data).into_sections();
    //        }
    //
    //        Ok(())
    //    }
}

#[derive(Debug, Clone, Copy)]
pub struct Section;

impl<'d> Data<'d, Section> {
    pub fn save_index(&self) -> u32 {
        mem::read_word(self.data, Section::SAVE_INDEX_OFFSET)
    }

    pub fn id(&self) -> u16 {
        mem::read_half_word(self.data, Section::SECTION_ID_OFFSET)
    }
}

impl DataView for Section {
    const SIZE: usize = 4096;
}

impl Section {
    pub const SECTION_ID_OFFSET: usize = 0x0FF4;
    pub const SAVE_INDEX_OFFSET: usize = 0x0FFC;
}

impl DataView for SaveSlot {
    const SIZE: usize = 57_344;
}

#[derive(Debug, Clone, Copy)]
pub struct Sections<'d> {
    trainer: Data<'d, TrainerSection>,
    team_items: Data<'d, TeamItemsSection>,
}

#[derive(Debug, Clone, Copy)]
pub struct TrainerSection;

impl DataView for TrainerSection {
    const SIZE: usize = 4096;
}

impl<'d> Data<'d, TrainerSection> {
    pub fn game_code(self) -> u32 {
        mem::read_word(self.data, TrainerSection::GAME_CODE_OFFSET)
    }

    pub fn gender(self) -> PkResult<Gender> {
        match self.data[TrainerSection::GENDER_OFFSET] {
            0 => Ok(Gender::Male),
            1 => Ok(Gender::Female),
            _ => Err(PkError::InvalidData("gender")),
        }
    }
}

impl TrainerSection {
    pub const ID: u16 = 0;
    pub const GAME_CODE_OFFSET: usize = 0x00AC;
    pub const GENDER_OFFSET: usize = 0x0008;

    //    fn from_section(section: Section<'d>) -> Self {
    //        debug_assert_eq!(section.id(), Self::ID, "trying to convert invalid section into trainer");
    //        Self { data: section.data }
    //    }
}

#[derive(Debug, Clone, Copy)]
pub struct TeamItemsSection;

impl DataView for TeamItemsSection {
    const SIZE: usize = 4096;
}

impl TeamItemsSection {
    pub const ID: u16 = 1;

    //fn from_section(section: Section<'d>) -> Self {
    //    debug_assert_eq!(section.id(), Self::ID, "trying to convert invalid section into team/items");
    //    Self { data: section.data }
    //}
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum GameVersion {
    #[default]
    RubySapphire = 0,
    FireRedLeafGreen = 1,
    Emerald = 2,
}

impl fmt::Display for GameVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameVersion::RubySapphire => write!(f, "Ruby/Sapphire"),
            GameVersion::FireRedLeafGreen => write!(f, "FireRed/LeafGreen"),
            GameVersion::Emerald => write!(f, "Emerald"),
        }
    }
}

impl From<u32> for GameVersion {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::RubySapphire,
            1 => Self::FireRedLeafGreen,
            // This is the security key field
            _ => Self::Emerald,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    #[default]
    Japanese = 1,
    English = 2,
    French = 3,
    Italian = 4,
    German = 5,
    Spanish = 7,
}

/// Returns the length of the emulator intro of the save file.
const fn emulator_intro_length(_: &[u8]) -> usize {
    const GNUBOY_OFFSET: usize = 0;
    GNUBOY_OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    pub fn new_save() -> Vec<u8> {
        vec![0; Game::SAVE_FILE_MIN_SIZE]
    }

    #[test]
    fn load_game() {
        let mut bytes = new_save();
        Game::try_from(bytes.as_mut_slice()).unwrap();
    }
}
