use std::{
	fmt::Display,
	io::{Cursor, Read, Seek, SeekFrom},
	num::ParseIntError,
	str::FromStr
};

use hitman_commons::metadata::{ReferenceFlags, ReferenceType, ResourceMetadata, ResourceReference, RuntimeID};
use indexmap::IndexMap;
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
	InvalidStr(#[from] std::str::Utf8Error),

	#[error("invalid utf-8: {0}")]
	InvalidString(#[from] std::string::FromUtf8Error),

	#[error("mismatched MATT/MATB entry count")]
	EntryCountMismatch,

	#[error("no such dependency index: {0}")]
	InvalidDependency(usize),

	#[error("unrecognised entry type: {0}")]
	UnrecognisedEntryType(u8),

	#[error("incorrect type pairing: {0} is not type {1}")]
	IncorrectType(String, u8),

	#[error("invalid runtime ID: {0}")]
	InvalidRuntimeID(#[from] hitman_commons::metadata::FromU64Error),

	#[error("instance not top-level")]
	InstanceNotTopLevel,

	#[error("required property not found: {0}")]
	RequiredPropertyNotFound(String),

	#[error("property at wrong level")]
	PropertyAtWrongLevel(IntermediateMaterialProperty),

	#[error("float vector of correct size for colour expected for property {0}")]
	InvalidColor(String),

	#[error("invalid material type: {0}")]
	InvalidMaterialType(String),

	#[error("invalid culling mode: {0}")]
	InvalidCullingMode(String),

	#[error("invalid blend mode: {0}")]
	InvalidBlendMode(String),

	#[error("vectors must be size 2, 3 or 4")]
	InvalidVector,

	#[error("invalid hex: {0}")]
	InvalidHex(#[from] ParseIntError)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialEntity {
	pub factory: RuntimeID,
	pub blueprint: RuntimeID,
	pub material: RuntimeID,
	pub overrides: IndexMap<String, MaterialOverride>
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialOverride {
	Texture(Option<RuntimeID>),
	Color(String),
	Float(f32),
	Vector(Vec<f32>)
}

impl MaterialEntity {
	/// Parse a material entity (MATT/MATB).
	#[try_fn]
	pub fn parse(
		matt_data: &[u8],
		matt_metadata: &ResourceMetadata,
		matb_data: &[u8],
		matb_metadata: &ResourceMetadata
	) -> Result<Self> {
		let mut properties = vec![];

		let mut matt = Cursor::new(matt_data);
		let mut matb = Cursor::new(matb_data);

		let mut prop_names = vec![];

		while matb.position() < (matb.get_ref().len() - 1) as u64 {
			// The type field is for the property type
			// but it doesn't matter here because all the MATB contains is the name
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

			properties.push((
				prop_names
					.get(cur_entry)
					.ok_or(MaterialError::EntryCountMismatch)?
					.into(),
				match entry_type {
					// A texture.
					1 => {
						let texture_dependency_index = u32::from_le_bytes({
							let mut x = [0u8; 4];
							matt.read_exact(&mut x)?;
							x
						});

						if texture_dependency_index != u32::MAX {
							MaterialOverride::Texture(Some(
								matt_metadata
									.references
									.get(usize::try_from(texture_dependency_index)?)
									.ok_or_else(|| {
										MaterialError::InvalidDependency(
											usize::try_from(texture_dependency_index).unwrap()
										)
									})?
									.resource
							))
						} else {
							MaterialOverride::Texture(None)
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

						MaterialOverride::Color(format!(
							"#{:0>2x}{:0>2x}{:0>2x}",
							(x * 255.0).round() as u8,
							(y * 255.0).round() as u8,
							(z * 255.0).round() as u8
						))
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

						MaterialOverride::Color(format!(
							"#{:0>2x}{:0>2x}{:0>2x}{:0>2x}",
							(x * 255.0).round() as u8,
							(y * 255.0).round() as u8,
							(z * 255.0).round() as u8,
							(w * 255.0).round() as u8
						))
					}

					// A float.
					4 => {
						let val = f32::from_le_bytes({
							let mut x = [0u8; 4];
							matt.read_exact(&mut x)?;
							x
						});

						MaterialOverride::Float(val)
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

						MaterialOverride::Vector(vec![x, y])
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

						MaterialOverride::Vector(vec![x, y, z])
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

						MaterialOverride::Vector(vec![x, y, z, w])
					}

					_ => return Err(MaterialError::UnrecognisedEntryType(entry_type))
				}
			));

			cur_entry += 1;
		}

		Self {
			factory: matt_metadata.id,
			blueprint: matb_metadata.id,
			material: matt_metadata.references.get(2).ok_or(MaterialError::InvalidDependency(2))?.resource,
			overrides: properties.into_iter().collect()
		}
	}

	/// Generate the game binary for this material entity.
	#[try_fn]
	pub fn generate(self) -> Result<((Vec<u8>, ResourceMetadata), (Vec<u8>, ResourceMetadata))> {
		let mut matt = vec![];
		let mut matb = vec![];

		let mut matt_references = vec![
			ResourceReference {
				resource: "00B4B11DA327CAD0".parse().unwrap(),
				flags: ReferenceFlags::default()
			},
			ResourceReference {
				resource: self.blueprint,
				flags: ReferenceFlags::default()
			},
			ResourceReference {
				resource: self.material,
				flags: ReferenceFlags::default()
			}
		];

		for (prop_name, prop_val) in self.overrides {
			let prop_name = prop_name.as_bytes();
			let prop_name_len = prop_name.len() as u32;

			match prop_val {
				MaterialOverride::Texture(Some(id)) => {
					matt_references.push(ResourceReference {
						resource: id,
						flags: ReferenceFlags::default()
					});

					matb.extend_from_slice(&[1]);

					matt.extend_from_slice(&[1]);
					matt.extend_from_slice(&(matt_references.len() as u32 - 1).to_le_bytes());
				}

				MaterialOverride::Texture(None) => {
					matt.extend_from_slice(&[1]);
					matt.extend_from_slice(&u32::MAX.to_le_bytes());
				}

				MaterialOverride::Color(value) => {
					if value.len() > 7 {
						let r =
							u8::from_str_radix(&value.chars().skip(1).take(2).collect::<String>(), 16)? as f32 / 255.0;

						let g =
							u8::from_str_radix(&value.chars().skip(3).take(2).collect::<String>(), 16)? as f32 / 255.0;

						let b =
							u8::from_str_radix(&value.chars().skip(5).take(2).collect::<String>(), 16)? as f32 / 255.0;

						let a =
							u8::from_str_radix(&value.chars().skip(7).take(2).collect::<String>(), 16)? as f32 / 255.0;

						matt.extend_from_slice(&[3]);
						matt.extend_from_slice(&r.to_le_bytes());
						matt.extend_from_slice(&g.to_le_bytes());
						matt.extend_from_slice(&b.to_le_bytes());
						matt.extend_from_slice(&a.to_le_bytes());
					} else {
						let r =
							u8::from_str_radix(&value.chars().skip(1).take(2).collect::<String>(), 16)? as f32 / 255.0;

						let g =
							u8::from_str_radix(&value.chars().skip(3).take(2).collect::<String>(), 16)? as f32 / 255.0;

						let b =
							u8::from_str_radix(&value.chars().skip(5).take(2).collect::<String>(), 16)? as f32 / 255.0;

						matt.extend_from_slice(&[2]);
						matt.extend_from_slice(&r.to_le_bytes());
						matt.extend_from_slice(&g.to_le_bytes());
						matt.extend_from_slice(&b.to_le_bytes());
					}
				}

				MaterialOverride::Float(val) => {
					matb.extend_from_slice(&[4]);

					matt.extend_from_slice(&[4]);
					matt.extend_from_slice(&val.to_le_bytes());
				}

				MaterialOverride::Vector(vec) => {
					let entry_type = match vec.len() {
						2 => 5,
						3 => 6,
						4 => 7,
						_ => return Err(MaterialError::InvalidVector)
					};

					matb.extend_from_slice(&[entry_type]);

					matt.extend_from_slice(&[entry_type]);
					for value in vec {
						matt.extend_from_slice(&value.to_le_bytes());
					}
				}
			};

			matb.extend_from_slice(&(prop_name_len + 1).to_le_bytes());
			matb.extend_from_slice(&[prop_name, &[0]].concat());
		}

		(
			(
				matt,
				ResourceMetadata {
					id: self.factory,
					resource_type: "MATT".try_into().unwrap(),
					compressed: ResourceMetadata::infer_compressed("MATT".try_into().unwrap()),
					scrambled: ResourceMetadata::infer_scrambled("MATT".try_into().unwrap()),
					references: matt_references
				}
			),
			(
				matb,
				ResourceMetadata {
					id: self.blueprint,
					resource_type: "MATB".try_into().unwrap(),
					compressed: ResourceMetadata::infer_compressed("MATB".try_into().unwrap()),
					scrambled: ResourceMetadata::infer_scrambled("MATB".try_into().unwrap()),
					references: vec![ResourceReference {
						resource: "00A1595C0918E2C9".parse().unwrap(),
						flags: ReferenceFlags::default()
					}]
				}
			)
		)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum IntermediateMaterialProperty {
	AlphaReference(u32),
	AlphaTestEnabled(u32),
	BlendEnabled(u32),
	Binder(Vec<IntermediateMaterialProperty>),
	BlendMode(String),
	Color(Vec<IntermediateMaterialProperty>),
	Color4(Vec<IntermediateMaterialProperty>),
	CullingMode(String),
	DecalBlendDiffuse(u32),
	DecalBlendEmission(u32),
	DecalBlendNormal(u32),
	DecalBlendRoughness(u32),
	DecalBlendSpecular(u32),
	Enabled(u32),
	FogEnabled(u32),
	FloatValue(Vec<IntermediateMaterialProperty>),
	Instance(Vec<IntermediateMaterialProperty>),
	Name(String),
	Opacity(f32),
	RenderState(Vec<IntermediateMaterialProperty>),
	SubsurfaceValue(f32),
	SubsurfaceBlue(f32),
	SubsurfaceGreen(f32),
	SubsurfaceRed(f32),
	Tags(String),
	Texture(Vec<IntermediateMaterialProperty>),
	TilingU(String),
	TilingV(String),
	TextureID(Option<RuntimeID>),
	Type(String),
	Value(FloatVal),
	ZBias(u32),
	ZOffset(f32)
}

#[derive(Clone, Debug, PartialEq)]
pub enum FloatVal {
	Single(f32),
	Vector(Vec<f32>)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialInstance {
	pub id: RuntimeID,

	pub name: String,

	#[cfg_attr(feature = "serde", serde(rename = "type"))]
	pub material_type: MaterialType,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "String::is_empty"))]
	#[cfg_attr(feature = "serde", serde(default))]
	pub tags: String,

	pub class: Option<RuntimeID>,
	pub descriptor: Option<RuntimeID>,
	pub class_flags: ClassFlags,
	pub instance_flags: InstanceFlags,

	#[cfg_attr(feature = "serde", serde(flatten))]
	pub binder: Binder
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum MaterialType {
	Standard,
	StandardLinked,
	StandardWeighted,
	SpriteParticleAO,
	SpriteParticleVelocity
}

impl FromStr for MaterialType {
	type Err = MaterialError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Standard" => Ok(Self::Standard),
			"StandardLinked" => Ok(Self::StandardLinked),
			"StandardWeighted" => Ok(Self::StandardWeighted),
			"SpriteParticle_AO" => Ok(Self::SpriteParticleAO),
			"SpriteParticleVelocity" => Ok(Self::SpriteParticleVelocity),
			_ => Err(MaterialError::InvalidMaterialType(s.into()))
		}
	}
}

impl Display for MaterialType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Standard => write!(f, "Standard"),
			Self::StandardLinked => write!(f, "StandardLinked"),
			Self::StandardWeighted => write!(f, "StandardWeighted"),
			Self::SpriteParticleAO => write!(f, "SpriteParticle_AO"),
			Self::SpriteParticleVelocity => write!(f, "SpriteParticleVelocity")
		}
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ClassFlags {
	#[cfg_attr(feature = "serde", serde(rename = "reflection2D"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub reflection_2d: bool,

	#[cfg_attr(feature = "serde", serde(rename = "refraction2D"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub refraction_2d: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub lighting: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub discard: bool,

	#[cfg_attr(feature = "serde", serde(rename = "lmSkin"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub lm_skin: bool,

	#[cfg_attr(feature = "serde", serde(rename = "primStandard"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub prim_standard: bool,

	#[cfg_attr(feature = "serde", serde(rename = "primLinked"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub prim_linked: bool,

	#[cfg_attr(feature = "serde", serde(rename = "primWeighted"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub prim_weighted: bool,

	#[cfg_attr(feature = "serde", serde(rename = "dofOverride"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub dof_override: bool,

	#[cfg_attr(feature = "serde", serde(rename = "usesDefaultVS"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub uses_default_vs: bool,

	#[cfg_attr(feature = "serde", serde(rename = "usesSpriteSAVS"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub uses_sprite_sa_vs: bool,

	#[cfg_attr(feature = "serde", serde(rename = "usesSpriteAOVS"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub uses_sprite_ao_vs: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub alpha: bool,

	#[cfg_attr(feature = "serde", serde(rename = "usesSimpleShader"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub uses_simple_shader: bool,

	#[cfg_attr(feature = "serde", serde(rename = "disableInstancing"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub disable_instancing: bool,

	#[cfg_attr(feature = "serde", serde(rename = "lmHair"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub lm_hair: bool,

	#[cfg_attr(feature = "serde", serde(rename = "sampleLighting"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub sample_lighting: bool,

	#[cfg_attr(feature = "serde", serde(rename = "horizonMapping"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub horizon_mapping: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub unknown_1: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub unknown_2: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub unknown_3: bool
}

impl ClassFlags {
	pub fn from_u32(flags: u32) -> Self {
		Self {
			reflection_2d: flags & 0x1 == 0x1,
			refraction_2d: flags & 0x2 == 0x2,
			lighting: flags & 0x4 == 0x4,
			emissive: flags & 0x8 == 0x8,
			discard: flags & 0x10 == 0x10,
			lm_skin: flags & 0x20 == 0x20,
			prim_standard: flags & 0x40 == 0x40,
			prim_linked: flags & 0x80 == 0x80,
			prim_weighted: flags & 0x100 == 0x100,
			dof_override: flags & 0x200 == 0x200,
			uses_default_vs: flags & 0x400 == 0x400,
			uses_sprite_sa_vs: flags & 0x800 == 0x800,
			uses_sprite_ao_vs: flags & 0x1000 == 0x1000,
			alpha: flags & 0x2000 == 0x2000,
			uses_simple_shader: flags & 0x4000 == 0x4000,
			disable_instancing: flags & 0x8000 == 0x8000,
			lm_hair: flags & 0x10000 == 0x10000,
			sample_lighting: flags & 0x20000 == 0x20000,
			horizon_mapping: flags & 0x40000 == 0x40000,
			unknown_1: flags & 0x80000 == 0x80000,
			unknown_2: flags & 0x100000 == 0x100000,
			unknown_3: flags & 0x200000 == 0x200000
		}
	}

	pub fn as_u32(&self) -> u32 {
		let mut flags = 0;

		if self.reflection_2d {
			flags |= 0x1;
		}

		if self.refraction_2d {
			flags |= 0x2;
		}

		if self.lighting {
			flags |= 0x4;
		}

		if self.emissive {
			flags |= 0x8;
		}

		if self.discard {
			flags |= 0x10;
		}

		if self.lm_skin {
			flags |= 0x20;
		}

		if self.prim_standard {
			flags |= 0x40;
		}

		if self.prim_linked {
			flags |= 0x80;
		}

		if self.prim_weighted {
			flags |= 0x100;
		}

		if self.dof_override {
			flags |= 0x200;
		}

		if self.uses_default_vs {
			flags |= 0x400;
		}

		if self.uses_sprite_sa_vs {
			flags |= 0x800;
		}

		if self.uses_sprite_ao_vs {
			flags |= 0x1000;
		}

		if self.alpha {
			flags |= 0x2000;
		}

		if self.uses_simple_shader {
			flags |= 0x4000;
		}

		if self.disable_instancing {
			flags |= 0x8000;
		}

		if self.lm_hair {
			flags |= 0x10000;
		}

		if self.sample_lighting {
			flags |= 0x20000;
		}

		if self.horizon_mapping {
			flags |= 0x40000;
		}

		if self.unknown_1 {
			flags |= 0x80000;
		}

		if self.unknown_2 {
			flags |= 0x100000;
		}

		if self.unknown_3 {
			flags |= 0x200000;
		}

		flags
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct InstanceFlags {
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub opaque_emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub trans_emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub trans_add_emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub opaque_lit: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub trans_lit: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub decal: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub refractive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub lm_skin: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub lm_hair: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub force_emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub disable_shader_lod: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub discard: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub decal_emissive: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub water_clipping: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub sample_lighting: bool,

	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub exclude_global_shadows: bool
}

impl InstanceFlags {
	pub fn from_u32(flags: u32) -> Self {
		Self {
			opaque_emissive: flags & 0x1 == 0x1,
			trans_emissive: flags & 0x2 == 0x2,
			trans_add_emissive: flags & 0x4 == 0x4,
			opaque_lit: flags & 0x8 == 0x8,
			trans_lit: flags & 0x10 == 0x10,
			decal: flags & 0x20 == 0x20,
			refractive: flags & 0x40 == 0x40,
			lm_skin: flags & 0x80 == 0x80,
			lm_hair: flags & 0x100 == 0x100,
			force_emissive: flags & 0x200 == 0x200,
			disable_shader_lod: flags & 0x400 == 0x400,
			discard: flags & 0x800 == 0x800,
			decal_emissive: flags & 0x1000 == 0x1000,
			water_clipping: flags & 0x2000 == 0x2000,
			sample_lighting: flags & 0x4000 == 0x4000,
			exclude_global_shadows: flags & 0x8000 == 0x8000
		}
	}

	pub fn as_u32(&self) -> u32 {
		let mut flags = 0;

		if self.opaque_emissive {
			flags |= 0x1;
		}

		if self.trans_emissive {
			flags |= 0x2;
		}

		if self.trans_add_emissive {
			flags |= 0x4;
		}

		if self.opaque_lit {
			flags |= 0x8;
		}

		if self.trans_lit {
			flags |= 0x10;
		}

		if self.decal {
			flags |= 0x20;
		}

		if self.refractive {
			flags |= 0x40;
		}

		if self.lm_skin {
			flags |= 0x80;
		}

		if self.lm_hair {
			flags |= 0x100;
		}

		if self.force_emissive {
			flags |= 0x200;
		}

		if self.disable_shader_lod {
			flags |= 0x400;
		}

		if self.discard {
			flags |= 0x800;
		}

		if self.decal_emissive {
			flags |= 0x1000;
		}

		if self.water_clipping {
			flags |= 0x2000;
		}

		if self.sample_lighting {
			flags |= 0x4000;
		}

		if self.exclude_global_shadows {
			flags |= 0x8000;
		}

		flags
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
pub struct Binder {
	pub render_state: RenderState,
	pub properties: IndexMap<String, MaterialPropertyValue>
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
pub struct RenderState {
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_default_renderstate"))]
	#[cfg_attr(feature = "serde", serde(default = "default_renderstate"))]
	pub name: Option<String>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub enabled: Option<bool>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub blend_enabled: Option<bool>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub blend_mode: Option<BlendMode>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub decal_blend_diffuse: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub decal_blend_normal: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub decal_blend_specular: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub decal_blend_roughness: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub decal_blend_emission: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub alpha_test_enabled: Option<bool>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub alpha_reference: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub fog_enabled: Option<bool>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub opacity: Option<f32>,

	pub culling_mode: CullingMode,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub z_bias: Option<u32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub z_offset: Option<f32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub subsurface_red: Option<f32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub subsurface_green: Option<f32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub subsurface_blue: Option<f32>,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub subsurface_value: Option<f32>
}

fn is_default_renderstate(value: &Option<String>) -> bool {
	*value == Some("RenderState".to_owned()) || value.is_none()
}

fn default_renderstate() -> Option<String> {
	Some("RenderState".to_owned())
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum CullingMode {
	DontCare,
	OneSided,
	TwoSided
}

impl FromStr for CullingMode {
	type Err = MaterialError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"DontCare" => Ok(Self::DontCare),
			"OneSided" => Ok(Self::OneSided),
			"TwoSided" => Ok(Self::TwoSided),
			_ => Err(MaterialError::InvalidCullingMode(s.into()))
		}
	}
}

impl Display for CullingMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::DontCare => write!(f, "DontCare"),
			Self::OneSided => write!(f, "OneSided"),
			Self::TwoSided => write!(f, "TwoSided")
		}
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum BlendMode {
	Add,
	Sub,
	Trans,
	TransOnOpaque,
	Opaque,
	TransPremultipliedAlpha
}

impl FromStr for BlendMode {
	type Err = MaterialError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"ADD" => Ok(Self::Add),
			"SUB" => Ok(Self::Sub),
			"TRANS" => Ok(Self::Trans),
			"TRANS_ON_OPAQUE" => Ok(Self::TransOnOpaque),
			"OPAQUE" => Ok(Self::Opaque),
			"TRANS_PREMULTIPLIED_ALPHA" => Ok(Self::TransPremultipliedAlpha),
			_ => Err(MaterialError::InvalidBlendMode(s.into()))
		}
	}
}

impl Display for BlendMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Add => write!(f, "ADD"),
			Self::Sub => write!(f, "SUB"),
			Self::Trans => write!(f, "TRANS"),
			Self::TransOnOpaque => write!(f, "TRANS_ON_OPAQUE"),
			Self::Opaque => write!(f, "OPAQUE"),
			Self::TransPremultipliedAlpha => write!(f, "TRANS_PREMULTIPLIED_ALPHA")
		}
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialPropertyValue {
	Float {
		enabled: bool,
		value: f32
	},
	Vector {
		enabled: bool,
		value: Vec<f32>
	},
	Texture {
		enabled: bool,
		value: Option<RuntimeID>,

		#[cfg_attr(feature = "serde", serde(rename = "tilingU"))]
		#[cfg_attr(feature = "serde", serde(skip_serializing_if = "String::is_empty"))]
		#[cfg_attr(feature = "serde", serde(default))]
		tiling_u: String,

		#[cfg_attr(feature = "serde", serde(rename = "tilingV"))]
		#[cfg_attr(feature = "serde", serde(skip_serializing_if = "String::is_empty"))]
		#[cfg_attr(feature = "serde", serde(default))]
		tiling_v: String,

		#[cfg_attr(feature = "serde", serde(rename = "type"))]
		texture_type: String
	},
	Colour {
		enabled: bool,
		value: String
	}
}

impl MaterialInstance {
	/// Parse a material instance (MATI).
	#[try_fn]
	pub fn parse(mati_data: &[u8], mati_metadata: &ResourceMetadata) -> Result<Self> {
		let mut mati = Cursor::new(mati_data);

		let header_offset = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		mati.seek(SeekFrom::Start(header_offset.into()))?;

		let type_offset = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		let material_type = String::from_utf8(
			mati_data
				.iter()
				.skip(type_offset as usize)
				.take_while(|x| **x != 0)
				.cloned()
				.collect()
		)?;

		let mate_index = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		let class_flags = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		let instance_flags = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		let eres_index = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		// Skipped: lImpactMaterial, lEffectResource
		let _ = {
			let mut x = [0u8; 8];
			mati.read_exact(&mut x)?;
			x[0]
		};

		let instance_offset = u32::from_le_bytes({
			let mut x = [0u8; 4];
			mati.read_exact(&mut x)?;
			x
		});

		let (name, tags, binder) = parse_instance(parse_material_property(
			mati_data,
			&mati_metadata.references,
			instance_offset.into()
		)?)?;

		Self {
			id: mati_metadata.id,
			name,
			material_type: material_type.parse()?,
			tags,
			class: mati_metadata.references.get(mate_index as usize).map(|x| x.resource),
			descriptor: mati_metadata.references.get(eres_index as usize).map(|x| x.resource),
			class_flags: ClassFlags::from_u32(class_flags),
			instance_flags: InstanceFlags::from_u32(instance_flags),
			binder
		}
	}

	/// Generate the game binary for this material instance.
	#[try_fn]
	pub fn generate(self) -> Result<(Vec<u8>, ResourceMetadata)> {
		let mut mati = vec![];
		let mut mati_references = vec![];

		// Header offset (placeholder)
		mati.extend_from_slice(&0u32.to_le_bytes());

		// Alignment
		while mati.len() % 16 != 0 {
			mati.push(0u8);
		}

		// Generate instance data
		let instance = IntermediateMaterialProperty::Instance(vec![
			IntermediateMaterialProperty::Name(self.name),
			IntermediateMaterialProperty::Tags(self.tags),
			to_intermediate(self.binder)?,
		]);

		let (instance_data, instance_resources) = generate_property(mati.len() as u32, &mut mati_references, instance)?;

		let Some(resources) = instance_resources else {
			unreachable!()
		};

		mati.extend_from_slice(&resources);

		// Update the instance offset
		let instance_offset = mati.len() as u32;

		mati.extend_from_slice(&instance_data);

		// Update the type offset
		let type_offset = mati.len() as u32;

		mati.extend_from_slice(&[self.material_type.to_string().as_bytes(), &[0]].concat());

		// Alignment
		while mati.len() % 16 != 0 {
			mati.push(0u8);
		}

		// Update header offset
		mati[0] = (mati.len() as u32).to_le_bytes()[0];
		mati[1] = (mati.len() as u32).to_le_bytes()[1];
		mati[2] = (mati.len() as u32).to_le_bytes()[2];
		mati[3] = (mati.len() as u32).to_le_bytes()[3];

		// Type offset
		mati.extend_from_slice(&type_offset.to_le_bytes());

		if let Some(class) = self.class {
			mati_references.push(ResourceReference {
				resource: class,
				flags: ReferenceFlags::default()
			});

			// MATE index
			mati.extend_from_slice(&(mati_references.len() as u32 - 1).to_le_bytes());
		} else {
			// MATE index
			mati.extend_from_slice(&u32::MAX.to_le_bytes());
		}

		// Class flags
		mati.extend_from_slice(&self.class_flags.as_u32().to_le_bytes());

		// Instance flags
		mati.extend_from_slice(&self.instance_flags.as_u32().to_le_bytes());

		if let Some(descriptor) = self.descriptor {
			mati_references.push(ResourceReference {
				resource: descriptor,
				flags: ReferenceFlags {
					reference_type: ReferenceType::Normal,
					acquired: false,
					language_code: 0b0001_1111
				}
			});

			// ERES index
			mati.extend_from_slice(&(mati_references.len() as u32 - 1).to_le_bytes());
		} else {
			// ERES index
			mati.extend_from_slice(&u32::MAX.to_le_bytes());
		}

		// Skipped: lImpactMaterial, lEffectResource
		mati.extend_from_slice(&[0u8; 8]);

		// Instance offset
		mati.extend_from_slice(&instance_offset.to_le_bytes());

		// Constant: 3
		mati.extend_from_slice(&3u32.to_le_bytes());

		// 12 zero bytes
		mati.extend_from_slice(&0u32.to_le_bytes());
		mati.extend_from_slice(&0u32.to_le_bytes());
		mati.extend_from_slice(&0u32.to_le_bytes());

		(
			mati,
			ResourceMetadata {
				id: self.id,
				resource_type: "MATI".try_into().unwrap(),
				compressed: ResourceMetadata::infer_compressed("MATI".try_into().unwrap()),
				scrambled: ResourceMetadata::infer_scrambled("MATI".try_into().unwrap()),
				references: mati_references
			}
		)
	}
}

#[try_fn]
fn generate_property(
	all_resources_offset: u32,
	mati_references: &mut Vec<ResourceReference>,
	property: IntermediateMaterialProperty
) -> Result<(Vec<u8>, Option<Vec<u8>>)> {
	match property {
		IntermediateMaterialProperty::AlphaReference(val)
		| IntermediateMaterialProperty::AlphaTestEnabled(val)
		| IntermediateMaterialProperty::BlendEnabled(val)
		| IntermediateMaterialProperty::DecalBlendDiffuse(val)
		| IntermediateMaterialProperty::DecalBlendEmission(val)
		| IntermediateMaterialProperty::DecalBlendNormal(val)
		| IntermediateMaterialProperty::DecalBlendRoughness(val)
		| IntermediateMaterialProperty::DecalBlendSpecular(val)
		| IntermediateMaterialProperty::Enabled(val)
		| IntermediateMaterialProperty::ZBias(val)
		| IntermediateMaterialProperty::FogEnabled(val) => {
			let name = match property {
				IntermediateMaterialProperty::AlphaReference(_) => "AREF",
				IntermediateMaterialProperty::AlphaTestEnabled(_) => "ATST",
				IntermediateMaterialProperty::BlendEnabled(_) => "BENA",
				IntermediateMaterialProperty::DecalBlendDiffuse(_) => "DBDE",
				IntermediateMaterialProperty::DecalBlendEmission(_) => "DBEE",
				IntermediateMaterialProperty::DecalBlendNormal(_) => "DBNE",
				IntermediateMaterialProperty::DecalBlendRoughness(_) => "DBRE",
				IntermediateMaterialProperty::DecalBlendSpecular(_) => "DBSE",
				IntermediateMaterialProperty::Enabled(_) => "ENAB",
				IntermediateMaterialProperty::ZBias(_) => "ZBIA",
				IntermediateMaterialProperty::FogEnabled(_) => "FENA",
				_ => unreachable!()
			};

			let mut data = vec![];

			data.extend_from_slice(&{
				let mut x = name.as_bytes().to_owned();
				x.reverse();
				x
			});
			data.extend_from_slice(&val.to_le_bytes());
			data.extend_from_slice(&[1, 0, 0, 0]); // Count (1 for this type)
			data.extend_from_slice(&[2, 0, 0, 0]); // Type (2 for int)

			(data, None)
		}

		IntermediateMaterialProperty::BlendMode(ref val)
		| IntermediateMaterialProperty::CullingMode(ref val)
		| IntermediateMaterialProperty::Name(ref val)
		| IntermediateMaterialProperty::Tags(ref val)
		| IntermediateMaterialProperty::TilingU(ref val)
		| IntermediateMaterialProperty::TilingV(ref val)
		| IntermediateMaterialProperty::Type(ref val) => {
			let name = match property {
				IntermediateMaterialProperty::BlendMode(_) => "BMOD",
				IntermediateMaterialProperty::CullingMode(_) => "CULL",
				IntermediateMaterialProperty::Name(_) => "NAME",
				IntermediateMaterialProperty::Tags(_) => "TAGS",
				IntermediateMaterialProperty::TilingU(_) => "TILU",
				IntermediateMaterialProperty::TilingV(_) => "TILV",
				IntermediateMaterialProperty::Type(_) => "TYPE",
				_ => unreachable!()
			};

			let mut data = vec![];

			data.extend_from_slice(&{
				let mut x = name.as_bytes().to_owned();
				x.reverse();
				x
			});
			data.extend_from_slice(&all_resources_offset.to_le_bytes()); // Pointer placeholder
			data.extend_from_slice(&(val.len() as u32 + 1).to_le_bytes()); // Count (string length plus null terminator)
			data.extend_from_slice(&[1, 0, 0, 0]); // Type (1 for string)

			let mut resources = [val.as_bytes(), &[0]].concat();

			// Alignment
			while resources.len() % 16 != 0 {
				resources.push(0u8);
			}

			(data, Some(resources))
		}

		IntermediateMaterialProperty::Binder(ref val)
		| IntermediateMaterialProperty::Color(ref val)
		| IntermediateMaterialProperty::Color4(ref val)
		| IntermediateMaterialProperty::FloatValue(ref val)
		| IntermediateMaterialProperty::Instance(ref val)
		| IntermediateMaterialProperty::RenderState(ref val)
		| IntermediateMaterialProperty::Texture(ref val) => {
			let name = match property {
				IntermediateMaterialProperty::Binder(_) => "BIND",
				IntermediateMaterialProperty::Color(_) => "COLO",
				IntermediateMaterialProperty::Color4(_) => "COL4",
				IntermediateMaterialProperty::FloatValue(_) => "FLTV",
				IntermediateMaterialProperty::Instance(_) => "INST",
				IntermediateMaterialProperty::RenderState(_) => "RSTA",
				IntermediateMaterialProperty::Texture(_) => "TEXT",
				_ => unreachable!()
			};

			let mut records = vec![];
			let mut resources = vec![];
			for sub_property in val.iter().cloned() {
				let (record, resource) = generate_property(
					all_resources_offset + (resources.len() as u32),
					mati_references,
					sub_property
				)?;

				records.extend(record);

				if let Some(res) = resource {
					resources.extend(res);
				}
			}

			let resource_chunk_size = resources.len() as u32;

			let mut resources_concat = [resources, records].concat();

			// Alignment
			while resources_concat.len() % 16 != 0 {
				resources_concat.push(0u8);
			}

			let mut data = vec![];

			data.extend_from_slice(&{
				let mut x = name.as_bytes().to_owned();
				x.reverse();
				x
			});
			data.extend_from_slice(&(all_resources_offset + resource_chunk_size).to_le_bytes()); // Pointer
			data.extend_from_slice(&(val.len() as u32).to_le_bytes()); // Count
			data.extend_from_slice(&[3, 0, 0, 0]); // Type (3 for property)

			(data, Some(resources_concat))
		}

		IntermediateMaterialProperty::Opacity(val)
		| IntermediateMaterialProperty::SubsurfaceValue(val)
		| IntermediateMaterialProperty::SubsurfaceBlue(val)
		| IntermediateMaterialProperty::SubsurfaceGreen(val)
		| IntermediateMaterialProperty::SubsurfaceRed(val)
		| IntermediateMaterialProperty::ZOffset(val) => {
			let name = match property {
				IntermediateMaterialProperty::Opacity(_) => "OPAC",
				IntermediateMaterialProperty::ZOffset(_) => "ZOFF",
				IntermediateMaterialProperty::SubsurfaceValue(_) => "SSBW",
				IntermediateMaterialProperty::SubsurfaceBlue(_) => "SSVB",
				IntermediateMaterialProperty::SubsurfaceGreen(_) => "SSVG",
				IntermediateMaterialProperty::SubsurfaceRed(_) => "SSVR",
				_ => unreachable!()
			};

			let mut data = vec![];

			data.extend_from_slice(&{
				let mut x = name.as_bytes().to_owned();
				x.reverse();
				x
			});
			data.extend_from_slice(&val.to_le_bytes());
			data.extend_from_slice(&[1, 0, 0, 0]); // Count (1 for this type)
			data.extend_from_slice(&[0, 0, 0, 0]); // Type (0 for float)

			(data, None)
		}

		IntermediateMaterialProperty::TextureID(val) => {
			let name = "TXID";

			let mut data = vec![];

			data.extend_from_slice(&{
				let mut x = name.as_bytes().to_owned();
				x.reverse();
				x
			});

			if let Some(id) = val {
				mati_references.push(ResourceReference {
					resource: id,
					flags: ReferenceFlags {
						reference_type: ReferenceType::Normal,
						acquired: false,
						language_code: 0b0001_1111
					}
				});

				data.extend_from_slice(&((mati_references.len() - 1) as u32).to_le_bytes());
			} else {
				data.extend_from_slice(&u32::MAX.to_le_bytes());
			}

			data.extend_from_slice(&[1, 0, 0, 0]); // Count (1 for this type)
			data.extend_from_slice(&[2, 0, 0, 0]); // Type (2 for int)

			(data, None)
		}

		IntermediateMaterialProperty::Value(val) => {
			let name = "VALU";

			match val {
				FloatVal::Single(val) => {
					let mut data = vec![];

					data.extend_from_slice(&{
						let mut x = name.as_bytes().to_owned();
						x.reverse();
						x
					});
					data.extend_from_slice(&val.to_le_bytes());
					data.extend_from_slice(&[1, 0, 0, 0]); // Count (1 for this type)
					data.extend_from_slice(&[0, 0, 0, 0]); // Type (0 for float)

					(data, None)
				}

				FloatVal::Vector(val) => {
					let mut data = vec![];

					data.extend_from_slice(&{
						let mut x = name.as_bytes().to_owned();
						x.reverse();
						x
					});
					data.extend_from_slice(&all_resources_offset.to_le_bytes()); // Pointer placeholder
					data.extend_from_slice(&(val.len() as u32).to_le_bytes()); // Count
					data.extend_from_slice(&[0, 0, 0, 0]); // Type (0 for float)

					let mut resources = val.into_iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();

					// Alignment
					while resources.len() % 16 != 0 {
						resources.push(0u8);
					}

					(data, Some(resources))
				}
			}
		}
	}
}

#[try_fn]
fn parse_material_property(
	mati_data: &[u8],
	mati_references: &[ResourceReference],
	start: u64
) -> Result<IntermediateMaterialProperty> {
	let mut mati = Cursor::new(mati_data);
	mati.seek(SeekFrom::Start(start))?;

	let name = {
		let mut x = [0u8; 4];
		mati.read_exact(&mut x)?;
		x.into_iter().rev().map(|x| x as char).collect::<String>()
	};

	let data = {
		let mut x = [0u8; 4];
		mati.read_exact(&mut x)?;
		x
	};

	let count = u32::from_le_bytes({
		let mut x = [0u8; 4];
		mati.read_exact(&mut x)?;
		x
	});

	let ty = u32::from_le_bytes({
		let mut x = [0u8; 4];
		mati.read_exact(&mut x)?;
		x
	}) as u8;

	match ty {
		// Float value
		0 => {
			if count == 1 {
				// Single float value

				let value = f32::from_le_bytes(data);

				match name.as_ref() {
					"OPAC" => IntermediateMaterialProperty::Opacity(value),
					"ZOFF" => IntermediateMaterialProperty::ZOffset(value),
					"SSBW" => IntermediateMaterialProperty::SubsurfaceValue(value),
					"SSVB" => IntermediateMaterialProperty::SubsurfaceBlue(value),
					"SSVG" => IntermediateMaterialProperty::SubsurfaceGreen(value),
					"SSVR" => IntermediateMaterialProperty::SubsurfaceRed(value),
					"VALU" => IntermediateMaterialProperty::Value(FloatVal::Single(value)),

					_ => return Err(MaterialError::IncorrectType(name, ty))
				}
			} else {
				// Vector
				mati.seek(SeekFrom::Start(u32::from_le_bytes(data).into()))?;

				let mut value = vec![];
				for _ in 0..count {
					value.push(f32::from_le_bytes({
						let mut x = [0u8; 4];
						mati.read_exact(&mut x)?;
						x
					}));
				}

				match name.as_ref() {
					"VALU" => IntermediateMaterialProperty::Value(FloatVal::Vector(value)),

					_ => return Err(MaterialError::IncorrectType(name, ty))
				}
			}
		}

		// String value
		1 => {
			let value = String::from_utf8(
				mati_data
					.iter()
					.skip(u32::from_le_bytes(data) as usize)
					.take_while(|x| **x != 0)
					.cloned()
					.collect()
			)?;

			match name.as_ref() {
				"BMOD" => IntermediateMaterialProperty::BlendMode(value),
				"CULL" => IntermediateMaterialProperty::CullingMode(value),
				"NAME" => IntermediateMaterialProperty::Name(value),
				"TAGS" => IntermediateMaterialProperty::Tags(value),
				"TILU" => IntermediateMaterialProperty::TilingU(value),
				"TILV" => IntermediateMaterialProperty::TilingV(value),
				"TYPE" => IntermediateMaterialProperty::Type(value),

				_ => return Err(MaterialError::IncorrectType(name, ty))
			}
		}

		// Int value
		2 => {
			let value = u32::from_le_bytes(data);

			match name.as_ref() {
				"AREF" => IntermediateMaterialProperty::AlphaReference(value),
				"ATST" => IntermediateMaterialProperty::AlphaTestEnabled(value),
				"BENA" => IntermediateMaterialProperty::BlendEnabled(value),
				"DBDE" => IntermediateMaterialProperty::DecalBlendDiffuse(value),
				"DBEE" => IntermediateMaterialProperty::DecalBlendEmission(value),
				"DBNE" => IntermediateMaterialProperty::DecalBlendNormal(value),
				"DBRE" => IntermediateMaterialProperty::DecalBlendRoughness(value),
				"DBSE" => IntermediateMaterialProperty::DecalBlendSpecular(value),
				"ENAB" => IntermediateMaterialProperty::Enabled(value),
				"FENA" => IntermediateMaterialProperty::FogEnabled(value),
				"ZBIA" => IntermediateMaterialProperty::ZBias(value),

				"TXID" => IntermediateMaterialProperty::TextureID(if value != 4294967295 {
					Some(
						mati_references
							.get(value as usize)
							.ok_or(MaterialError::InvalidDependency(value as usize))?
							.resource
					)
				} else {
					None
				}),

				_ => return Err(MaterialError::IncorrectType(name, ty))
			}
		}

		// Property value
		3 => {
			mati.seek(SeekFrom::Start(u32::from_le_bytes(data).into()))?;

			let mut values = vec![];
			for _ in 0..count {
				values.push(parse_material_property(mati_data, mati_references, mati.position())?);
				mati.seek(SeekFrom::Current(0x10))?;
			}

			match name.as_ref() {
				"BIND" => IntermediateMaterialProperty::Binder(values),
				"COLO" => IntermediateMaterialProperty::Color(values),
				"COL4" => IntermediateMaterialProperty::Color4(values),
				"FLTV" => IntermediateMaterialProperty::FloatValue(values),
				"INST" => IntermediateMaterialProperty::Instance(values),
				"RSTA" => IntermediateMaterialProperty::RenderState(values),
				"TEXT" => IntermediateMaterialProperty::Texture(values),

				_ => return Err(MaterialError::IncorrectType(name, ty))
			}
		}

		_ => return Err(MaterialError::UnrecognisedEntryType(ty))
	}
}

#[try_fn]
fn parse_instance(instance: IntermediateMaterialProperty) -> Result<(String, String, Binder)> {
	let IntermediateMaterialProperty::Instance(properties) = instance else {
		return Err(MaterialError::InstanceNotTopLevel);
	};

	(
		properties
			.iter()
			.find_map(|x| match x {
				IntermediateMaterialProperty::Name(x) => Some(x),
				_ => None
			})
			.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?
			.to_owned(),
		properties
			.iter()
			.find_map(|x| match x {
				IntermediateMaterialProperty::Tags(x) => Some(x),
				_ => None
			})
			.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TAGS".into()))?
			.to_owned(),
		{
			let binder = properties
				.iter()
				.find_map(|x| match x {
					IntermediateMaterialProperty::Binder(x) => Some(x),
					_ => None
				})
				.ok_or_else(|| MaterialError::RequiredPropertyNotFound("BIND".into()))?;

			Binder {
				render_state: {
					let props = binder
						.iter()
						.find_map(|x| match x {
							IntermediateMaterialProperty::RenderState(x) => Some(x),
							_ => None
						})
						.ok_or_else(|| MaterialError::RequiredPropertyNotFound("RSTA".into()))?
						.to_owned();

					RenderState {
						name: props.iter().find_map(|x| match x {
							IntermediateMaterialProperty::Name(x) => Some(x.to_owned()),
							_ => None
						}),
						enabled: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::Enabled(x) => Some(x != 0),
							_ => None
						}),
						blend_enabled: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::BlendEnabled(x) => Some(x != 0),
							_ => None
						}),
						blend_mode: props
							.iter()
							.find_map(|x| match *x {
								IntermediateMaterialProperty::BlendMode(ref x) => Some(x.parse()),
								_ => None
							})
							.transpose()?,
						decal_blend_diffuse: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::DecalBlendDiffuse(x) => Some(x),
							_ => None
						}),
						decal_blend_normal: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::DecalBlendNormal(x) => Some(x),
							_ => None
						}),
						decal_blend_specular: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::DecalBlendSpecular(x) => Some(x),
							_ => None
						}),
						decal_blend_roughness: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::DecalBlendRoughness(x) => Some(x),
							_ => None
						}),
						decal_blend_emission: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::DecalBlendEmission(x) => Some(x),
							_ => None
						}),
						alpha_test_enabled: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::AlphaTestEnabled(x) => Some(x != 0),
							_ => None
						}),
						alpha_reference: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::AlphaReference(x) => Some(x),
							_ => None
						}),
						fog_enabled: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::FogEnabled(x) => Some(x != 0),
							_ => None
						}),
						opacity: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::Opacity(x) => Some(x),
							_ => None
						}),
						culling_mode: props
							.iter()
							.find_map(|x| match *x {
								IntermediateMaterialProperty::CullingMode(ref x) => Some(x.parse()),
								_ => None
							})
							.transpose()?
							.ok_or_else(|| MaterialError::RequiredPropertyNotFound("CULL".into()))?,
						z_bias: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::ZBias(x) => Some(x),
							_ => None
						}),
						z_offset: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::ZOffset(x) => Some(x),
							_ => None
						}),
						subsurface_red: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::SubsurfaceRed(x) => Some(x),
							_ => None
						}),
						subsurface_green: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::SubsurfaceGreen(x) => Some(x),
							_ => None
						}),
						subsurface_blue: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::SubsurfaceBlue(x) => Some(x),
							_ => None
						}),
						subsurface_value: props.iter().find_map(|x| match *x {
							IntermediateMaterialProperty::SubsurfaceValue(x) => Some(x),
							_ => None
						})
					}
				},

				properties: binder
					.iter()
					.filter(|x| !matches!(x, IntermediateMaterialProperty::RenderState(_)))
					.map(|x| {
						Ok(match x {
							IntermediateMaterialProperty::FloatValue(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Value(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("VALU".into()))?;

								(
									name.to_owned(),
									match value {
										FloatVal::Single(x) => MaterialPropertyValue::Float {
											enabled: *enabled != 0,
											value: x.to_owned()
										},
										FloatVal::Vector(x) => MaterialPropertyValue::Vector {
											enabled: *enabled != 0,
											value: x.to_owned()
										}
									}
									.to_owned()
								)
							}

							IntermediateMaterialProperty::Texture(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let tiling_u = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::TilingU(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TILU".into()))?;

								let tiling_v = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::TilingV(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TILV".into()))?;

								let texture_id = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::TextureID(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TXID".into()))?;

								let texture_type = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Type(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TYPE".into()))?;

								(
									name.to_owned(),
									MaterialPropertyValue::Texture {
										enabled: *enabled != 0,
										value: texture_id.to_owned(),
										tiling_u: tiling_u.to_owned(),
										tiling_v: tiling_v.to_owned(),
										texture_type: texture_type.to_owned()
									}
								)
							}

							IntermediateMaterialProperty::Color(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Value(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("VALU".into()))?;

								let FloatVal::Vector(value) = value else {
									return Err(MaterialError::InvalidColor(name.to_owned()));
								};

								(
									name.to_owned(),
									if value.len() == 3 {
										MaterialPropertyValue::Colour {
											enabled: *enabled != 0,
											value: format!(
												"#{:0>2x}{:0>2x}{:0>2x}",
												(value[0] * 255.0).round() as u8,
												(value[1] * 255.0).round() as u8,
												(value[2] * 255.0).round() as u8
											)
										}
									} else {
										return Err(MaterialError::InvalidColor(name.to_owned()));
									}
								)
							}

							IntermediateMaterialProperty::Color4(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										IntermediateMaterialProperty::Value(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("VALU".into()))?;

								let FloatVal::Vector(value) = value else {
									return Err(MaterialError::InvalidColor(name.to_owned()));
								};

								(
									name.to_owned(),
									if value.len() == 4 {
										MaterialPropertyValue::Colour {
											enabled: *enabled != 0,
											value: format!(
												"#{:0>2x}{:0>2x}{:0>2x}{:0>2x}",
												(value[0] * 255.0).round() as u8,
												(value[1] * 255.0).round() as u8,
												(value[2] * 255.0).round() as u8,
												(value[3] * 255.0).round() as u8
											)
										}
									} else {
										return Err(MaterialError::InvalidColor(name.to_owned()));
									}
								)
							}

							_ => return Err(MaterialError::PropertyAtWrongLevel(x.to_owned()))
						})
					})
					.collect::<Result<_>>()?
			}
		}
	)
}

