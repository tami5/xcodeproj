#![allow(missing_docs)]
use super::object::PBXObjectKind;
use super::{PBXArray, PBXHashMap, PBXProjectData};
use crate::pbxproj::PBXValue;
use anyhow::Context;
use convert_case::{Case, Casing};
use itertools::Itertools;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, num::ParseIntError};

use pest_consume::*;
use tap::Pipe;

/// Pest Parser to parse into [`XProj`]
#[derive(Parser)]
#[grammar = "parser/pbxproj/grammar.pest"]
pub(crate) struct PBXProjectParser;

pub(crate) type NodeResult<T> = std::result::Result<T, Error<Rule>>;
pub(crate) type Node<'i> = pest_consume::Node<'i, Rule, ()>;

#[parser]
impl PBXProjectParser {
    fn key(input: Node) -> NodeResult<String> {
        let inner = input.into_children().next().unwrap();

        if inner.as_rule() == Rule::ident {
            Ok(inner.as_str().to_case(Case::Snake))
        } else {
            Ok(inner.as_str().to_string())
        }
    }

    fn string(input: Node) -> NodeResult<PBXValue> {
        let value = input.as_str().replace("\"", "");
        // println!("string value: `{value}`");
        value.pipe(PBXValue::String).pipe(Ok)
    }

    fn kind(input: Node) -> NodeResult<PBXValue> {
        let value = PBXObjectKind::from(input.as_str());
        value.pipe(PBXValue::Kind).pipe(Ok)
    }

    fn ident(input: Node) -> NodeResult<PBXValue> {
        input.as_str().to_string().pipe(PBXValue::String).pipe(Ok)
    }

    fn uuid(input: Node) -> NodeResult<PBXValue> {
        input.as_str().to_string().pipe(PBXValue::String).pipe(Ok)
    }

    fn number(input: Node) -> NodeResult<PBXValue> {
        // TODO: identify versions as string instead of number or as ident!
        let value = input.as_str();
        if value.contains(".") {
            return Ok(PBXValue::String(value.into()));
        }
        value
            .parse()
            .map_err(|e: ParseIntError| input.error(e))
            .map(PBXValue::Number)
    }

    fn bool(input: Node) -> NodeResult<PBXValue> {
        match input.as_str() {
            "YES" => Ok(true),
            "NO" => Ok(false),
            value => input
                .error(format!("{value:?} is not parseable as boolean!"))
                .pipe(Err),
        }?
        .pipe(PBXValue::Bool)
        .pipe(Ok)
    }

    fn array(input: Node) -> NodeResult<PBXValue> {
        match_nodes!(input.into_children();
            [value(values)..] => values.collect::<Vec<PBXValue>>()
        )
        .pipe(PBXArray::new)
        .pipe(PBXValue::Array)
        .pipe(Ok)
    }

    fn value(input: Node) -> NodeResult<PBXValue> {
        match_nodes!(input.into_children();
         [array(value)] => value,
         [object(value)] => value,
         [string(value)] => value,
         [bool(value)] => value,
         [kind(value)] => value,
         [number(value)] => value,
         [uuid(value)] => value,
         [ident(value)] => value
        )
        .pipe(Ok)
    }

    fn field(node: Node) -> NodeResult<(String, PBXValue)> {
        let (k, v) = node.into_children().collect_tuple().unwrap();
        let key = Self::key(k)?;
        let value = Self::value(v)?;

        Ok((key, value))
    }

    fn object(input: Node) -> NodeResult<PBXValue> {
        match_nodes!(input.into_children();
            [field(fields)..] => fields.collect::<HashMap<String, PBXValue>>(),
        )
        .pipe(PBXHashMap::new)
        .pipe(PBXValue::Object)
        .pipe(Ok)
    }

    pub fn file(input: Node) -> NodeResult<PBXHashMap> {
        let node = input.into_children().next().unwrap();
        Self::object(node)?.try_into_object().unwrap().pipe(Ok)
    }
}

impl TryFrom<&str> for PBXProjectData {
    type Error = anyhow::Error;
    fn try_from(content: &str) -> anyhow::Result<Self> {
        let nodes = PBXProjectParser::parse(Rule::file, content).context("Parse content")?;
        let node = nodes.single().context("nodes to single")?;
        let mut object = PBXProjectParser::file(node)?;

        let archive_version = object.try_remove_number("archive_version")? as u8;
        let object_version = object.try_remove_number("object_version")? as u8;
        let classes = object.try_remove_object("classes").unwrap_or_default();
        let root_object_reference = object.try_remove_string("root_object")?;

        let objects = object.try_remove_object("objects")?;
        Ok(Self::new(
            archive_version,
            object_version,
            classes,
            objects,
            root_object_reference,
        ))
    }
}

impl TryFrom<String> for PBXProjectData {
    type Error = anyhow::Error;
    fn try_from(content: String) -> anyhow::Result<Self> {
        PBXProjectData::try_from(content.as_str())
    }
}

impl TryFrom<&Path> for PBXProjectData {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> anyhow::Result<Self> {
        std::fs::read_to_string(&value)
            .map_err(|e| anyhow::anyhow!("PBXProjectData from path {value:?}: {e}"))?
            .pipe(TryFrom::try_from)
    }
}

impl TryFrom<PathBuf> for PBXProjectData {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> anyhow::Result<Self> {
        Self::try_from(value.as_path())
    }
}

#[cfg(test)]
macro_rules! test_file {
    ($path:expr) => {{
        use super::*;

        let demo = std::fs::read_to_string($path).unwrap();
        let file = PBXProjectParser::parse(Rule::file, &demo);
        if file.is_err() {
            println!("Error: {:#?}", file.as_ref().unwrap_err())
        }
        assert!(file.is_ok());
        file.unwrap()
    }};
}

#[cfg(test)]
mod parse_tests {
    macro_rules! test_samples {
        ($($name:ident),*) => {
            $(#[test]
                fn $name() {
                    let (root, name) = (env!("CARGO_MANIFEST_DIR"), stringify!($name));
                    test_file!(format!("{root}/tests/samples/{name}.pbxproj"));
                })*
        };
    }

    test_samples![demo1, demo2, demo3, demo4, demo5, demo6, demo7, demo8, demo9];
}

#[cfg(test)]
mod consume {
    use super::*;
    use pest_consume::Parser;

    #[test]
    fn parse_key_pair() {
        let str =
            "0EC07ACE89150EC90442393B = {isa = PBXBuildFile; fileRef = F2E640B5C2B85914F6801498; };";
        let (key, value) = PBXProjectParser::parse(Rule::field, str)
            .map(|n| PBXProjectParser::field(n.single().unwrap()))
            .unwrap()
            .unwrap();

        assert_eq!(key, "0EC07ACE89150EC90442393B");
        assert!(matches!(value, PBXValue::Object(_)));

        let object = value.try_into_object().unwrap();
        assert_eq!(
            object.get("isa"),
            Some(&PBXValue::Kind("PBXBuildFile".into()))
        );
        assert_eq!(
            object["file_ref"],
            PBXValue::String("F2E640B5C2B85914F6801498".into())
        );
    }

    #[test]
    #[ignore = "reason"]
    fn test_consume() {
        let demo = include_str!("../../../tests/samples/demo2.pbxproj");
        let inputs = PBXProjectParser::parse(Rule::file, demo).unwrap();
        let input = inputs.single().unwrap();
        let object = PBXProjectParser::file(input).unwrap();
        println!("{object:#?}");
    }
}
