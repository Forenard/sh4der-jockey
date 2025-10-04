use vergen::EmitBuilder;
use std::env;
use std::fs;
use std::path::Path;

pub fn main() -> anyhow::Result<()> {
    EmitBuilder::builder()
        .all_build()
        //.all_cargo()
        .all_git()
        //.all_rustc()
        //.all_sysinfo()
        .emit()?;

    // Copy SpoutLibrary.dll from libs/ to target directory if it exists
    let source_dll = Path::new("libs/SpoutLibrary.dll");
    if source_dll.exists() {
        let out_dir = env::var("OUT_DIR")?;
        let target_dir = Path::new(&out_dir)
            .ancestors()
            .nth(3) // Navigate up from OUT_DIR to target/debug or target/release
            .ok_or_else(|| anyhow::anyhow!("Could not find target directory"))?;

        let dest_dll = target_dir.join("SpoutLibrary.dll");

        // Only copy if source is newer or dest doesn't exist
        let should_copy = if dest_dll.exists() {
            let src_metadata = fs::metadata(source_dll)?;
            let dst_metadata = fs::metadata(&dest_dll)?;
            src_metadata.modified()? > dst_metadata.modified()?
        } else {
            true
        };

        if should_copy {
            fs::copy(source_dll, &dest_dll)?;
            println!("cargo:warning=Copied SpoutLibrary.dll to {:?}", dest_dll);
        }
    }

    Ok(())
}
