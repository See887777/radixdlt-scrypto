use clap::Parser;
use std::fs;
use std::path::PathBuf;

use crate::scrypto::*;

/// Create a Scrypto package
#[derive(Parser, Debug)]
pub struct NewPackage {
    /// The package name
    package_name: String,

    /// The package directory
    #[clap(long)]
    path: Option<PathBuf>,

    /// Use local Scrypto as dependency
    #[clap(short, long)]
    local: bool,
}

impl NewPackage {
    pub fn run(&self) -> Result<(), String> {
        let wasm_name = self.package_name.replace("-", "_");
        let path = self
            .path
            .clone()
            .unwrap_or(PathBuf::from(&self.package_name));
        let simulator_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (sbor, scrypto, scrypto_test) = if self.local {
            let scrypto_dir = simulator_dir
                .parent()
                .unwrap()
                .to_string_lossy()
                .replace("\\", "/");
            (
                format!("{{ path = \"{}/sbor\" }}", scrypto_dir),
                format!("{{ path = \"{}/scrypto\" }}", scrypto_dir),
                format!("{{ path = \"{}/scrypto-test\" }}", scrypto_dir),
            )
        } else {
            let s = format!("{{ version = \"{}\" }}", env!("CARGO_PKG_VERSION"));
            (s.clone(), s.clone(), s.clone())
        };

        if path.exists() {
            Err(Error::PackageAlreadyExists.into())
        } else {
            fs::create_dir_all(child_of(&path, "src")).map_err(Error::IOError)?;
            fs::create_dir_all(child_of(&path, "tests")).map_err(Error::IOError)?;

            fs::write(
                child_of(&path, "Cargo.toml"),
                include_str!("../../assets/template/Cargo.toml_template")
                    .replace("${package_name}", &self.package_name)
                    .replace("${sbor}", &sbor)
                    .replace("${scrypto}", &scrypto)
                    .replace("${scrypto-test}", &scrypto_test),
            )
            .map_err(Error::IOError)?;

            fs::write(
                child_of(&path, ".gitignore"),
                include_str!("../../assets/template/.gitignore"),
            )
            .map_err(Error::IOError)?;

            fs::write(
                child_of(&child_of(&path, "src"), "lib.rs"),
                include_str!("../../assets/template/src/lib.rs"),
            )
            .map_err(Error::IOError)?;

            fs::write(
                child_of(&child_of(&path, "tests"), "lib.rs"),
                include_str!("../../assets/template/tests/lib.rs")
                    .replace("${wasm_name}", &wasm_name),
            )
            .map_err(Error::IOError)?;

            Ok(())
        }
    }
}

fn child_of(path: &PathBuf, name: &str) -> PathBuf {
    let mut p = path.clone();
    p.push(name);
    p
}
