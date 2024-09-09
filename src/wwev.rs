use std::io::{Cursor, Read};

use thiserror::Error;
use tryvial::try_fn;

type Result<T, E = WwevError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum WwevError {
	#[error("seek error: {0}")]
	Seek(#[from] std::io::Error),

	#[error("invalid number: {0}")]
	InvalidNumber(#[from] std::num::TryFromIntError),

	#[error("invalid utf-8: {0}")]
	InvalidString(#[from] std::str::Utf8Error)
}

/// A Wwise event; a parsed WWEV file.
pub struct WwiseEvent {
	/// The name of the event.
	pub name: String,

	pub event_max_attenuation: u32,

	/// The event's data. Can be streamed or non-streamed.
	pub data: WwiseEventData
}

pub enum WwiseEventData {
	/// One or more non-streamed audio objects (all data is stored directly in the WWEV).
	NonStreamed(Vec<WwiseNonStreamedAudioObject>),

	/// One or more streamed audio objects (depending on WWEM files which contain the full data).
	Streamed(Vec<WwiseStreamedAudioObject>)
}

pub struct WwiseNonStreamedAudioObject {
	pub wem_id: u32,
	pub data: Vec<u8>
}

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

		let event_max_attenuation = u32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		let non_streamed_count = i32::from_le_bytes({
			let mut x = [0u8; 4];
			wwev.read_exact(&mut x)?;
			x
		});

		// TODO: There is apparently another i32 here in 2016 WWEVs

		if non_streamed_count == 0 {
			// This is a streamed WWEV or it is simply empty

			let entries_count = u32::from_le_bytes({
				let mut x = [0u8; 4];
				wwev.read_exact(&mut x)?;
				x
			});

			let mut audio_objects = vec![];

			let mut cur_entry = 0;

			while cur_entry < entries_count {
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

					audio_objects.push(WwiseStreamedAudioObject {
						dependency_index: wem_index,
						wem_id,
						prefetched_data: Some(prefetched_data)
					});
				} else {
					audio_objects.push(WwiseStreamedAudioObject {
						dependency_index: wem_index,
						wem_id,
						prefetched_data: None
					});
				}

				cur_entry += 1;
			}

			WwiseEvent {
				name: wwev_name,
				event_max_attenuation,
				data: WwiseEventData::Streamed(audio_objects)
			}
		} else {
			// This WWEV has only non-streamed audio objects

			let mut audio_objects = vec![];

			let mut cur_entry = 0;

			while cur_entry < non_streamed_count {
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

				audio_objects.push(WwiseNonStreamedAudioObject { wem_id, data: wem_data });

				cur_entry += 1;
			}

			WwiseEvent {
				name: wwev_name,
				event_max_attenuation,
				data: WwiseEventData::NonStreamed(audio_objects)
			}
		}
	}

	/// Serialise this WWEV.
	pub fn generate(self) -> Vec<u8> {
		let mut wwev = vec![];

		// Name
		wwev.extend_from_slice(&(self.name.len() as u32 + 1).to_le_bytes());
		wwev.extend_from_slice(self.name.as_bytes());
		wwev.push(0);

		// Max attenuation
		wwev.extend_from_slice(&self.event_max_attenuation.to_le_bytes());

		match self.data {
			WwiseEventData::NonStreamed(ref audio_objects) => {
				// Non-streamed count
				wwev.extend_from_slice(&(audio_objects.len() as i32).to_le_bytes());

				for audio_object in audio_objects {
					wwev.extend_from_slice(&audio_object.wem_id.to_le_bytes());
					wwev.extend_from_slice(&(audio_object.data.len() as u32).to_le_bytes());
					wwev.extend_from_slice(&audio_object.data);
				}
			}

			WwiseEventData::Streamed(ref audio_objects) => {
				// Non-streamed count
				wwev.extend_from_slice(&0u32.to_le_bytes());

				// Entries count
				wwev.extend_from_slice(&(audio_objects.len() as u32).to_le_bytes());

				for audio_object in audio_objects {
					wwev.extend_from_slice(&audio_object.dependency_index.to_le_bytes());
					wwev.extend_from_slice(&audio_object.wem_id.to_le_bytes());

					if let Some(ref prefetched_data) = audio_object.prefetched_data {
						wwev.extend_from_slice(&(prefetched_data.len() as u32).to_le_bytes());
						wwev.extend_from_slice(prefetched_data);
					} else {
						wwev.extend_from_slice(&0u32.to_le_bytes());
					}
				}
			}
		}

		wwev
	}
}
