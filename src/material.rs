use std::{
	fmt::Display,
	io::{Cursor, Read, Seek, SeekFrom},
	str::FromStr
};

use hitman_commons::metadata::{ResourceReference, RuntimeID};
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
	PropertyAtWrongLevel(ParsedMaterialProperty),

	#[error("float vector of correct size for colour expected for property {0}")]
	InvalidColor(String),

	#[error("invalid material type: {0}")]
	InvalidMaterialType(String),

	#[error("invalid culling mode: {0}")]
	InvalidCullingMode(String),

	#[error("invalid blend mode: {0}")]
	InvalidBlendMode(String)
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

#[derive(Clone, Debug)]
pub enum ParsedMaterialProperty {
	AlphaReference(u32),
	AlphaTestEnabled(u32),
	BlendEnabled(u32),
	Binder(Vec<ParsedMaterialProperty>),
	BlendMode(String),
	Color(Vec<ParsedMaterialProperty>),
	Color4(Vec<ParsedMaterialProperty>),
	CullingMode(String),
	DecalBlendDiffuse(u32),
	DecalBlendEmission(u32),
	DecalBlendNormal(u32),
	DecalBlendRoughness(u32),
	DecalBlendSpecular(u32),
	Enabled(u32),
	FogEnabled(u32),
	FloatValue(Vec<ParsedMaterialProperty>),
	Instance(Vec<ParsedMaterialProperty>),
	Name(String),
	Opacity(f32),
	RenderState(Vec<ParsedMaterialProperty>),
	SubsurfaceValue(f32),
	SubsurfaceBlue(f32),
	SubsurfaceGreen(f32),
	SubsurfaceRed(f32),
	Tags(String),
	Texture(Vec<ParsedMaterialProperty>),
	TilingU(String),
	TilingV(String),
	TextureID(Option<RuntimeID>),
	Type(String),
	Value(FloatVal),
	ZBias(u32),
	ZOffset(f32)
}

#[derive(Clone, Debug)]
pub enum FloatVal {
	Single(f32),
	Vector(Vec<f32>)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct Material {
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
	pub uses_sprite_savs: bool,

	#[cfg_attr(feature = "serde", serde(rename = "usesSpriteAOVS"))]
	#[cfg_attr(feature = "serde", serde(default))]
	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "std::ops::Not::not"))]
	pub uses_sprite_aovs: bool,

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
			uses_sprite_savs: flags & 0x800 == 0x800,
			uses_sprite_aovs: flags & 0x1000 == 0x1000,
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

		if self.uses_sprite_savs {
			flags |= 0x800;
		}

		if self.uses_sprite_aovs {
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
#[derive(Clone, Debug)]
pub struct Binder {
	pub render_state: RenderState,
	pub properties: IndexMap<String, MaterialPropertyValue>
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct RenderState {
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
#[derive(Clone, Debug)]
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

impl Material {
	#[try_fn]
	pub fn parse(mati_data: &[u8], mati_references: &[ResourceReference]) -> Result<Self> {
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
			mati_references,
			instance_offset.into()
		)?)?;

		Self {
			name,
			material_type: material_type.parse()?,
			tags,
			class: mati_references.get(mate_index as usize).map(|x| x.resource),
			descriptor: mati_references.get(eres_index as usize).map(|x| x.resource),
			class_flags: ClassFlags::from_u32(class_flags),
			instance_flags: InstanceFlags::from_u32(instance_flags),
			binder
		}
	}
}

#[try_fn]
fn parse_material_property(
	mati_data: &[u8],
	mati_references: &[ResourceReference],
	start: u64
) -> Result<ParsedMaterialProperty> {
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
					"OPAC" => ParsedMaterialProperty::Opacity(value),
					"ZOFF" => ParsedMaterialProperty::ZOffset(value),
					"SSBW" => ParsedMaterialProperty::SubsurfaceValue(value),
					"SSVB" => ParsedMaterialProperty::SubsurfaceBlue(value),
					"SSVG" => ParsedMaterialProperty::SubsurfaceGreen(value),
					"SSVR" => ParsedMaterialProperty::SubsurfaceRed(value),
					"VALU" => ParsedMaterialProperty::Value(FloatVal::Single(value)),

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
					"VALU" => ParsedMaterialProperty::Value(FloatVal::Vector(value)),

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
				"BMOD" => ParsedMaterialProperty::BlendMode(value),
				"CULL" => ParsedMaterialProperty::CullingMode(value),
				"NAME" => ParsedMaterialProperty::Name(value),
				"TAGS" => ParsedMaterialProperty::Tags(value),
				"TILU" => ParsedMaterialProperty::TilingU(value),
				"TILV" => ParsedMaterialProperty::TilingV(value),
				"TYPE" => ParsedMaterialProperty::Type(value),

				_ => return Err(MaterialError::IncorrectType(name, ty))
			}
		}

		// Int value
		2 => {
			let value = u32::from_le_bytes(data);

			match name.as_ref() {
				"AREF" => ParsedMaterialProperty::AlphaReference(value),
				"ATST" => ParsedMaterialProperty::AlphaTestEnabled(value),
				"BENA" => ParsedMaterialProperty::BlendEnabled(value),
				"DBDE" => ParsedMaterialProperty::DecalBlendDiffuse(value),
				"DBEE" => ParsedMaterialProperty::DecalBlendEmission(value),
				"DBNE" => ParsedMaterialProperty::DecalBlendNormal(value),
				"DBRE" => ParsedMaterialProperty::DecalBlendRoughness(value),
				"DBSE" => ParsedMaterialProperty::DecalBlendSpecular(value),
				"ENAB" => ParsedMaterialProperty::Enabled(value),
				"FENA" => ParsedMaterialProperty::FogEnabled(value),
				"ZBIA" => ParsedMaterialProperty::ZBias(value),

				"TXID" => ParsedMaterialProperty::TextureID(if value != 4294967295 {
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
				"BIND" => ParsedMaterialProperty::Binder(values),
				"COLO" => ParsedMaterialProperty::Color(values),
				"COL4" => ParsedMaterialProperty::Color4(values),
				"FLTV" => ParsedMaterialProperty::FloatValue(values),
				"INST" => ParsedMaterialProperty::Instance(values),
				"RSTA" => ParsedMaterialProperty::RenderState(values),
				"TEXT" => ParsedMaterialProperty::Texture(values),

				_ => return Err(MaterialError::IncorrectType(name, ty))
			}
		}

		_ => return Err(MaterialError::UnrecognisedEntryType(ty))
	}
}

