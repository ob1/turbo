use std::collections::BTreeMap;

use anyhow::Result;
use biome_deserialize::json::deserialize_from_json_str;
use biome_deserialize_macros::Deserializable;
use biome_diagnostics::DiagnosticExt;
use biome_json_parser::JsonParserOptions;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use turbopath::{AbsoluteSystemPath, RelativeUnixPathBuf};
use turborepo_errors::ParseDiagnostic;

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq, Deserializable)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "turbo", default, skip_serializing_if = "Option::is_none")]
    pub legacy_turbo_config: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scripts: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnpm: Option<PnpmConfig>,
    // Unstructured fields kept for round trip capabilities
    //#[serde(flatten)]
    //pub other: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Deserializable)]
#[serde(rename_all = "camelCase")]
pub struct PnpmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patched_dependencies: Option<BTreeMap<String, RelativeUnixPathBuf>>,
    // Unstructured config options kept for round trip capabilities
    //#[serde(flatten)]
    //pub other: BTreeMap<String, Value>,
}

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum Error {
    #[error("unable to read package.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("unable to parse package.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unable to parse package.json")]
    #[diagnostic(code(package_json_parse_error))]
    Parse(#[related] Vec<ParseDiagnostic>),
}

impl PackageJson {
    pub fn load(path: &AbsoluteSystemPath) -> Result<PackageJson, Error> {
        tracing::debug!("loading package.json from {}", path);
        let contents = path.read_to_string()?;
        Self::load_from_str(&contents, path.as_str())
    }

    pub fn load_from_str(contents: &str, path: &str) -> Result<PackageJson, Error> {
        let (result, errors) =
            deserialize_from_json_str(contents, JsonParserOptions::default(), path).consume();

        match result {
            Some(package_json) => Ok(package_json),
            None => Err(Error::Parse(
                errors
                    .into_iter()
                    .map(|d| {
                        d.with_file_source_code(contents)
                            .with_file_path(path)
                            .into()
                    })
                    .collect(),
            )),
        }
    }

    // Utility method for easy construction of package.json during testing
    pub fn from_value(value: serde_json::Value) -> Result<PackageJson, Error> {
        let contents = serde_json::to_string(&value)?;
        let package_json: PackageJson = Self::load_from_str(&contents, "package.json")?;
        Ok(package_json)
    }

    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &String)> + '_ {
        self.dev_dependencies
            .iter()
            .flatten()
            .chain(self.optional_dependencies.iter().flatten())
            .chain(self.dependencies.iter().flatten())
    }

    /// Returns the command for script_name if it is non-empty
    pub fn command(&self, script_name: &str) -> Option<&str> {
        self.scripts
            .get(script_name)
            .filter(|command| !command.is_empty())
            .map(|command| command.as_str())
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use test_case::test_case;

    use super::*;

    #[test_case(json!({"name": "foo", "random-field": true}) ; "additional fields kept during round trip")]
    #[test_case(json!({"name": "foo", "resolutions": {"foo": "1.0.0"}}) ; "berry resolutions")]
    #[test_case(json!({"name": "foo", "pnpm": {"patchedDependencies": {"some-pkg": "./patchfile"}, "another-field": 1}}) ; "pnpm")]
    #[test_case(json!({"name": "foo", "pnpm": {"another-field": 1}}) ; "pnpm without patches")]
    fn test_roundtrip(json: serde_json::Value) {
        let package_json: PackageJson = PackageJson::from_value(json.clone()).unwrap();
        let actual = serde_json::to_value(package_json).unwrap();
        assert_eq!(actual, json);
    }

    #[test]
    fn test_legacy_turbo_config() -> Result<()> {
        let contents = r#"{"turbo": {}}"#;
        let package_json = PackageJson::load_from_str(contents, "package.json")?;

        assert!(package_json.legacy_turbo_config.is_some());

        let contents = r#"{"turbo": { "globalDependencies": [".env"] } }"#;
        let package_json = PackageJson::load_from_str(contents, "package.json")?;

        assert!(package_json.legacy_turbo_config.is_some());

        Ok(())
    }
}
