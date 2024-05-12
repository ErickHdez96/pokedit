use core::fmt;
use std::{io::Write, path::Path};

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
    security_key: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Validate {
    None = 0,
    #[default]
    Basic = 1,
    Full = 2,
}

impl<'d> Game<'d> {
    /// 128KiB
    const SAVE_FILE_MIN_SIZE: usize = 128 * 1024;

    pub fn new(bytes: &'d mut [u8]) -> PkResult<Self> {
        Self::new_with_validation(bytes, Validate::default())
    }

    pub fn new_with_validation(bytes: &'d mut [u8], validation: Validate) -> PkResult<Self> {
        debug!("Loading Gen 3 game with size: {}", bytes.len());
        let offset = emulator_intro_length(bytes);
        if offset > 0 {
            debug!("Skipping {offset} bytes from emulator intro");
        }

        if bytes.len() < Self::SAVE_FILE_MIN_SIZE {
            return Err(PkError::Load(PkErrorLoad::SaveFileTooSmall {
                expected_size: Self::SAVE_FILE_MIN_SIZE,
                received_size: bytes.len(),
            }));
        }

        let data = &mut bytes[offset..];
        let (current_save_slot_data, backup_save_slot_data, version, security_key) = {
            let ((current_offset, current_save_slot), (backup_offset, backup_save_slot)) =
                SaveSlot::save_slots(data);
            current_save_slot.validate(validation)?;
            backup_save_slot.validate(validation)?;

            let trainer_section = current_save_slot.to_sections()?.trainer;

            (
                current_save_slot.to_info(current_offset),
                backup_save_slot.to_info(backup_offset),
                trainer_section.game_code().into(),
                trainer_section.security_key().unwrap_or(0),
            )
        };

        debug!("Gen 3 game {} loaded", version);

        Ok(Self {
            data,
            current_save_slot_info: current_save_slot_data,
            backup_save_slot_info: backup_save_slot_data,
            version,
            security_key,
        })
    }

    pub fn validate(&self) -> Result<(), PkError> {
        Ok(())
    }

    pub fn save_slot(&self) -> Data<SaveSlot> {
        Data::from_offset(self.data, self.current_save_slot_info.offset)
    }

    pub fn save_slot_mut(&mut self) -> DataMut<SaveSlot> {
        DataMut::from_offset(self.data, self.current_save_slot_info.offset)
    }

    pub fn trainer(&self) -> Data<TrainerSection> {
        Data::from_offset(self.data, self.current_save_slot_info.trainer)
    }

    pub fn team_items(&self) -> Data<TeamItemsSection> {
        Data::from_offset(self.data, self.current_save_slot_info.team_items).with_context(
            TeamItemsSection {
                version: self.version,
                security_key: self.security_key,
            },
        )
    }

    pub fn team_items_mut(&mut self) -> DataMut<TeamItemsSection> {
        DataMut::from_offset(self.data, self.current_save_slot_info.team_items).with_context(
            TeamItemsSection {
                version: self.version,
                security_key: self.security_key,
            },
        )
    }

    pub fn version(&self) -> GameVersion {
        self.version
    }

    pub fn update_checksum(&mut self) {
        for mut section in self.save_slot_mut().sections_mut() {
            section.update_checksum();
        }
    }

    pub fn save(&mut self, save_path: impl AsRef<Path>) -> PkResult<()> {
        self.update_checksum();
        let mut file = std::fs::File::create(save_path.as_ref())?;
        file.write_all(self.data)?;
        Ok(())
    }
}

impl<'d> TryFrom<&'d mut [u8]> for Game<'d> {
    type Error = PkError;

    fn try_from(bytes: &'d mut [u8]) -> Result<Self, Self::Error> {
        Self::new(bytes)
    }
}

pub trait DataView {
    const SIZE: usize;
}

#[derive(Debug)]
pub struct DataMut<'d, D>
where
    D: fmt::Debug,
{
    data: &'d mut [u8],
    view_context: D,
}

