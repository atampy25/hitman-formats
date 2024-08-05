use std::io::{Cursor, Read};

use hitman_commons::metadata::{RuntimeID, ResourceReference};
use thiserror::Error;
use tryvial::try_fn;

type Result<T, E = MaterialError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum MaterialError {
	#[error("seek error: {0}")]
	Seek(#[from] std::io::Error),

	#[error("invalid number: {0}")]
	InvalidNumber(#[from] std::num::TryFromIntError),

	#[error("invalid utf-8: {0}")]
	InvalidString(#[from] std::str::Utf8Error),

	#[error("mismatched MATT/MATB entry count")]
	EntryCountMismatch,

	#[error("no such dependency index: {0}")]
	InvalidDependency(usize),

	#[error("unrecognised entry type: {0}")]
	UnrecognisedEntryType(u8)
}

#[derive(Clone, Debug)]
pub struct MaterialOverride {
	pub name: String,
	pub data: MaterialOverrideData
}

#[derive(Clone, Debug)]
pub enum MaterialOverrideData {
	Texture(Option<RuntimeID>),
	ColorRGB(f32, f32, f32),
	ColorRGBA(f32, f32, f32, f32),
	Float(f32),
	Vector2(f32, f32),
	Vector3(f32, f32, f32),
	Vector4(f32, f32, f32, f32)
}

/// Get the overrides of a material entity (MATT/MATB).
#[try_fn]
pub fn get_material_overrides(
	matt_data: &[u8],
	matt_references: &[ResourceReference],
	matb_data: &[u8]
) -> Result<Vec<MaterialOverride>> {
	let mut properties = vec![];

	let mut matt = Cursor::new(matt_data);
	let mut matb = Cursor::new(matb_data);

	let mut prop_names = vec![];

	while matb.position() < (matb.get_ref().len() - 1) as u64 {
		// All MATB entries are strings apparently so this type field is useless
		let _ = {
			let mut x = [0u8; 1];
			matb.read_exact(&mut x)?;
			x[0]
		};

		let matb_string_length = u32::from_le_bytes({
			let mut x = [0u8; 4];
			matb.read_exact(&mut x)?;
			x
		});

		// I'm assuming that no one is using a 16-bit computer
		let mut string_data = vec![0; matb_string_length as usize];
		matb.read_exact(&mut string_data)?;

		prop_names.push(std::str::from_utf8(&string_data[0..string_data.len() - 1])?.to_owned());
	}

	let mut cur_entry = 0;

	while matt.position() < (matt.get_ref().len() - 1) as u64 {
		let entry_type = {
			let mut x = [0u8; 1];
			matt.read_exact(&mut x)?;
			x[0]
		};

		properties.push(MaterialOverride {
			name: prop_names
				.get(cur_entry)
				.ok_or(MaterialError::EntryCountMismatch)?
				.into(),
			data: match entry_type {
				// A texture.
				1 => {
					let texture_dependency_index = u32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					if texture_dependency_index != u32::MAX {
						MaterialOverrideData::Texture(Some(
							matt_references
								.get(usize::try_from(texture_dependency_index)?)
								.ok_or_else(|| {
									MaterialError::InvalidDependency(usize::try_from(texture_dependency_index).unwrap())
								})?
								.resource
						))
					} else {
						MaterialOverrideData::Texture(None)
					}
				}

				// An RGB colour.
				2 => {
					let x = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let y = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let z = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::ColorRGB(x, y, z)
				}

				// An RGBA colour.
				3 => {
					let x = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let y = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let z = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let w = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::ColorRGBA(x, y, z, w)
				}

				// A float.
				4 => {
					let val = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::Float(val)
				}

				// A Vector2.
				5 => {
					let x = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let y = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::Vector2(x, y)
				}

				// A Vector3.
				6 => {
					let x = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let y = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let z = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::Vector3(x, y, z)
				}

				// A Vector4.
				7 => {
					let x = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let y = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let z = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					let w = f32::from_le_bytes({
						let mut x = [0u8; 4];
						matt.read_exact(&mut x)?;
						x
					});

					MaterialOverrideData::Vector4(x, y, z, w)
				}

				_ => return Err(MaterialError::UnrecognisedEntryType(entry_type))
			}
		});

		cur_entry += 1;
	}

	properties
}