#[try_fn]
fn parse_instance(instance: ParsedMaterialProperty) -> Result<(String, String, Binder)> {
	let ParsedMaterialProperty::Instance(properties) = instance else {
		return Err(MaterialError::InstanceNotTopLevel);
	};

	(
		properties
			.iter()
			.find_map(|x| match x {
				ParsedMaterialProperty::Name(x) => Some(x),
				_ => None
			})
			.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?
			.to_owned(),
		properties
			.iter()
			.find_map(|x| match x {
				ParsedMaterialProperty::Tags(x) => Some(x),
				_ => None
			})
			.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TAGS".into()))?
			.to_owned(),
		{
			let binder = properties
				.iter()
				.find_map(|x| match x {
					ParsedMaterialProperty::Binder(x) => Some(x),
					_ => None
				})
				.ok_or_else(|| MaterialError::RequiredPropertyNotFound("BIND".into()))?;

			Binder {
				render_state: {
					let props = binder
						.iter()
						.find_map(|x| match x {
							ParsedMaterialProperty::RenderState(x) => Some(x),
							_ => None
						})
						.ok_or_else(|| MaterialError::RequiredPropertyNotFound("RSTA".into()))?
						.to_owned();

					RenderState {
						enabled: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::Enabled(x) => Some(x != 0),
							_ => None
						}),
						blend_enabled: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::BlendEnabled(x) => Some(x != 0),
							_ => None
						}),
						blend_mode: props
							.iter()
							.find_map(|x| match *x {
								ParsedMaterialProperty::BlendMode(ref x) => Some(x.parse()),
								_ => None
							})
							.transpose()?,
						decal_blend_diffuse: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::DecalBlendDiffuse(x) => Some(x),
							_ => None
						}),
						decal_blend_normal: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::DecalBlendNormal(x) => Some(x),
							_ => None
						}),
						decal_blend_specular: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::DecalBlendSpecular(x) => Some(x),
							_ => None
						}),
						decal_blend_roughness: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::DecalBlendRoughness(x) => Some(x),
							_ => None
						}),
						decal_blend_emission: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::DecalBlendEmission(x) => Some(x),
							_ => None
						}),
						alpha_test_enabled: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::AlphaTestEnabled(x) => Some(x != 0),
							_ => None
						}),
						alpha_reference: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::AlphaReference(x) => Some(x),
							_ => None
						}),
						fog_enabled: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::FogEnabled(x) => Some(x != 0),
							_ => None
						}),
						opacity: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::Opacity(x) => Some(x),
							_ => None
						}),
						culling_mode: props
							.iter()
							.find_map(|x| match *x {
								ParsedMaterialProperty::CullingMode(ref x) => Some(x.parse()),
								_ => None
							})
							.transpose()?
							.ok_or_else(|| MaterialError::RequiredPropertyNotFound("CULL".into()))?,
						z_bias: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::ZBias(x) => Some(x),
							_ => None
						}),
						z_offset: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::ZOffset(x) => Some(x),
							_ => None
						}),
						subsurface_red: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::SubsurfaceRed(x) => Some(x),
							_ => None
						}),
						subsurface_green: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::SubsurfaceGreen(x) => Some(x),
							_ => None
						}),
						subsurface_blue: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::SubsurfaceBlue(x) => Some(x),
							_ => None
						}),
						subsurface_value: props.iter().find_map(|x| match *x {
							ParsedMaterialProperty::SubsurfaceValue(x) => Some(x),
							_ => None
						})
					}
				},

				properties: binder
					.iter()
					.filter(|x| !matches!(x, ParsedMaterialProperty::RenderState(_)))
					.map(|x| {
						Ok(match x {
							ParsedMaterialProperty::FloatValue(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Value(x) => Some(x),
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

							ParsedMaterialProperty::Texture(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let tiling_u = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::TilingU(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TILU".into()))?;

								let tiling_v = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::TilingV(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TILV".into()))?;

								let texture_id = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::TextureID(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("TXID".into()))?;

								let texture_type = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Type(x) => Some(x),
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

							ParsedMaterialProperty::Color(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Value(x) => Some(x),
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

							ParsedMaterialProperty::Color4(x) => {
								let name = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Name(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("NAME".into()))?;

								let enabled = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Enabled(x) => Some(x),
										_ => None
									})
									.ok_or_else(|| MaterialError::RequiredPropertyNotFound("ENAB".into()))?;

								let value = x
									.iter()
									.find_map(|x| match x {
										ParsedMaterialProperty::Value(x) => Some(x),
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
