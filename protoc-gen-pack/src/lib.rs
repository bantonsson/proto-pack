use std::{
    collections::{BTreeMap, HashSet},
    fmt, str,
};

use anyhow::Result;
use heck::ToSnakeCase;
use log::debug;
use prost::Message;
use prost_types::{compiler::CodeGeneratorRequest, FileDescriptorProto};
mod args;

/// execute the file generator from an encoded [`CodeGeneratorRequest`]
pub fn execute(program_name: String, raw_request: &[u8]) -> Result<()> {
    debug!("DCD ->");
    let request = CodeGeneratorRequest::decode(raw_request)?;
    let param_string = request.parameter().to_string();
    debug!("PRM -> {:?}", param_string);
    let param_name = vec![program_name];
    let params = param_name
        .iter()
        .map(String::as_str)
        .chain(param_string.split(','))
        .flat_map(|a| a.split(' '))
        .map(|a| a.trim());
    let args = args::try_parse_from(params)?;

    debug!("MRS ->");
    let module_request_set = ModuleRequestSet::new(
        request.file_to_generate,
        request.proto_file,
        args.default_package_name.as_deref(),
    )?;
    debug!("MRS <-");

    module_request_set.requests().for_each(|(m, mr)| {
        debug!(
            "TFW {} -> {} : {}",
            m.to_string(),
            mr.proto_package_name(),
            mr.files().fold(String::new(), |s, f| {
                s.to_owned()
                    + " "
                    + f.name()
                    + " <- "
                    + f.options
                        .iter()
                        .fold(String::new(), |s, o| {
                            s.to_owned()
                                + "'"
                                + o.java_generate_equals_and_hash().to_string().as_str()
                                + "'"
                        })
                        .as_str()
            }),
        )
    });

    Ok(())
}

/// A set of requests to generate code for a series of modules
pub struct ModuleRequestSet {
    requests: BTreeMap<Module, ModuleRequest>,
}

impl ModuleRequestSet {
    /// Construct a new module request set from an encoded [`CodeGeneratorRequest`]
    ///
    /// [`CodeGeneratorRequest`]: prost_types::compiler::CodeGeneratorRequest
    pub fn new<I>(
        input_protos: I,
        proto_file: Vec<FileDescriptorProto>,
        default_package_filename: Option<&str>,
    ) -> std::result::Result<Self, prost::DecodeError>
    where
        I: IntoIterator<Item = String>,
    {
        Ok(Self::new_decoded(
            input_protos,
            proto_file,
            default_package_filename.unwrap_or("_"),
        ))
    }

    fn new_decoded<I>(
        input_protos: I,
        proto_file: Vec<FileDescriptorProto>,
        default_package_filename: &str,
    ) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        debug!("IPP ->");
        let input_protos: HashSet<_> = input_protos.into_iter().collect();
        debug!("IPP <-");

        let requests = proto_file
            .into_iter()
            .fold(BTreeMap::new(), |mut acc, proto| {
                debug!(
                    "PII -> {}",
                    proto.name.clone().unwrap_or_else(|| String::from("?"))
                );
                let module = Module::from_protobuf_package_name(proto.package());
                debug!("PII -> 1");
                let proto_filename = proto.name();
                debug!("PII -> 2");
                let entry = acc
                    .entry(module)
                    .or_insert_with(|| ModuleRequest::new(proto.package().to_owned()));
                debug!("PII -> 3");

                if entry.output_filename().is_none() && input_protos.contains(proto_filename) {
                    let filename = match proto.package() {
                        "" => default_package_filename.to_owned(),
                        package => format!("{package}.java"),
                    };
                    entry.with_output_filename(filename);
                }
                debug!("PII -> 4");

                entry.push_file_descriptor_proto(proto);
                debug!("PII -> 5");
                acc
            });

        Self { requests }
    }

    /// An ordered iterator of all requests
    pub fn requests(&self) -> impl Iterator<Item = (&Module, &ModuleRequest)> {
        self.requests.iter()
    }

    /// Retrieve the request for the given module
    pub fn for_module(&self, module: &Module) -> Option<&ModuleRequest> {
        self.requests.get(module)
    }
}

/// A code generation request for a specific module
pub struct ModuleRequest {
    proto_package_name: String,
    output_filename: Option<String>,
    files: Vec<FileDescriptorProto>,
}

impl ModuleRequest {
    fn new(proto_package_name: String) -> Self {
        Self {
            proto_package_name,
            output_filename: None,
            files: Vec::new(),
        }
    }

    fn with_output_filename(&mut self, filename: String) {
        self.output_filename = Some(filename);
    }

    fn push_file_descriptor_proto(&mut self, encoded: FileDescriptorProto) {
        self.files.push(encoded);
    }

    /// The protobuf package name for this module
    pub fn proto_package_name(&self) -> &str {
        &self.proto_package_name
    }

    /// The output filename for this module
    pub fn output_filename(&self) -> Option<&str> {
        self.output_filename.as_deref()
    }

    /// An iterator of the file descriptors
    pub fn files(&self) -> impl Iterator<Item = &FileDescriptorProto> {
        self.files.iter()
    }
}

/// A Rust module path for a Protobuf package.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Module {
    components: Vec<String>,
}

impl Module {
    /// Construct a module path from an iterator of parts.
    pub fn from_parts<I>(parts: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            components: parts.into_iter().map(|s| s.into()).collect(),
        }
    }

    /// Construct a module path from a Protobuf package name.
    ///
    /// Constituent parts are automatically converted to snake case in order to follow
    /// Rust module naming conventions.
    pub fn from_protobuf_package_name(name: &str) -> Self {
        Self {
            components: name
                .split('.')
                .filter(|s| !s.is_empty())
                .map(ToSnakeCase::to_snake_case)
                .collect(),
        }
    }

    /// An iterator over the parts of the path.
    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.components.iter().map(|s| s.as_str())
    }

    /// The number of parts in the module's path.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Whether the module's path contains any components.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut parts = self.parts();
        if let Some(first) = parts.next() {
            f.write_str(first)?;
        }
        for part in parts {
            f.write_str("::")?;
            f.write_str(part)?;
        }
        Ok(())
    }
}
