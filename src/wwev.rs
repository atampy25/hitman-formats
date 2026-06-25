use std::io::{Cursor, Read};

use glacier_commons::{
	game::GlacierGame,
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

	/// The soundbank (WBNK) referenced by this event.
	pub soundbank: RuntimeID,

	/// The WavFX (WWFX) referenced by this event.
	pub fx: Option<RuntimeID>,

	/// The metadata (WEMD) referenced by this event.
	pub metadata: Option<RuntimeID>,

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
			fx: Default::default(),
			metadata: Default::default(),
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

	/// Almost always the same as `wem_id`, but sometimes 0. Only present in FL.
	pub wem_id_2: Option<u32>,

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

	/// Almost always the same as `wem_id`, but sometimes 0. Only present in FL.
	pub wem_id_2: Option<u32>,

	/// The WWEM which contains the audio for this object.
	pub source: RuntimeID,

	/// Some amount of audio data included in the WWEV to aid loading.
	pub prefetched_data: Option<Vec<u8>>
}

impl WwiseEvent {
	/// Parse a WWEV.
	#[try_fn]
	#[cfg_attr(feature = "rune", rune::function(keep, path = Self::parse))]
	#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
	pub fn parse(version: GlacierGame, wwev_data: &[u8], wwev_metadata: &ResourceMetadata) -> Result<Self> {
		let mut wwev = Cursor::new(wwev_data);

		let wwev_name_length = u32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut wwev_name_data = vec![0; wwev_name_length as usize];
		wwev.read_exact(&mut wwev_name_data)?;

		let wwev_name = std::str::from_utf8(&wwev_name_data[0..wwev_name_data.len() - 1])?.to_owned();

		let metadata_reference = if version == GlacierGame::FL {
			let mut x = [0u8; 1];
			wwev.read_exact(&mut x)?;
			Some(x[0])
		} else {
			None
		};

		// Max attenuation
		let max_attenuation_radius = f32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let soundbank_reference = if version == GlacierGame::FL {
			Some(u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			}))
		} else {
			None
		};

		let fx_reference = if version == GlacierGame::H1 {
			Some(u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			}))
		} else {
			None
		};

		let non_streamed_count = i32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let mut non_streamed = vec![];

		for _ in 0..non_streamed_count {
			let wem_id = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let wem_id_2 = if version == GlacierGame::FL {
				Some(u32::from_le_bytes({
					let mut x = [0u8; 4];
					wwev.read_exact(&mut x)?;
					x
				}))
				.filter(|&x| x != wem_id)
			} else {
				None
			};

			let wem_size = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let mut wem_data = vec![0; wem_size as usize];
			wwev.read_exact(&mut wem_data)?;

			non_streamed.push(WwiseNonStreamedAudioObject {
				wem_id,
				wem_id_2,
				data: wem_data
			});
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

			let wem_id_2 = if version == GlacierGame::FL {
				Some(u32::from_le_bytes({
					let mut x = [0u8; 4];
					wwev.read_exact(&mut x)?;
					x
				}))
				.filter(|&x| x != wem_id)
			} else {
				None
			};

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
					wem_id_2,
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
					wem_id_2,
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
			soundbank: if let Some(idx) = soundbank_reference {
				wwev_metadata
					.references
					.get(idx as usize)
					.ok_or(WwevError::InvalidReference(idx as usize))?
					.resource
			} else {
				wwev_metadata
					.references
					.first()
					.ok_or(WwevError::InvalidReference(0))?
					.resource
			},
			fx: fx_reference
				.filter(|&idx| idx != u32::MAX)
				.map(|idx| {
					wwev_metadata
						.references
						.get(idx as usize)
						.map(|x| x.resource)
						.ok_or(WwevError::InvalidReference(idx as usize))
				})
				.transpose()?,
			metadata: metadata_reference
				.filter(|&idx| idx != 0)
				.map(|idx| {
					wwev_metadata
						.references
						.get(idx as usize)
						.map(|x| x.resource)
						.ok_or(WwevError::InvalidReference(idx as usize))
				})
				.transpose()?,
			name: wwev_name,
			max_attenuation_radius,
			non_streamed,
			streamed
		}
	}

	/// Serialise this WWEV.
	#[cfg_attr(feature = "rune", rune::function(keep, instance))]
	#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
	pub fn generate(self, version: GlacierGame) -> (Vec<u8>, ResourceMetadata) {
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
			.chain(
				if version == GlacierGame::H1
					&& let Some(fx) = self.fx
				{
					vec![ResourceReference {
						resource: fx,
						flags: ReferenceFlags {
							reference_type: ReferenceType::Weak, // no idea what type this should be, WavFX is unused in H1
							..Default::default()
						}
					}]
				} else {
					vec![]
				}
			)
			.chain(
				if version == GlacierGame::FL
					&& let Some(metadata) = self.metadata
				{
					vec![ResourceReference {
						resource: metadata,
						flags: ReferenceFlags {
							reference_type: ReferenceType::Weak,
							..Default::default()
						}
					}]
				} else {
					vec![]
				}
			)
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

		if version == GlacierGame::FL {
			wwev.push(if let Some(metadata) = self.metadata {
				wwev_meta
					.references
					.iter()
					.position(|x| x.resource == metadata)
					.unwrap() as u8
			} else {
				0
			});
		}

		// Max attenuation
		wwev.extend_from_slice(&self.max_attenuation_radius.to_le_bytes());

		if version == GlacierGame::H1 {
			// WavFX reference
			wwev.extend_from_slice(
				&if let Some(fx) = self.fx {
					wwev_meta.references.iter().position(|x| x.resource == fx).unwrap() as u32
				} else {
					u32::MAX
				}
				.to_le_bytes()
			);
		} else if version == GlacierGame::FL {
			// Soundbank reference
			wwev.extend_from_slice(
				&(wwev_meta
					.references
					.iter()
					.position(|x| x.resource == self.soundbank)
					.unwrap() as u32)
					.to_le_bytes()
			);
		}

		// Non-streamed count
		wwev.extend_from_slice(&(self.non_streamed.len() as u32).to_le_bytes());

		for audio in self.non_streamed {
			wwev.extend_from_slice(&audio.wem_id.to_le_bytes());

			if version == GlacierGame::FL {
				wwev.extend_from_slice(&audio.wem_id_2.unwrap_or(audio.wem_id).to_le_bytes());
			}

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

			if version == GlacierGame::FL {
				wwev.extend_from_slice(&audio.wem_id_2.unwrap_or(audio.wem_id).to_le_bytes());
			}

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
