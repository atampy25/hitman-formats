use std::fmt::Display;

use glacier_commons::metadata::RuntimeID;

#[cfg(feature = "rune")]
pub fn rune_module() -> Result<rune::Module, rune::ContextError> {
	let mut module = rune::Module::with_crate_item("hitman_formats", ["texture"])?;

	module.ty::<TextureMetadata>()?;
	module.ty::<TextureType>()?;
	module.ty::<RenderFormat>()?;
	module.ty::<InterpretAs>()?;

	Ok(module)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::texture))]
#[cfg_attr(feature = "rune", rune_derive(DEBUG_FMT, CLONE, PARTIAL_EQ, EQ))]
pub struct TextureMetadata {
	pub text: RuntimeID,

	#[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
	pub texd: Option<RuntimeID>,

	#[cfg_attr(feature = "serde", serde(rename = "type"))]
	pub texture_type: TextureType,

	#[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_default"))]
	pub format: RenderFormat,

	#[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_default"))]
	pub interpret_as: InterpretAs
}

#[cfg(feature = "serde")]
fn is_default<T: Default + PartialEq>(t: &T) -> bool {
	t == &T::default()
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::texture))]
#[cfg_attr(feature = "rune", rune_derive(DISPLAY_FMT, DEBUG_FMT, CLONE, PARTIAL_EQ, EQ))]
pub enum TextureType {
	Colour,
	Normal,
	Height,
	CompoundNormal,
	Billboard,
	Projection,
	Emission,
	Cubemap,
	UNKNOWN5,
	UNKNOWN517
}

impl Display for TextureType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TextureType::Colour => f.write_str("Colour"),
			TextureType::Normal => f.write_str("Normal"),
			TextureType::Height => f.write_str("Height"),
			TextureType::CompoundNormal => f.write_str("CompoundNormal"),
			TextureType::Billboard => f.write_str("Billboard"),
			TextureType::Projection => f.write_str("Projection"),
			TextureType::Emission => f.write_str("Emission"),
			TextureType::Cubemap => f.write_str("Cubemap"),
			TextureType::UNKNOWN5 => f.write_str("UNKNOWN5"),
			TextureType::UNKNOWN517 => f.write_str("UNKNOWN517")
		}
	}
}

impl From<TextureType> for glacier_texture::enums::TextureType {
	fn from(value: TextureType) -> Self {
		match value {
			TextureType::Colour => Self::Colour,
			TextureType::Normal => Self::Normal,
			TextureType::Height => Self::Height,
			TextureType::CompoundNormal => Self::CompoundNormal,
			TextureType::Billboard => Self::Billboard,
			TextureType::Projection => Self::Projection,
			TextureType::Emission => Self::Emission,
			TextureType::Cubemap => Self::Cubemap,
			TextureType::UNKNOWN5 => Self::UNKNOWN5,
			TextureType::UNKNOWN517 => Self::UNKNOWN517
		}
	}
}

impl From<glacier_texture::enums::TextureType> for TextureType {
	fn from(value: glacier_texture::enums::TextureType) -> Self {
		match value {
			glacier_texture::enums::TextureType::Colour => Self::Colour,
			glacier_texture::enums::TextureType::Normal => Self::Normal,
			glacier_texture::enums::TextureType::Height => Self::Height,
			glacier_texture::enums::TextureType::CompoundNormal => Self::CompoundNormal,
			glacier_texture::enums::TextureType::Billboard => Self::Billboard,
			glacier_texture::enums::TextureType::Projection => Self::Projection,
			glacier_texture::enums::TextureType::Emission => Self::Emission,
			glacier_texture::enums::TextureType::Cubemap => Self::Cubemap,
			glacier_texture::enums::TextureType::UNKNOWN5 => Self::UNKNOWN5,
			glacier_texture::enums::TextureType::UNKNOWN517 => Self::UNKNOWN517,
			_ => panic!("Unknown TextureType value {value:?}")
		}
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::texture))]
#[cfg_attr(feature = "rune", rune_derive(DISPLAY_FMT, DEBUG_FMT, CLONE, PARTIAL_EQ, EQ))]
pub enum RenderFormat {
	R32G32B32A32,
	R16G16B16A16,
	R8G8B8A8,
	R32,
	R8G8,
	A8,
	BC1,
	BC2,
	BC3,
	BC4,
	BC5,
	BC6,

	#[default]
	BC7
}

impl Display for RenderFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RenderFormat::R32G32B32A32 => f.write_str("R32G32B32A32"),
			RenderFormat::R16G16B16A16 => f.write_str("R16G16B16A16"),
			RenderFormat::R8G8B8A8 => f.write_str("R8G8B8A8"),
			RenderFormat::R32 => f.write_str("R32"),
			RenderFormat::R8G8 => f.write_str("R8G8"),
			RenderFormat::A8 => f.write_str("A8"),
			RenderFormat::BC1 => f.write_str("BC1"),
			RenderFormat::BC2 => f.write_str("BC2"),
			RenderFormat::BC3 => f.write_str("BC3"),
			RenderFormat::BC4 => f.write_str("BC4"),
			RenderFormat::BC5 => f.write_str("BC5"),
			RenderFormat::BC6 => f.write_str("BC6"),
			RenderFormat::BC7 => f.write_str("BC7")
		}
	}
}

