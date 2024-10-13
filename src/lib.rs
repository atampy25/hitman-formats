#[cfg(feature = "material")]
pub mod material;

#[cfg(feature = "ores")]
pub mod ores;

#[cfg(feature = "wwev")]
pub mod wwev;

#[cfg(feature = "rune")]
pub fn rune_install(ctx: &mut rune::Context) -> Result<(), rune::ContextError> {
	#[cfg(feature = "material")]
	ctx.install(material::rune_module()?)?;

	#[cfg(feature = "ores")]
	ctx.install(ores::rune_module()?)?;

	#[cfg(feature = "wwev")]
	ctx.install(wwev::rune_module()?)?;

	Ok(())
}