#[derive(Debug, Clone, Copy)]
pub struct Data<'d, D>
where
    D: fmt::Debug + Clone + Copy,
{
    data: &'d [u8],
    view_context: D,
}

impl<'d, D> DataMut<'d, D>
where
    D: fmt::Debug + DataView + Default,
{
    fn new(data: &'d mut [u8]) -> Self {
        debug_assert!(
            data.len() >= D::SIZE,
            "Save slot expects {} bytes, got {}",
            SaveSlot::SIZE,
            data.len()
        );

        Self {
            data: &mut data[0..D::SIZE],
            view_context: D::default(),
        }
    }

    fn from_offset(data: &'d mut [u8], offset: usize) -> Self {
        let data = &mut data[offset..];
        debug_assert!(
            data.len() >= D::SIZE,
            "DataView expects {} bytes, got {}",
            SaveSlot::SIZE,
            data.len()
        );

        Self {
            data: &mut data[..D::SIZE],
            view_context: D::default(),
        }
    }
}

impl<'d, D> DataMut<'d, D>
where
    D: fmt::Debug,
{
    fn with_context(mut self, view_context: D) -> Self {
        self.view_context = view_context;
        self
    }
}

impl<'d, D> DataMut<'d, D>
where
    D: fmt::Debug + Clone + Copy,
{
    pub fn as_data(&self) -> Data<D> {
        Data {
            data: self.data,
            view_context: self.view_context,
        }
    }
}

impl<'d, D> Data<'d, D>
where
    D: DataView + Default + fmt::Debug + Clone + Copy,
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
            view_context: D::default(),
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
            view_context: D::default(),
        }
    }
}

