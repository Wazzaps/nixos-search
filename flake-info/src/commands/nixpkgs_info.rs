use anyhow::{Context, Result};
use serde_json::Deserializer;
use std::{collections::HashMap, fmt::Display};

use command_run::{Command, LogTo};
use log::error;

use crate::data::import::{NixOption, NixpkgsEntry, Package};

pub fn get_nixpkgs_info<T: AsRef<str> + Display>(nixpkgs_channel: T) -> Result<Vec<NixpkgsEntry>> {
    let mut command = Command::new("nix-env");
    command.add_args(&[
        "--json",
        "-f",
        "<nixpkgs>",
        "-I",
        format!("nixpkgs={}", nixpkgs_channel.as_ref()).as_str(),
        "--arg",
        "config",
        "import <nixpkgs/pkgs/top-level/packages-config.nix>",
        "-qa",
        "--meta",
    ]);

    command.enable_capture();
    command.log_to = LogTo::Log;
    command.log_output_on_error = true;

    let parsed: Result<Vec<NixpkgsEntry>> = command
        .run()
        .with_context(|| {
            format!(
                "Failed to gather information about nixpkgs {}",
                nixpkgs_channel.as_ref()
            )
        })
        .and_then(|o| {
            let output = &*o.stdout_string_lossy();
            let de = &mut Deserializer::from_str(output);
            let attr_set: HashMap<String, Package> =
                serde_path_to_error::deserialize(de).with_context(|| "Could not parse packages")?;
            Ok(attr_set
                .into_iter()
                .map(|(attribute, package)| NixpkgsEntry::Derivation { attribute, package })
                .collect())
        });

    parsed
}

pub fn get_nixpkgs_options<T: AsRef<str> + Display>(
    nixpkgs_channel: T,
) -> Result<Vec<NixpkgsEntry>> {
    let mut command = Command::with_args("nix", &["eval", "--json"]);
    command.add_arg_pair("-f", super::EXTRACT_SCRIPT.clone());
    command.add_arg_pair("-I", format!("nixpkgs={}", nixpkgs_channel.as_ref()));
    command.add_arg("nixos-options");

    command.enable_capture();
    command.log_to = LogTo::Log;
    command.log_output_on_error = true;

    let parsed = command.run().with_context(|| {
        format!(
            "Failed to gather information about nixpkgs {}",
            nixpkgs_channel.as_ref()
        )
    });

    if let Err(ref e) = parsed {
        error!("Command error: {}", e);
    }

    parsed.and_then(|o| {
        let output = &*o.stdout_string_lossy();
        let de = &mut Deserializer::from_str(output);
        let attr_set: Vec<NixOption> =
            serde_path_to_error::deserialize(de).with_context(|| "Could not parse options")?;
        Ok(attr_set.into_iter().map(NixpkgsEntry::Option).collect())
    })
}
