use std::io::{Cursor, Read};

use hitman_commons::{
	game::GameVersion,
	metadata::{ReferenceFlags, ReferenceType, ResourceMetadata, ResourceReference, RuntimeID}
};
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

	#[error("no such reference at index {0}")]
	InvalidReference(usize),

	#[error("did not read the entire WWEV file")]
	DidNotReadEntireFile
}

/// A Wwise event; a parsed WWEV file.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, CLONE))]
#[cfg_attr(feature = "rune", rune(constructor_fn = Self::rune_construct))]
pub struct WwiseEvent {
	pub id: RuntimeID,

	/// The soundbank referenced by this event.
	pub soundbank: RuntimeID,

	/// The name of the event.
	pub name: String,

	/// The maximum distance from the audio emitter that this event is audible from (or -1).
	pub max_attenuation_radius: f32,

	/// Non-streamed audio objects (all data is stored directly in the WWEV).
	pub non_streamed: Vec<WwiseNonStreamedAudioObject>,

	/// Streamed audio objects (depending on WWEM files which contain the full data).
	pub streamed: Vec<WwiseStreamedAudioObject>
}

#[cfg(feature = "rune")]
impl WwiseEvent {
	fn rune_construct(id: RuntimeID, soundbank: RuntimeID, name: String) -> Self {
		Self {
			id,
			soundbank,
			name,
			max_attenuation_radius: -1.0,
			non_streamed: Default::default(),
			streamed: Default::default()
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, EQ, CLONE))]
#[cfg_attr(feature = "rune", rune(constructor))]
pub struct WwiseNonStreamedAudioObject {
	pub wem_id: u32,
	pub data: Vec<u8>
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "rune", serde_with::apply(_ => #[rune(get, set)]))]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::wwev))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, PARTIAL_EQ, EQ, CLONE))]
#[cfg_attr(feature = "rune", rune(constructor))]
pub struct WwiseStreamedAudioObject {
	pub wem_id: u32,

	/// The WWEM which contains the audio for this object.
	pub source: RuntimeID,

	/// Some amount of audio data included in the WWEV to aid loading.
	pub prefetched_data: Option<Vec<u8>>
}

impl WwiseEvent {
	/// Parse a WWEV.
	#[try_fn]
	#[cfg_attr(feature = "rune", rune::function(keep, path = Self::parse))]
	pub fn parse(wwev_data: &[u8], wwev_metadata: &ResourceMetadata) -> Result<Self> {
		let mut wwev = Cursor::new(wwev_data);

		let wwev_name_length = u32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut wwev_name_data = vec![0; wwev_name_length as usize];
		wwev.read_exact(&mut wwev_name_data)?;

		let wwev_name = std::str::from_utf8(&wwev_name_data[0..wwev_name_data.len() - 1])?.to_owned();

		// Max attenuation
		let max_attenuation_radius = f32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut non_streamed_count = i32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		// In H1, there's a WavFX reference before the non-streamed count which is unused (and thus always 0xFFFFFFFF)
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
			}) as usize;

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
					wem_id,
					source: wwev_metadata
						.references
						.get(wem_index)
						.ok_or(WwevError::InvalidReference(wem_index))?
						.resource,
					prefetched_data: Some(prefetched_data)
				});
			} else {
				streamed.push(WwiseStreamedAudioObject {
					wem_id,
					source: wwev_metadata
						.references
						.get(wem_index)
						.ok_or(WwevError::InvalidReference(wem_index))?
						.resource,
					prefetched_data: None
				});
			}
		}

		if wwev.position() != wwev_data.len() as u64 {
			return Err(WwevError::DidNotReadEntireFile);
		}

		WwiseEvent {
			id: wwev_metadata.id,
			soundbank: wwev_metadata
				.references
				.first()
				.ok_or(WwevError::InvalidReference(0))?
				.resource,
			name: wwev_name,
			max_attenuation_radius,
			non_streamed,
			streamed
		}
	}

	/// Serialise this WWEV.
	#[cfg_attr(feature = "rune", rune::function(keep, instance))]
	pub fn generate(self, version: GameVersion) -> (Vec<u8>, ResourceMetadata) {
		let mut wwev = vec![];

		let wwev_meta = ResourceMetadata {
			id: self.id,
			resource_type: "WWEV".try_into().unwrap(),
			compressed: ResourceMetadata::infer_compressed("WWEV".try_into().unwrap()),
			scrambled: ResourceMetadata::infer_scrambled("WWEV".try_into().unwrap()),
			references: [ResourceReference {
				resource: self.soundbank,
				flags: ReferenceFlags::default()
			}]
			.into_iter()
			.chain(self.streamed.iter().map(|x| ResourceReference {
				resource: x.source,
				flags: ReferenceFlags {
					reference_type: ReferenceType::Weak, // FIXME: Media type for legacy support?
					..Default::default()
				}
			}))
			.collect()
		};

		// Name
		wwev.extend_from_slice(&(self.name.len() as u32 + 1).to_le_bytes());
		wwev.extend_from_slice(self.name.as_bytes());
		wwev.push(0);

		// Max attenuation
		wwev.extend_from_slice(&self.max_attenuation_radius.to_le_bytes());

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
			wwev.extend_from_slice(
				&(wwev_meta
					.references
					.iter()
					.position(|x| x.resource == audio.source)
					.unwrap() as u32)
					.to_le_bytes()
			);
			wwev.extend_from_slice(&audio.wem_id.to_le_bytes());

			if let Some(ref prefetched_data) = audio.prefetched_data {
				wwev.extend_from_slice(&(prefetched_data.len() as u32).to_le_bytes());
				wwev.extend_from_slice(prefetched_data);
			} else {
				wwev.extend_from_slice(&0u32.to_le_bytes());
			}
		}

		(wwev, wwev_meta)
	}
}