#[try_fn]
fn to_intermediate(binder: Binder) -> Result<IntermediateMaterialProperty> {
	IntermediateMaterialProperty::Binder(
		[
			vec![IntermediateMaterialProperty::RenderState({
				let mut props = Vec::new();

				// Unused by the game but added for completeness
				props.push(IntermediateMaterialProperty::Name("RenderState".into()));

				if let Some(enabled) = binder.render_state.enabled {
					props.push(IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }));
				}

				if let Some(blend_enabled) = binder.render_state.blend_enabled {
					props.push(IntermediateMaterialProperty::BlendEnabled(if blend_enabled {
						1
					} else {
						0
					}));
				}

				if let Some(blend_mode) = &binder.render_state.blend_mode {
					props.push(IntermediateMaterialProperty::BlendMode(blend_mode.to_string()));
				}

				if let Some(decal_blend_diffuse) = binder.render_state.decal_blend_diffuse {
					props.push(IntermediateMaterialProperty::DecalBlendDiffuse(decal_blend_diffuse));
				}

				if let Some(decal_blend_normal) = binder.render_state.decal_blend_normal {
					props.push(IntermediateMaterialProperty::DecalBlendNormal(decal_blend_normal));
				}

				if let Some(decal_blend_specular) = binder.render_state.decal_blend_specular {
					props.push(IntermediateMaterialProperty::DecalBlendSpecular(decal_blend_specular));
				}

				if let Some(decal_blend_roughness) = binder.render_state.decal_blend_roughness {
					props.push(IntermediateMaterialProperty::DecalBlendRoughness(decal_blend_roughness));
				}

				if let Some(decal_blend_emission) = binder.render_state.decal_blend_emission {
					props.push(IntermediateMaterialProperty::DecalBlendEmission(decal_blend_emission));
				}

				if let Some(alpha_test_enabled) = binder.render_state.alpha_test_enabled {
					props.push(IntermediateMaterialProperty::AlphaTestEnabled(if alpha_test_enabled {
						1
					} else {
						0
					}));
				}

				if let Some(alpha_reference) = binder.render_state.alpha_reference {
					props.push(IntermediateMaterialProperty::AlphaReference(alpha_reference));
				}

				if let Some(fog_enabled) = binder.render_state.fog_enabled {
					props.push(IntermediateMaterialProperty::FogEnabled(if fog_enabled {
						1
					} else {
						0
					}));
				}

				if let Some(opacity) = binder.render_state.opacity {
					props.push(IntermediateMaterialProperty::Opacity(opacity));
				}

				props.push(IntermediateMaterialProperty::CullingMode(
					binder.render_state.culling_mode.to_string()
				));

				if let Some(z_bias) = binder.render_state.z_bias {
					props.push(IntermediateMaterialProperty::ZBias(z_bias));
				}

				if let Some(z_offset) = binder.render_state.z_offset {
					props.push(IntermediateMaterialProperty::ZOffset(z_offset));
				}

				if let Some(subsurface_red) = binder.render_state.subsurface_red {
					props.push(IntermediateMaterialProperty::SubsurfaceRed(subsurface_red));
				}

				if let Some(subsurface_green) = binder.render_state.subsurface_green {
					props.push(IntermediateMaterialProperty::SubsurfaceGreen(subsurface_green));
				}

				if let Some(subsurface_blue) = binder.render_state.subsurface_blue {
					props.push(IntermediateMaterialProperty::SubsurfaceBlue(subsurface_blue));
				}

				if let Some(subsurface_value) = binder.render_state.subsurface_value {
					props.push(IntermediateMaterialProperty::SubsurfaceValue(subsurface_value));
				}

				props
			})],
			binder
				.properties
				.into_iter()
				.map(|(name, value)| {
					Ok({
						match value {
							MaterialPropertyValue::Float { enabled, value } => {
								IntermediateMaterialProperty::FloatValue(vec![
									IntermediateMaterialProperty::Name(name),
									IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }),
									IntermediateMaterialProperty::Value(FloatVal::Single(value)),
								])
							}

							MaterialPropertyValue::Vector { enabled, value } => {
								IntermediateMaterialProperty::FloatValue(vec![
									IntermediateMaterialProperty::Name(name),
									IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }),
									IntermediateMaterialProperty::Value(FloatVal::Vector(value)),
								])
							}

							MaterialPropertyValue::Texture {
								enabled,
								value,
								tiling_u,
								tiling_v,
								texture_type
							} => IntermediateMaterialProperty::Texture(vec![
								IntermediateMaterialProperty::Name(name),
								IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }),
								IntermediateMaterialProperty::TextureID(value),
								IntermediateMaterialProperty::TilingU(tiling_u),
								IntermediateMaterialProperty::TilingV(tiling_v),
								IntermediateMaterialProperty::Type(texture_type),
							]),

							MaterialPropertyValue::Colour { enabled, value } => {
								if value.len() > 7 {
									let r = u8::from_str_radix(&value.chars().skip(1).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									let g = u8::from_str_radix(&value.chars().skip(3).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									let b = u8::from_str_radix(&value.chars().skip(5).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									let a = u8::from_str_radix(&value.chars().skip(7).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									IntermediateMaterialProperty::Color4(vec![
										IntermediateMaterialProperty::Name(name),
										IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }),
										IntermediateMaterialProperty::Value(FloatVal::Vector(vec![r, g, b, a])),
									])
								} else {
									let r = u8::from_str_radix(&value.chars().skip(1).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									let g = u8::from_str_radix(&value.chars().skip(3).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									let b = u8::from_str_radix(&value.chars().skip(5).take(2).collect::<String>(), 16)?
										as f32 / 255.0;

									IntermediateMaterialProperty::Color(vec![
										IntermediateMaterialProperty::Name(name),
										IntermediateMaterialProperty::Enabled(if enabled { 1 } else { 0 }),
										IntermediateMaterialProperty::Value(FloatVal::Vector(vec![r, g, b])),
									])
								}
							}
						}
					})
				})
				.collect::<Result<Vec<_>>>()?
		]
		.concat()
	)
}
