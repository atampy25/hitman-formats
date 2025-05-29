use std::io::{Cursor, Read};

use hitman_commons::game::GameVersion;
use thiserror::Error;
use tryvial::try_fn;

#[cfg(feature = "rune")]
pub fn rune_module() -> Result<rune::Module, rune::ContextError> {
	let mut module = rune::Module::with_crate_item("hitman_formats", ["wwev"])?;

	module.ty::<WwevError>()?;
	module.ty::<WwiseEvent>()?;
	module.ty::<WwiseNonStreamedAudioObject>()?;
	module.ty::<WwiseStreamedAudioObject>()?;

	Ok(module)
}

type Result<T, E = WwevError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DISPLAY_FMT, DEBUG_FMT))]
pub enum WwevError {
	#[error("seek error: {0}")]
	Seek(#[from] std::io::Error),

	#[error("invalid number: {0}")]
	InvalidNumber(#[from] std::num::TryFromIntError),

	#[error("invalid utf-8: {0}")]
	InvalidString(#[from] std::str::Utf8Error),

	#[error("did not read the entire WWEV file")]
	DidNotReadEntireFile
}

/// A Wwise event; a parsed WWEV file.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, EQ))]
#[cfg_attr(feature = "rune", rune(constructor))]
pub struct WwiseEvent {
	/// The name of the event.
	pub name: String,

	pub event_max_attenuation: u32,

	/// Non-streamed audio objects (all data is stored directly in the WWEV).
	pub non_streamed: Vec<WwiseNonStreamedAudioObject>,

	/// Streamed audio objects (depending on WWEM files which contain the full data).
	pub streamed: Vec<WwiseStreamedAudioObject>
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, EQ))]
#[cfg_attr(feature = "rune", rune(constructor))]
pub struct WwiseNonStreamedAudioObject {
	pub wem_id: u32,
	pub data: Vec<u8>
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, EQ))]
#[cfg_attr(feature = "rune", rune(constructor))]
pub struct WwiseStreamedAudioObject {
	/// The index of the WWEM dependency which contains the data for this object.
	pub dependency_index: u32,

	pub wem_id: u32,

	/// Some amount of audio data included in the WWEV to aid loading.
	pub prefetched_data: Option<Vec<u8>>
}

impl WwiseEvent {
	/// Parse a WWEV.
	#[try_fn]
	#[cfg_attr(feature = "rune", rune::function(keep, path = Self::parse))]
	pub fn parse(wwev_data: &[u8]) -> Result<Self> {
		let mut wwev = Cursor::new(wwev_data);

		let wwev_name_length = u32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut wwev_name_data = vec![0; wwev_name_length as usize];
		wwev.read_exact(&mut wwev_name_data)?;

		let wwev_name = std::str::from_utf8(&wwev_name_data[0..wwev_name_data.len() - 1])?.to_owned();

		// Max attenuation (seems to be big-endian for some reason)
		let event_max_attenuation = u32::from_be_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut non_streamed_count = i32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		// In H1, there's an unknown value that seems to always be 0xFFFFFFFF before the non-streamed count
		if non_streamed_count == -1 {
			// Advance to the actual non-streamed count
			non_streamed_count = i32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});
		}

		let mut non_streamed = vec![];

		for _ in 0..non_streamed_count {
			let wem_id = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let wem_size = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let mut wem_data = vec![0; wem_size as usize];
			wwev.read_exact(&mut wem_data)?;

			non_streamed.push(WwiseNonStreamedAudioObject { wem_id, data: wem_data });
		}

		let streamed_count = u32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut streamed = vec![];

		for _ in 0..streamed_count {
			let wem_index = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let wem_id = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let prefetch_size = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			if prefetch_size != 0 {
				let mut prefetched_data = vec![0; prefetch_size as usize];
				wwev.read_exact(&mut prefetched_data)?;

				streamed.push(WwiseStreamedAudioObject {
					dependency_index: wem_index,
					wem_id,
					prefetched_data: Some(prefetched_data)
				});
			} else {
				streamed.push(WwiseStreamedAudioObject {
					dependency_index: wem_index,
					wem_id,
					prefetched_data: None
				});
			}
		}

		if wwev.position() != wwev_data.len() as u64 {
			return Err(WwevError::DidNotReadEntireFile);
		}

		WwiseEvent {
			name: wwev_name,
			event_max_attenuation,
			non_streamed,
			streamed
		}
	}

	/// Serialise this WWEV.
	#[cfg_attr(feature = "rune", rune::function(keep, instance))]
	pub fn generate(self, version: GameVersion) -> Vec<u8> {
		let mut wwev = vec![];

		// Name
		wwev.extend_from_slice(&(self.name.len() as u32 + 1).to_le_bytes());
		wwev.extend_from_slice(self.name.as_bytes());
		wwev.push(0);

		// Max attenuation
		wwev.extend_from_slice(&self.event_max_attenuation.to_be_bytes());

		if version == GameVersion::H1 {
			// Replicate the unknown value
			wwev.extend_from_slice(&u32::MAX.to_le_bytes());
		}

		// Non-streamed count
		wwev.extend_from_slice(&(self.non_streamed.len() as u32).to_le_bytes());

		for audio in self.non_streamed {
			wwev.extend_from_slice(&audio.wem_id.to_le_bytes());
			wwev.extend_from_slice(&(audio.data.len() as u32).to_le_bytes());
			wwev.extend_from_slice(&audio.data);
		}

		// Streamed count
		wwev.extend_from_slice(&(self.streamed.len() as u32).to_le_bytes());

		for audio in self.streamed {
			wwev.extend_from_slice(&audio.dependency_index.to_le_bytes());
			wwev.extend_from_slice(&audio.wem_id.to_le_bytes());

			if let Some(ref prefetched_data) = audio.prefetched_data {
				wwev.extend_from_slice(&(prefetched_data.len() as u32).to_le_bytes());
				wwev.extend_from_slice(prefetched_data);
			} else {
				wwev.extend_from_slice(&0u32.to_le_bytes());
			}
		}

		wwev
	}
}