impl From<RenderFormat> for glacier_texture::enums::RenderFormat {
	fn from(value: RenderFormat) -> Self {
		match value {
			RenderFormat::R32G32B32A32 => Self::R32G32B32A32,
			RenderFormat::R16G16B16A16 => Self::R16G16B16A16,
			RenderFormat::R8G8B8A8 => Self::R8G8B8A8,
			RenderFormat::R32 => Self::R32,
			RenderFormat::R8G8 => Self::R8G8,
			RenderFormat::A8 => Self::A8,
			RenderFormat::BC1 => Self::BC1,
			RenderFormat::BC2 => Self::BC2,
			RenderFormat::BC3 => Self::BC3,
			RenderFormat::BC4 => Self::BC4,
			RenderFormat::BC5 => Self::BC5,
			RenderFormat::BC6 => Self::BC6,
			RenderFormat::BC7 => Self::BC7
		}
	}
}

impl From<glacier_texture::enums::RenderFormat> for RenderFormat {
	fn from(value: glacier_texture::enums::RenderFormat) -> Self {
		match value {
			glacier_texture::enums::RenderFormat::R32G32B32A32 => Self::R32G32B32A32,
			glacier_texture::enums::RenderFormat::R16G16B16A16 => Self::R16G16B16A16,
			glacier_texture::enums::RenderFormat::R8G8B8A8 => Self::R8G8B8A8,
			glacier_texture::enums::RenderFormat::R32 => Self::R32,
			glacier_texture::enums::RenderFormat::R8G8 => Self::R8G8,
			glacier_texture::enums::RenderFormat::A8 => Self::A8,
			glacier_texture::enums::RenderFormat::BC1 => Self::BC1,
			glacier_texture::enums::RenderFormat::BC2 => Self::BC2,
			glacier_texture::enums::RenderFormat::BC3 => Self::BC3,
			glacier_texture::enums::RenderFormat::BC4 => Self::BC4,
			glacier_texture::enums::RenderFormat::BC5 => Self::BC5,
			glacier_texture::enums::RenderFormat::BC6 => Self::BC6,
			glacier_texture::enums::RenderFormat::BC7 => Self::BC7,
			_ => panic!("Unknown RenderFormat value {value:?}")
		}
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "rune", derive(better_rune_derive::Any))]
#[cfg_attr(feature = "rune", rune(item = ::hitman_formats::texture))]
#[cfg_attr(feature = "rune", rune_derive(DISPLAY_FMT, DEBUG_FMT, CLONE, PARTIAL_EQ, EQ))]
pub enum InterpretAs {
	Colour,

	#[default]
	Normal,

	Height,
	CompoundNormal,
	Billboard,
	Cubemap,
	Emission,
	Volume
}

impl Display for InterpretAs {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			InterpretAs::Colour => f.write_str("Colour"),
			InterpretAs::Normal => f.write_str("Normal"),
			InterpretAs::Height => f.write_str("Height"),
			InterpretAs::CompoundNormal => f.write_str("CompoundNormal"),
			InterpretAs::Billboard => f.write_str("Billboard"),
			InterpretAs::Cubemap => f.write_str("Cubemap"),
			InterpretAs::Emission => f.write_str("Emission"),
			InterpretAs::Volume => f.write_str("Volume")
		}
	}
}

impl From<InterpretAs> for glacier_texture::enums::InterpretAs {
	fn from(value: InterpretAs) -> Self {
		match value {
			InterpretAs::Colour => Self::Colour,
			InterpretAs::Normal => Self::Normal,
			InterpretAs::Height => Self::Height,
			InterpretAs::CompoundNormal => Self::CompoundNormal,
			InterpretAs::Billboard => Self::Billboard,
			InterpretAs::Cubemap => Self::Cubemap,
			InterpretAs::Emission => Self::Emission,
			InterpretAs::Volume => Self::Volume
		}
	}
}

impl From<glacier_texture::enums::InterpretAs> for InterpretAs {
	fn from(value: glacier_texture::enums::InterpretAs) -> Self {
		match value {
			glacier_texture::enums::InterpretAs::Colour => Self::Colour,
			glacier_texture::enums::InterpretAs::Normal => Self::Normal,
			glacier_texture::enums::InterpretAs::Height => Self::Height,
			glacier_texture::enums::InterpretAs::CompoundNormal => Self::CompoundNormal,
			glacier_texture::enums::InterpretAs::Billboard => Self::Billboard,
			glacier_texture::enums::InterpretAs::Cubemap => Self::Cubemap,
			glacier_texture::enums::InterpretAs::Emission => Self::Emission,
			glacier_texture::enums::InterpretAs::Volume => Self::Volume
		}
	}
}