impl<'d, D> Data<'d, D>
where
    D: fmt::Debug + Clone + Copy,
{
    fn with_context(mut self, view_context: D) -> Self {
        self.view_context = view_context;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SaveSlotInfo {
    offset: usize,
    trainer: usize,
    team_items: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SaveSlot;

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
}

impl DataView for SaveSlot {
    const SIZE: usize = 57_344;
}

impl<'d> Data<'d, SaveSlot> {
    pub fn save_index(&self) -> u32 {
        Data::<'d, Section>::new(self.data).save_index()
    }

    fn validate(&self, validation: Validate) -> PkResult<()> {
        if validation == Validate::None {
            return Ok(());
        }

        let mut sections = 0;
        let expected_save_index = self.save_index();

        for section in self.sections() {
            sections += 1;
            if section.save_index() != expected_save_index {
                error!(
                    "missmatched save index - expected {expected_save_index}, found: {}",
                    section.save_index(),
                );
                return Err(PkError::Load(PkErrorLoad::MissmatchedSaveFileIndex(
                    expected_save_index,
                    section.save_index(),
                )));
            }

            section.validate(validation)?;
        }

        if sections != SaveSlot::SECTION_COUNT {
            error!(
                "wrong number of sections, expected: {}, found {sections}",
                SaveSlot::SECTION_COUNT
            );
            return Err(PkError::Msg("wrong number of sections in save slot"));
        }

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
                    info.trainer = current_offset + Section::SIZE * i;
                }
                TeamItemsSection::ID => {
                    info.team_items = current_offset + Section::SIZE * i;
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

impl<'d> DataMut<'d, SaveSlot> {
    pub fn sections_mut(&mut self) -> impl Iterator<Item = DataMut<Section>> {
        self.data.chunks_exact_mut(Section::SIZE).map(DataMut::new)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Section;

impl DataView for Section {
    const SIZE: usize = 4096;
}

impl Section {
    pub const SECTION_ID_OFFSET: usize = 0x0FF4;
    pub const CHECKSUM_OFFSET: usize = 0x0FF6;
    pub const SIGNATURE_OFFSET: usize = 0x0FF8;
    pub const SAVE_INDEX_OFFSET: usize = 0x0FFC;

    pub const MAGIC_SIGNATURE: u32 = 0x08012025;
}

impl<'d> Data<'d, Section> {
    pub fn checksum(&self) -> u16 {
        mem::read_half_word(self.data, Section::CHECKSUM_OFFSET)
    }

    pub fn calculate_checksum(&self) -> u16 {
        let checksumable_bytes = match self.id() {
            TrainerSection::ID => 3884,
            TeamItemsSection::ID => 3968,
            2 => 3968,
            3 => 3968,
            4 => 3848,
            5..=12 => 3968,
            13 => 2000,
            id => panic!("invalid id {id}"),
        };
        calculate_checksum(&self.data[0..checksumable_bytes])
    }

    pub fn signature(&self) -> u32 {
        mem::read_word(self.data, Section::SIGNATURE_OFFSET)
    }

    pub fn save_index(&self) -> u32 {
        mem::read_word(self.data, Section::SAVE_INDEX_OFFSET)
    }

    pub fn id(&self) -> u16 {
        mem::read_half_word(self.data, Section::SECTION_ID_OFFSET)
    }

    pub fn validate(&self, validation: Validate) -> PkResult<()> {
        if validation == Validate::None {
            return Ok(());
        }

        let current_checksum = self.checksum();
        let expected_checksum = self.calculate_checksum();

        if current_checksum != expected_checksum {
            return Err(PkError::Load(PkErrorLoad::InvalidChecksum {
                section_id: self.id(),
                expected: expected_checksum,
                found: current_checksum,
            }));
        }

        let current_signature = self.signature();
        if current_signature != Section::MAGIC_SIGNATURE {
            return Err(PkError::Load(PkErrorLoad::InvalidSignature {
                section_id: self.id(),
                expected: Section::MAGIC_SIGNATURE,
                found: current_signature,
            }));
        }

        Ok(())
    }
}

impl<'d> DataMut<'d, Section> {
    pub fn update_checksum(&mut self) {
        let checksum = self.as_data().calculate_checksum();
        mem::write_half_word(self.data, Section::CHECKSUM_OFFSET, checksum);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sections<'d> {
    trainer: Data<'d, TrainerSection>,
    team_items: Data<'d, TeamItemsSection>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TrainerSection;

impl DataView for TrainerSection {
    const SIZE: usize = 4096;
}

impl TrainerSection {
    pub const ID: u16 = 0;
    pub const PLAYER_NAME_OFFSET: usize = 0x0000;
    pub const PLAYER_NAME_LENGTH: usize = 7;
    pub const GAME_CODE_OFFSET: usize = 0x00AC;
    pub const GENDER_OFFSET: usize = 0x0008;

    pub const TRAINER_ID_OFFSET: usize = 0x000A;
    pub const PUBLIC_TRAINER_ID_OFFSET: usize = Self::TRAINER_ID_OFFSET;
    pub const PRIVATE_TRAINER_ID_OFFSET: usize = Self::TRAINER_ID_OFFSET + 2;

    pub const TIME_PLAYED_OFFSET: usize = 0x000E;
    pub const HOURS_PLAYED_OFFSET: usize = Self::TIME_PLAYED_OFFSET;
    pub const MINUTES_PLAYED_OFFSET: usize = Self::HOURS_PLAYED_OFFSET + 2;
    pub const SECONDS_PLAYED_OFFSET: usize = Self::MINUTES_PLAYED_OFFSET + 1;
    pub const FRAMES_PLAYED_OFFSET: usize = Self::SECONDS_PLAYED_OFFSET + 1;
}

impl<'d> Data<'d, TrainerSection> {
    fn to_section(self) -> Data<'d, Section> {
        Data::new(self.data)
    }

    pub fn checksum(self) -> u16 {
        self.to_section().checksum()
    }

    pub fn name_raw(self) -> [u8; 7] {
        self.data[TrainerSection::PLAYER_NAME_OFFSET
            ..(TrainerSection::PLAYER_NAME_OFFSET + TrainerSection::PLAYER_NAME_LENGTH)]
            .try_into()
            .unwrap()
    }

    pub fn game_code(self) -> u32 {
        mem::read_word(self.data, TrainerSection::GAME_CODE_OFFSET)
    }

    pub fn gender(self) -> PkResult<Gender> {
        match self.data[TrainerSection::GENDER_OFFSET] {
            0 => Ok(Gender::Male),
            1 => Ok(Gender::Female),
            g => {
                error!("invalid gender found: {g}");
                Err(PkError::InvalidData("gender"))
            }
        }
    }

    pub fn trainer_id(self) -> TrainerId {
        TrainerId {
            public: mem::read_half_word(self.data, TrainerSection::PUBLIC_TRAINER_ID_OFFSET),
            private: mem::read_half_word(self.data, TrainerSection::PRIVATE_TRAINER_ID_OFFSET),
        }
    }

    pub fn time_played(self) -> Playtime {
        Playtime {
            hours: mem::read_half_word(self.data, TrainerSection::HOURS_PLAYED_OFFSET),
            minutes: self.data[TrainerSection::MINUTES_PLAYED_OFFSET],
            seconds: self.data[TrainerSection::SECONDS_PLAYED_OFFSET],
            frames: self.data[TrainerSection::FRAMES_PLAYED_OFFSET],
        }
    }

    pub fn security_key(self) -> PkResult<u32> {
        match self.game_code() {
            // Sapphire/Ruby doesn't have a security key.
            0 => Err(PkError::NotAvailableInGameVersion("Security Key")),
            1 => Ok(mem::read_word(
                self.data,
                GameVersion::FireRedLeafGreen.security_key_offset(),
            )),
            // Emerald stores its security key as the game code.
            n => Ok(n),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TrainerId {
    pub public: u16,
    pub private: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Playtime {
    pub hours: u16,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TeamItemsSection {
    version: GameVersion,
    security_key: u32,
}

impl TeamItemsSection {
    pub const ID: u16 = 1;

    //fn from_section(section: Section<'d>) -> Self {
    //    debug_assert_eq!(section.id(), Self::ID, "trying to convert invalid section into team/items");
    //    Self { data: section.data }
    //}
}

impl DataView for TeamItemsSection {
    const SIZE: usize = 4096;
}

impl<'d> Data<'d, TeamItemsSection> {
    pub fn money(self) -> u32 {
        decrypt_word(
            self.view_context.security_key,
            mem::read_word(self.data, self.view_context.version.money_offset()),
        )
    }
}

impl<'d> DataMut<'d, TeamItemsSection> {
    pub fn set_money(&mut self, value: u32) {
        mem::write_word(
            self.data,
            self.view_context.version.money_offset(),
            encrypt_word(self.view_context.security_key, value),
        );
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GameVersion {
    #[default]
    RubySapphire = 0,
    FireRedLeafGreen = 1,
    Emerald = 2,
}

impl GameVersion {
    /// Returns the offset into the trainer section where the security key is stored.
    pub const fn security_key_offset(self) -> usize {
        match self {
            GameVersion::RubySapphire => 0,
            GameVersion::FireRedLeafGreen => 0x0AF8,
            GameVersion::Emerald => 0x00AC,
        }
    }

    /// Returns the offset into the team/items section where the player's money is stored.
    pub const fn money_offset(self) -> usize {
        match self {
            GameVersion::RubySapphire | GameVersion::Emerald => 0x0490,
            GameVersion::FireRedLeafGreen => 0x0290,
        }
    }
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

fn calculate_checksum(data: &[u8]) -> u16 {
    debug_assert_eq!(
        data.len() % 4,
        0,
        "data must be divisible by 4 for the checksum"
    );
    let mut checksum = 0u32;

    for chunk in data.chunks_exact(4) {
        checksum = checksum.wrapping_add(u32::from_le_bytes(chunk.try_into().unwrap()));
    }

    ((checksum >> 16) as u16).wrapping_add((checksum & 0xFFFF) as u16)
}

const fn decrypt_word(key: u32, value: u32) -> u32 {
    key ^ value
}

const fn encrypt_word(key: u32, value: u32) -> u32 {
    key ^ value
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
