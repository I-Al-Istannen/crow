use crate::context::{Test, TestDetail};
use indexmap::IndexMap;
use markdown::mdast::{Code, Text};
use markdown::{ParseOptions, mdast};
use mdast::{Heading, Node, Root};
use shared::{CompilerFailReason, CrashSignal, TestModifier};
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu, ensure, location};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum FormatError {
    #[snafu(display("Could not read file `{}` at {location}", path.display()))]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Found duplicate heading `{heading}` at {location}"))]
    DuplicateHeading {
        heading: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not parse test modifier `{message}` at {location}"))]
    MalformedModifier {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing section `{section}` at {location}"))]
    MissingSection {
        section: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing value for key `{key}` at {location}"))]
    MissingValue {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Keys {
    CompilerModifiers,
    BinaryModifiers,
    Meta,
    Hash,
    Creator,
    AdminAuthored,
    LimitedToCategory,
}

impl Display for Keys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerModifiers => write!(f, "Executing your compiler"),
            Self::BinaryModifiers => write!(f, "Executing the compiled binary"),
            Self::Meta => write!(f, "Meta"),
            Self::Hash => write!(f, "Hash"),
            Self::Creator => write!(f, "Creator"),
            Self::AdminAuthored => write!(f, "Admin Authored"),
            Self::LimitedToCategory => write!(f, "Limited to Category"),
        }
    }
}

pub fn from_markdown(
    path: &Path,
    category: String,
    id: String,
) -> Result<(Test, TestDetail), FormatError> {
    let file = std::fs::read_to_string(path).context(FileReadSnafu {
        path: path.to_path_buf(),
    })?;
    let file = markdown::to_mdast(&file, &ParseOptions::default()).unwrap();
    let nodes_to_process = file.children().unwrap_or(&Vec::new()).clone();

    let mut nodes = associate_to_headings(nodes_to_process)?;

    let meta = extract_key_values(extract_heading(Keys::Meta, &mut nodes)?, |_| true)?;
    let mut meta = IndexMap::from_iter(meta);
    let hash = extract_value(Keys::Hash, &mut meta)?;
    let creator_id = extract_value(Keys::Creator, &mut meta)?;
    let admin_authored = extract_value(Keys::AdminAuthored, &mut meta)?
        .parse::<bool>()
        .map_err(|e| {
            MalformedModifierSnafu {
                message: format!("Could not parse admin authored: {e}"),
            }
            .into_error(NoneError)
        })?;
    let limited_to_category = extract_value(Keys::LimitedToCategory, &mut meta)
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .map_err(|e| {
            MalformedModifierSnafu {
                message: format!("Could not parse limited to category: {e}"),
            }
            .into_error(NoneError)
        })?;

    let test = Test {
        id,
        creator_id,
        hash,
        category,
        admin_authored,
        limited_to_category,
    };
    let test_detail = details_from_markdown(path)?;

    Ok((test, test_detail))
}

pub fn details_from_markdown(path: &Path) -> Result<TestDetail, FormatError> {
    let file = std::fs::read_to_string(path).context(FileReadSnafu {
        path: path.to_path_buf(),
    })?;
    let file = markdown::to_mdast(&file, &ParseOptions::default()).unwrap();
    let nodes_to_process = file.children().unwrap_or(&Vec::new()).clone();

    let mut nodes = associate_to_headings(nodes_to_process)?;

    let compiler_modifiers =
        extract_modifiers(extract_heading(Keys::CompilerModifiers, &mut nodes)?)?;
    let binary_modifiers = extract_modifiers(extract_heading(Keys::BinaryModifiers, &mut nodes)?)?;

    Ok(TestDetail {
        compiler_modifiers,
        binary_modifiers,
    })
}

fn associate_to_headings(nodes: Vec<Node>) -> Result<IndexMap<String, Vec<Node>>, FormatError> {
    let mut result = IndexMap::new();
    let mut current_batch = Vec::new();
    let mut current_heading = None;

    for node in nodes {
        if let Node::Heading(Heading {
            depth: 1, children, ..
        }) = &node
        {
            if let Some(current_heading) = current_heading {
                result.insert(current_heading, current_batch);
            }

            let header = children.iter().map(|it| it.to_string()).collect::<String>();
            let header = header.trim();

            ensure!(
                !result.contains_key(header),
                DuplicateHeadingSnafu {
                    heading: header.to_string()
                }
            );
            current_batch = vec![];
            current_heading = Some(header.to_string());
        }

        if let Node::Code(Code { .. }) = &node {
            current_batch.push(node);
        } else if let Node::Heading(Heading { depth: 2, .. }) = &node {
            current_batch.push(node);
        }
    }

    if let Some(current_heading) = current_heading {
        result.insert(current_heading, current_batch);
    }

    Ok(result)
}

fn extract_heading(
    name: Keys,
    nodes: &mut IndexMap<String, Vec<Node>>,
) -> Result<Vec<Node>, FormatError> {
    match nodes.shift_remove(&name.to_string()) {
        Some(nodes) => Ok(nodes),
        None => Err(FormatError::MissingSection {
            section: name.to_string(),
            location: location!(),
        }),
    }
}

fn extract_modifiers(nodes: Vec<Node>) -> Result<Vec<TestModifier>, FormatError> {
    let mut result = vec![];

    for (name, val) in extract_key_values(nodes, modifier_requires_argument)? {
        result.push(modifier_from_string(&name, val)?);
    }

    Ok(result)
}

fn extract_key_values(
    mut nodes: Vec<Node>,
    needs_value: impl Fn(&str) -> bool,
) -> Result<Vec<(String, Option<String>)>, FormatError> {
    let mut result: Vec<(String, Option<String>)> = Vec::new();

    while !nodes.is_empty() {
        let node = nodes.remove(0);
        if let Node::Heading(Heading {
            depth: 2, children, ..
        }) = node
        {
            let header = children.iter().map(|it| it.to_string()).collect::<String>();
            let header = header.trim().to_string();

            if !needs_value(&header) {
                result.push((header, None));
                continue;
            }

            while !nodes.is_empty() {
                let node = nodes.remove(0);
                if let Node::Code(Code { value, .. }) = node {
                    result.push((header, Some(value)));
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn extract_value(
    name: Keys,
    values: &mut IndexMap<String, Option<String>>,
) -> Result<String, FormatError> {
    match values.shift_remove(&name.to_string()).flatten() {
        Some(value) => Ok(value),
        None => Err(FormatError::MissingValue {
            key: name.to_string(),
            location: location!(),
        }),
    }
}

pub fn to_markdown(test: &Test, detail: &TestDetail) -> String {
    let mut root = Root {
        children: vec![],
        position: None,
    };

    root.children.extend(modifiers_to_markdown(
        Keys::CompilerModifiers.to_string(),
        &detail.compiler_modifiers,
    ));
    root.children.extend(modifiers_to_markdown(
        Keys::BinaryModifiers.to_string(),
        &detail.binary_modifiers,
    ));

    root.children
        .extend(write_heading_value(&Keys::Meta.to_string(), 1, None));

    root.children.extend(write_heading_value(
        &Keys::LimitedToCategory.to_string(),
        2,
        Some(test.limited_to_category.to_string()),
    ));

    root.children.extend(write_heading_value(
        &Keys::Creator.to_string(),
        2,
        Some(test.creator_id.clone()),
    ));

    root.children.extend(write_heading_value(
        &Keys::AdminAuthored.to_string(),
        2,
        Some(test.admin_authored.to_string()),
    ));

    root.children.extend(write_heading_value(
        &Keys::Hash.to_string(),
        2,
        Some(test.hash.clone()),
    ));

    mdast_util_to_markdown::to_markdown(&Node::Root(root)).expect("Could convert to markdown")
}

fn modifiers_to_markdown(heading: String, modifiers: &[TestModifier]) -> Vec<Node> {
    let mut result = write_heading_value(&heading, 1, None);

    result.extend(modifiers.iter().flat_map(modifier_to_markdown));

    result
}

fn modifier_to_markdown(modifier: &TestModifier) -> Vec<Node> {
    write_heading_value(modifier.name(), 2, modifier_arg_to_string(modifier))
}

fn modifier_arg_to_string(modifier: &TestModifier) -> Option<String> {
    match modifier {
        TestModifier::ExitCode { code } => Some(code.to_string()),
        TestModifier::ExpectedOutput { output } => Some(output.to_string()),
        TestModifier::ProgramArgument { arg } => Some(arg.to_string()),
        TestModifier::ProgramArgumentFile { contents } => Some(contents.to_string()),
        TestModifier::ProgramInput { input } => Some(input.to_string()),
        TestModifier::ShouldCrash { signal } => Some(signal.to_string()),
        TestModifier::ShouldFail { reason } => Some(reason.to_string()),
        TestModifier::ShouldSucceed => None,
        TestModifier::ShouldTimeout => None,
    }
}

fn write_heading_value(heading: &str, depth: u8, value: Option<String>) -> Vec<Node> {
    let mut res = vec![Node::Heading(Heading {
        depth,
        children: vec![Node::Text(Text {
            value: heading.to_string(),
            position: None,
        })],
        position: None,
    })];

    if let Some(value) = value {
        res.push(Node::Code(Code {
            value,
            lang: None,
            meta: None,
            position: None,
        }));
    }

    res
}

fn modifier_from_string(type_: &str, value: Option<String>) -> Result<TestModifier, FormatError> {
    let res = match type_ {
        "ExitCode" => {
            let value = require_value("ExitCode", value)?;
            TestModifier::ExitCode {
                code: value.parse::<u8>().map_err(|e| {
                    MalformedModifierSnafu {
                        message: format!("Could not parse exit code: {e}"),
                    }
                    .into_error(NoneError)
                })?,
            }
        }
        "ExpectedOutput" => TestModifier::ExpectedOutput {
            output: require_value("ExpectedOutput", value)?,
        },
        "ProgramArgument" => TestModifier::ProgramArgument {
            arg: require_value("ProgramArgument", value)?,
        },
        "ProgramArgumentFile" => TestModifier::ProgramArgumentFile {
            contents: require_value("ProgramArgumentFile", value)?,
        },
        "ProgramInput" => TestModifier::ProgramInput {
            input: require_value("ProgramInput", value)?,
        },
        "ShouldCrash" => TestModifier::ShouldCrash {
            signal: parse_crash_signal(&require_value("ShouldCrash", value)?)?,
        },
        "ShouldFail" => TestModifier::ShouldFail {
            reason: parse_fail_reason(&require_value("ShouldFail", value)?)?,
        },
        "ShouldSucceed" => TestModifier::ShouldSucceed,
        "ShouldTimeout" => TestModifier::ShouldTimeout,
        _ => {
            return Err(FormatError::MalformedModifier {
                message: format!("Unknown modifier type `{type_}`"),
                location: location!(),
            });
        }
    };

    Ok(res)
}

fn require_value(name: &str, maybe_value: Option<String>) -> Result<String, FormatError> {
    maybe_value
        .ok_or_else(|| {
            MalformedModifierSnafu {
                message: format!("Modifier `{name}` requires a value"),
            }
            .into_error(NoneError)
        })
        .map(|v| v.to_string())
}

fn parse_crash_signal(val: &str) -> Result<CrashSignal, FormatError> {
    match val {
        "Abort" => Ok(CrashSignal::Abort),
        "SegmentationFault" => Ok(CrashSignal::SegmentationFault),
        "FloatingPointException" => Ok(CrashSignal::FloatingPointException),
        other => Err(FormatError::MalformedModifier {
            message: format!(
                "Unknown crash signal `{other}`. \
                Valid are `Abort`, `SegmentationFault`, and `FloatingPointException`"
            ),
            location: location!(),
        }),
    }
}

fn parse_fail_reason(val: &str) -> Result<CompilerFailReason, FormatError> {
    match val {
        "Parsing" => Ok(CompilerFailReason::Parsing),
        "SemanticAnalysis" => Ok(CompilerFailReason::SemanticAnalysis),
        other => Err(FormatError::MalformedModifier {
            message: format!(
                "Unknown fail reason `{other}`. Valid are `Parsing` and `SemanticAnalysis`"
            ),
            location: location!(),
        }),
    }
}

fn modifier_requires_argument(modifier: &str) -> bool {
    modifier != "ShouldSucceed" && modifier != "ShouldTimeout"
}
