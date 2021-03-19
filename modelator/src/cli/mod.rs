// CLI output.
mod output;

// Re-exports.
pub use output::{CliOutput, CliStatus};

use crate::artifact::{JsonTrace, TlaConfigFile, TlaFile, TlaTrace};
use crate::Error;
use clap::{AppSettings, Clap, Subcommand};
use serde_json::{json, Value as JsonValue};
use std::path::Path;

#[derive(Clap, Debug)]
#[clap(name = "modelator")]
#[clap(setting = AppSettings::DisableHelpSubcommand)]
pub struct CliOptions {
    #[clap(subcommand)]
    subcommand: Modules,
}

#[derive(Debug, Subcommand)]
enum Modules {
    /// Generate TLA+ test cases and parse TLA+ traces.
    Tla(TlaMethods),
    /// Generate TLA+ traces using Apalache.
    Apalache(ApalacheMethods),
    /// Generate TLA+ traces using TLC.
    Tlc(TlcMethods),
}

#[derive(Debug, Clap)]
#[clap(setting = AppSettings::DisableHelpSubcommand)]
pub enum TlaMethods {
    /// Generate TLA+ tests.
    GenerateTests {
        /// TLA+ file with test cases.
        tla_file: String,
        /// TLA+ config file with CONSTANTS, INIT and NEXT.
        tla_config_file: String,
    },
    /// Convert a TLA+ trace to a JSON trace.
    TlaTraceToJsonTrace {
        /// File with a TLA+ trace produced by the Apalache or TLC modules.
        tla_trace_file: String,
    },
}

#[derive(Debug, Clap)]
#[clap(setting = AppSettings::DisableHelpSubcommand)]
pub enum ApalacheMethods {
    /// Generate TLA+ trace from a TLA+ test.
    Test {
        /// TLA+ file generated by the generate-test method in the TLA module.
        tla_file: String,
        /// TLA+ config file generated by the generate-test method in the TLA module.
        tla_config_file: String,
    },
}

#[derive(Debug, Clap)]
#[clap(setting = AppSettings::DisableHelpSubcommand)]
pub enum TlcMethods {
    /// Generate TLA+ trace from a TLA+ test.
    Test {
        /// TLA+ file generated by the generate-test method in the TLA module.
        tla_file: String,
        /// TLA+ config file generated by the generate-test method in the TLA module.
        tla_config_file: String,
    },
}

impl CliOptions {
    pub fn run(self) -> CliOutput {
        let result = self.subcommand.run();
        CliOutput::with_result(result)
    }
}

impl Modules {
    fn run(self) -> Result<JsonValue, Error> {
        // setup modelator
        let options = crate::Options::default();
        crate::setup(&options)?;

        // run the subcommand
        match self {
            Self::Tla(options) => options.run(),
            Self::Apalache(options) => options.run(),
            Self::Tlc(options) => options.run(),
        }
    }
}

impl TlaMethods {
    fn run(self) -> Result<JsonValue, Error> {
        match self {
            Self::GenerateTests {
                tla_file,
                tla_config_file,
            } => Self::generate_tests(tla_file, tla_config_file),
            Self::TlaTraceToJsonTrace { tla_trace_file } => {
                Self::tla_trace_to_json_trace(tla_trace_file)
            }
        }
    }

    fn generate_tests(tla_file: String, tla_config_file: String) -> Result<JsonValue, Error> {
        let tests = crate::module::Tla::generate_tests(tla_file.into(), tla_config_file.into())?;
        tracing::debug!("Tla::generate_tests output {:#?}", tests);

        generated_tests(tests)
    }

    fn tla_trace_to_json_trace(tla_trace_file: String) -> Result<JsonValue, Error> {
        // parse tla trace
        let tla_trace_file = Path::new(&tla_trace_file);
        if !tla_trace_file.is_file() {
            return Err(Error::FileNotFound(tla_trace_file.to_path_buf()));
        }
        let tla_trace = std::fs::read_to_string(&tla_trace_file).map_err(Error::io)?;
        let tla_trace = TlaTrace::parse(tla_trace)?;

        let json_trace = crate::module::Tla::tla_trace_to_json_trace(tla_trace)?;
        tracing::debug!("Tla::tla_trace_to_json_trace output {}", json_trace);

        save_json_trace(json_trace)
    }
}

impl ApalacheMethods {
    fn run(self) -> Result<JsonValue, Error> {
        match self {
            Self::Test {
                tla_file,
                tla_config_file,
            } => Self::test(tla_file, tla_config_file),
        }
    }

    fn test(tla_file: String, tla_config_file: String) -> Result<JsonValue, Error> {
        let options = crate::Options::default();
        let tla_trace =
            crate::module::Apalache::test(tla_file.into(), tla_config_file.into(), &options)?;
        tracing::debug!("Apalache::test output {}", tla_trace);

        save_tla_trace(tla_trace)
    }
}

impl TlcMethods {
    fn run(self) -> Result<JsonValue, Error> {
        match self {
            Self::Test {
                tla_file,
                tla_config_file,
            } => Self::test(tla_file, tla_config_file),
        }
    }

    fn test(tla_file: String, tla_config_file: String) -> Result<JsonValue, Error> {
        let options = crate::Options::default();
        let tla_trace =
            crate::module::Tlc::test(tla_file.into(), tla_config_file.into(), &options)?;
        tracing::debug!("Tlc::test output {}", tla_trace);

        save_tla_trace(tla_trace)
    }
}

#[allow(clippy::unnecessary_wraps)]
fn generated_tests(tests: Vec<(TlaFile, TlaConfigFile)>) -> Result<JsonValue, Error> {
    let json_array_entry = |tla_file: TlaFile, tla_config_file: TlaConfigFile| {
        json!({
            "tla_file": format!("{}", tla_file),
            "tla_config_file": format!("{}", tla_config_file),
        })
    };
    let json_array = tests
        .into_iter()
        .map(|(tla_file, tla_config_file)| json_array_entry(tla_file, tla_config_file))
        .collect();
    Ok(JsonValue::Array(json_array))
}

fn save_tla_trace(tla_trace: TlaTrace) -> Result<JsonValue, Error> {
    let path = Path::new("trace.tla").to_path_buf();
    std::fs::write(&path, format!("{}", tla_trace)).map_err(Error::io)?;
    Ok(json!({
        "tla_trace_file": crate::util::absolute_path(&path),
    }))
}

fn save_json_trace(json_trace: JsonTrace) -> Result<JsonValue, Error> {
    let path = Path::new("trace.json").to_path_buf();
    std::fs::write(&path, format!("{}", json_trace)).map_err(Error::io)?;
    Ok(json!({
        "json_trace_file": crate::util::absolute_path(&path),
    }))
}
