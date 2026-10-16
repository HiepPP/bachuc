//! Generates the protocol types from `codegen/protocol.schema.json`, which
//! `codegen/export-schema.mjs` exports from the zod schemas, so a build does not need Node.
//!
//! The schema is adjusted before typify sees it:
//! - Validation keywords are dropped, so decoding is not stricter than the protocol contract.
//! - Every schema that becomes a Rust type gets a unique title. typify names an inline type
//!   after its parent and silently merges two types of the same name, so without titles the
//!   `payload` of every session message would share the first payload's type.
//! - Recursive JSON value definitions become `{}`, which typify maps to `serde_json::Value`.
//!
//! After generation, every internally tagged enum gets an `Unrecognized` variant marked
//! `#[serde(other)]`, so a message type from a newer daemon decodes instead of failing the frame.
//! A list of string enum values that a newer daemon can extend skips each value it does not know.

use heck::ToPascalCase;
use quote::ToTokens;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use typify::{TypeSpace, TypeSpaceSettings};

const SCHEMA: &str = "codegen/protocol.schema.json";
const FALLBACK_VARIANT: &str = "Unrecognized";

fn main() {
    println!("cargo:rerun-if-changed={SCHEMA}");
    println!("cargo:rerun-if-changed=build.rs");
    let text = std::fs::read_to_string(SCHEMA).expect("read the protocol schema");
    let document: Value = serde_json::from_str(&text).expect("parse the protocol schema");
    let roots = document["roots"].as_object().expect("roots");

    let mut schemas = Vec::new();
    for (name, root) in roots {
        let mut root = root.clone();
        let root_map = root.as_object_mut().expect("root object");
        root_map.remove("$schema");
        let local = match root_map.remove("definitions") {
            Some(Value::Object(local)) => local,
            _ => Map::new(),
        };
        let json_values = local
            .iter()
            .filter(|(_, def)| is_json_value(def))
            .map(|(key, _)| key.clone())
            .collect::<BTreeSet<_>>();
        resolve_refs(&mut root, name, &json_values);
        loosen(&mut root);
        normalize_unions(&mut root);
        schemas.push((name.clone(), root));
        // Both exports name their definitions `__schemaN`, so scope the rest by root.
        for (key, mut def) in local {
            if json_values.contains(&key) {
                continue;
            }
            resolve_refs(&mut def, name, &json_values);
            loosen(&mut def);
            normalize_unions(&mut def);
            schemas.push((format!("{name}{key}"), def));
        }
    }

    let mut namer = Namer::default();
    for (name, schema) in &mut schemas {
        namer.collect_root(schema, name);
    }
    namer.resolve();
    let mut definitions = Map::new();
    for (name, mut schema) in schemas {
        namer.apply_root(&mut schema, &name);
        definitions.insert(name, schema);
    }

    let root_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": definitions,
    });
    let root_schema: schemars::schema::RootSchema =
        serde_json::from_value(root_schema).expect("schemars root schema");

    let mut settings = TypeSpaceSettings::default();
    settings.with_struct_builder(false);
    let mut space = TypeSpace::new(&settings);
    space
        .add_root_schema(root_schema)
        .expect("typify accepts the protocol schema");

    let mut file: syn::File = syn::parse2(space.to_stream()).expect("generated tokens parse");
    // typify merges two types of the same name without a check, so a type it named itself
    // could stand for two different schemas. Every type must carry a title from the namer.
    let titles = namer.names.values().collect::<HashSet<_>>();
    let untitled = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item.ident.to_string()),
            syn::Item::Enum(item) => Some(item.ident.to_string()),
            syn::Item::Type(item) => Some(item.ident.to_string()),
            _ => None,
        })
        .filter(|name| !titles.contains(name))
        .collect::<Vec<_>>();
    assert!(
        untitled.is_empty(),
        "typify named these types itself; teach becomes_type about them: {untitled:?}"
    );
    add_fallback_variants(&mut file);
    skip_unknown_list_values(&mut file);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("protocol.rs");
    std::fs::write(&out, prettyplease::unparse(&file)).expect("write generated code");
}

/// A definition that zod emits for `z.json()`-like values: any JSON, defined recursively.
fn is_json_value(def: &Value) -> bool {
    let Some(members) = def["anyOf"].as_array() else {
        return false;
    };
    let types = members
        .iter()
        .filter_map(|m| m["type"].as_str())
        .collect::<BTreeSet<_>>();
    members.len() == 6
        && types == BTreeSet::from(["array", "boolean", "null", "number", "object", "string"])
}

/// Replaces a reference to a JSON value definition with `{}`, and scopes the other
/// references by root name.
fn resolve_refs(value: &mut Value, root: &str, json_values: &BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(target)) = map.get("$ref") {
                if let Some(key) = target.strip_prefix("#/definitions/") {
                    if json_values.contains(key) {
                        map.clear();
                        return;
                    }
                    let scoped = format!("#/definitions/{root}{key}");
                    map.insert("$ref".into(), Value::String(scoped));
                }
            }
            map.values_mut()
                .for_each(|v| resolve_refs(v, root, json_values));
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|v| resolve_refs(v, root, json_values)),
        _ => {}
    }
}

/// Drops the validation keywords that would make decoding stricter than the protocol contract:
/// a closed object would reject a new field, and length, pattern, or format checks would reject
/// a value the client does not need to judge. Defaults go too: an optional field decodes to
/// `None`, and typify rejects some zod defaults. Keyword values are never objects, so a
/// property schema with one of these names stays.
fn loosen(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.get("additionalProperties") == Some(&Value::Bool(false)) {
                map.remove("additionalProperties");
            }
            for key in [
                "pattern",
                "format",
                "minLength",
                "maxLength",
                "minItems",
                "maxItems",
                "default",
            ] {
                if map.get(key).is_some_and(|v| !v.is_object()) {
                    map.remove(key);
                }
            }
            map.values_mut().for_each(loosen);
        }
        Value::Array(items) => items.iter_mut().for_each(loosen),
        _ => {}
    }
}

/// Rewrites unions so typify can tag them. typify turns an `anyOf` whose members are not
/// mutually exclusive into a struct of flattened `Option`s, which decodes a frame into the
/// wrong shape, and it cannot nest one tagged enum inside a variant of another.
/// - A member that is itself a union is spliced into the parent.
/// - Members that share a tag value, such as the four `tool_call` statuses, merge into one
///   object. A property that differs between them becomes a string enum when every member has
///   a constant, and untyped JSON otherwise.
/// - A union whose members then carry distinct tag values becomes a `oneOf`.
fn normalize_unions(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.values_mut().for_each(normalize_unions);
            for key in ["anyOf", "oneOf"] {
                let Some(Value::Array(members)) = map.get(key) else {
                    continue;
                };
                if is_optional(members) {
                    continue;
                }
                let mut spliced = Vec::new();
                for member in members {
                    match pure_union(member) {
                        Some(inner) => spliced.extend(inner.iter().cloned()),
                        None => spliced.push(member.clone()),
                    }
                }
                let merged = merge_by_tag(spliced);
                let tagged = union_tags(&merged).is_some();
                map.remove(key);
                let key = if tagged { "oneOf" } else { key };
                map.insert(key.into(), Value::Array(merged));
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalize_unions),
        _ => {}
    }
}

/// The members of a schema that is only a union, with no other keywords.
fn pure_union(schema: &Value) -> Option<&Vec<Value>> {
    let map = schema.as_object()?;
    let key = ["anyOf", "oneOf"]
        .into_iter()
        .find(|key| map.contains_key(*key))?;
    let others = map
        .keys()
        .all(|k| k == key || k == "description" || k == "title");
    let members = map[key].as_array()?;
    (others && !is_optional(members)).then_some(members)
}

/// Merges the members that share a tag value. The tag is the property every member holds as a
/// required constant string, with the most distinct values.
fn merge_by_tag(members: Vec<Value>) -> Vec<Value> {
    let constants = members
        .iter()
        .map(|member| {
            let properties = member["properties"].as_object()?;
            let required = member["required"].as_array()?;
            Some(
                properties
                    .iter()
                    .filter(|(key, _)| required.iter().any(|r| r == key.as_str()))
                    .filter_map(|(key, schema)| Some((key.clone(), constant_string(schema)?)))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<Option<Vec<_>>>();
    let Some(constants) = constants else {
        return members;
    };
    let distinct = |key: &String| {
        constants
            .iter()
            .map(|c| &c[key])
            .collect::<BTreeSet<_>>()
            .len()
    };
    let Some(tag) = constants
        .first()
        .into_iter()
        .flat_map(|first| first.keys())
        .filter(|key| constants.iter().all(|c| c.contains_key(*key)))
        .max_by(|a, b| distinct(a).cmp(&distinct(b)).then(b.cmp(a)))
        .cloned()
    else {
        return members;
    };
    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    for (member, constants) in members.into_iter().zip(&constants) {
        let value = &constants[&tag];
        match groups.iter_mut().find(|(v, _)| v == value) {
            Some((_, group)) => group.push(member),
            None => groups.push((value.clone(), vec![member])),
        }
    }
    groups
        .into_iter()
        .map(|(_, group)| {
            if group.len() == 1 {
                group.into_iter().next().unwrap()
            } else {
                merge_objects(&group)
            }
        })
        .collect()
}

fn merge_objects(group: &[Value]) -> Value {
    let mut names = BTreeSet::new();
    for member in group {
        names.extend(member["properties"].as_object().unwrap().keys().cloned());
    }
    let mut properties = Map::new();
    for name in &names {
        let schemas = group
            .iter()
            .map(|m| m["properties"].get(name))
            .collect::<Vec<_>>();
        let first = schemas[0];
        let merged = if first.is_some() && schemas.iter().all(|s| *s == first) {
            first.unwrap().clone()
        } else {
            let values = schemas
                .iter()
                .map(|s| s.and_then(constant_string))
                .collect::<Option<BTreeSet<_>>>();
            match values {
                Some(values) => serde_json::json!({"type": "string", "enum": values}),
                None => serde_json::json!({}),
            }
        };
        properties.insert(name.clone(), merged);
    }
    let required = names
        .iter()
        .filter(|name| {
            group.iter().all(|m| {
                m["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r == name.as_str())
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    serde_json::json!({"type": "object", "properties": properties, "required": required})
}

/// How many name candidates each schema passes down to its children.
const KEPT_CANDIDATES: usize = 3;

/// Gives each schema that typify turns into a Rust type a unique title. A variant of a tagged
/// union is named after its tag (`agent_stream` -> `AgentStream`); other types are named after
/// their parent and property. Identical schemas share one type, named with the shortest
/// candidate from any place they appear, so the name does not depend on which message lists
/// the schema first.
///
/// Naming runs in two passes over the same schemas: `collect` gathers candidates per schema
/// content, `resolve` picks unique names, and `apply` writes them as titles.
#[derive(Default)]
struct Namer {
    candidates: HashMap<String, BTreeSet<(usize, String)>>,
    order: Vec<String>,
    roots: Vec<(String, String)>,
    names: HashMap<String, String>,
}

impl Namer {
    fn collect_root(&mut self, root: &mut Value, name: &str) {
        let name = name.to_pascal_case();
        self.roots.push((root.to_string(), name.clone()));
        self.children(root, &[name], true, false);
    }

    fn apply_root(&mut self, root: &mut Value, name: &str) {
        let name = name.to_pascal_case();
        self.children(root, std::slice::from_ref(&name), true, true);
        set_title(root, &name);
    }

    fn resolve(&mut self) {
        let mut used = HashSet::new();
        for (content, name) in &self.roots {
            assert!(used.insert(name.clone()), "two roots named {name}");
            self.names.insert(content.clone(), name.clone());
        }
        for content in &self.order {
            if self.names.contains_key(content) {
                continue;
            }
            let candidates = &self.candidates[content];
            let first = &candidates.first().unwrap().1;
            let name = candidates
                .iter()
                .map(|(_, name)| name.clone())
                .chain((2..).map(|n| format!("{first}{n}")))
                .find(|name| !used.contains(name))
                .unwrap();
            used.insert(name.clone());
            self.names.insert(content.clone(), name);
        }
    }

    fn visit(
        &mut self,
        node: &mut Value,
        hints: Vec<String>,
        preferred: Option<String>,
        apply: bool,
    ) {
        if !node.is_object() {
            return;
        }
        let bases = if becomes_type(node) {
            let content = node.to_string();
            let own = ranked(preferred.into_iter().chain(hints));
            if apply {
                let name = self.names[&content].clone();
                set_title(node, &name);
            } else {
                if !self.candidates.contains_key(&content) {
                    self.order.push(content.clone());
                }
                let entry = self.candidates.entry(content).or_default();
                entry.extend(own.iter().map(|name| (name.len(), name.clone())));
            }
            own
        } else {
            hints
        };
        self.children(node, &bases, false, apply);
    }

    /// `root` marks the members of a root union. They take the root name as a prefix
    /// (`WsInboundSession`), so the two envelopes name their session messages alike.
    fn children(&mut self, node: &mut Value, bases: &[String], root: bool, apply: bool) {
        let suffixed = |suffix: &str| -> Vec<String> {
            ranked(bases.iter().map(|base| format!("{base}-{suffix}")))
        };
        let Some(map) = node.as_object_mut() else {
            return;
        };
        for key in ["oneOf", "anyOf"] {
            let Some(Value::Array(members)) = map.get_mut(key) else {
                continue;
            };
            let tags = union_tags(members);
            let optional = is_optional(members);
            for (index, member) in members.iter_mut().enumerate() {
                if let Some(tags) = &tags {
                    let tag = &tags[index];
                    let preferred = if root {
                        format!("{}-{tag}", bases[0])
                    } else {
                        tag.clone()
                    };
                    self.visit(member, suffixed(tag), Some(preferred), apply);
                } else if optional {
                    // `anyOf [X, null]` becomes `Option<X>`, so X keeps the parent's hints.
                    self.visit(member, bases.to_vec(), None, apply);
                } else {
                    self.visit(member, suffixed(&format!("Variant{index}")), None, apply);
                }
            }
        }
        if let Some(Value::Array(members)) = map.get_mut("allOf") {
            for member in members {
                self.visit(member, bases.to_vec(), None, apply);
            }
        }
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            for (key, property) in properties.iter_mut() {
                self.visit(property, suffixed(key), None, apply);
            }
        }
        if let Some(items) = map.get_mut("items") {
            match items {
                Value::Array(items) => {
                    for (index, item) in items.iter_mut().enumerate() {
                        self.visit(item, suffixed(&format!("Item{index}")), None, apply);
                    }
                }
                item => self.visit(item, suffixed("Item"), None, apply),
            }
        }
        if let Some(value) = map.get_mut("additionalProperties") {
            self.visit(value, suffixed("Value"), None, apply);
        }
    }
}

/// Pascal-cases the names, drops duplicates, and keeps the shortest few.
fn ranked(names: impl Iterator<Item = String>) -> Vec<String> {
    let names = names
        .map(|name| name.to_pascal_case())
        .map(|name| (name.len(), name))
        .collect::<BTreeSet<_>>();
    names
        .into_iter()
        .take(KEPT_CANDIDATES)
        .map(|(_, name)| name)
        .collect()
}

fn set_title(node: &mut Value, name: &str) {
    if let Some(map) = node.as_object_mut() {
        map.insert("title".into(), Value::String(name.to_string()));
    }
}

fn is_null(node: &Value) -> bool {
    node["type"] == "null"
}

/// Whether typify generates a named type for this schema: a struct, a string enum, or a union.
fn becomes_type(node: &Value) -> bool {
    let has_properties = node["properties"]
        .as_object()
        .is_some_and(|p| !p.is_empty());
    let has_enum = node["enum"].as_array().is_some_and(|e| !e.is_empty());
    let has_union = ["oneOf", "anyOf"].iter().any(|key| {
        node[key]
            .as_array()
            .is_some_and(|members| !is_optional(members))
    });
    has_properties || has_enum || has_union
}

/// `anyOf [X, null]`, which typify turns into `Option<X>` with no type of its own.
fn is_optional(members: &[Value]) -> bool {
    members.len() == 2 && members.iter().filter(|m| is_null(m)).count() == 1
}

/// The tag value of each member when the union is internally tagged the way typify detects it:
/// every member is an object with a required constant string property, and the values differ.
/// typify picks the first such property by name.
fn union_tags(members: &[Value]) -> Option<Vec<String>> {
    let per_member = members
        .iter()
        .map(|member| {
            let properties = member["properties"].as_object()?;
            let required = member["required"].as_array()?;
            let constants = properties
                .iter()
                .filter(|(key, _)| required.iter().any(|r| r == key.as_str()))
                .filter_map(|(key, schema)| Some((key.clone(), constant_string(schema)?)))
                .collect::<BTreeMap<_, _>>();
            Some(constants)
        })
        .collect::<Option<Vec<_>>>()?;
    let first = per_member.first()?;
    let tag = first.keys().find(|key| {
        let values = per_member
            .iter()
            .map(|constants| constants.get(*key))
            .collect::<Option<Vec<_>>>();
        values.is_some_and(|values| values.iter().collect::<BTreeSet<_>>().len() == values.len())
    })?;
    Some(per_member.iter().map(|c| c[tag].clone()).collect())
}

fn constant_string(schema: &Value) -> Option<String> {
    if let Some(value) = schema["const"].as_str() {
        return Some(value.to_string());
    }
    match schema["enum"].as_array() {
        Some(values) if values.len() == 1 => values[0].as_str().map(str::to_string),
        _ => None,
    }
}

/// List fields of a string enum that a newer daemon can extend, as (struct, field).
const OPEN_ENUM_LISTS: &[(&str, &str)] = &[("ServerInfo", "permissions")];

/// Makes each `OPEN_ENUM_LISTS` field skip a value this client does not know, so the known
/// values stay typed and the message still decodes.
fn skip_unknown_list_values(file: &mut syn::File) {
    for (struct_name, field_name) in OPEN_ENUM_LISTS {
        let field = file
            .items
            .iter_mut()
            .find_map(|item| match item {
                syn::Item::Struct(item) if item.ident == *struct_name => {
                    item.fields.iter_mut().find(|field| {
                        field
                            .ident
                            .as_ref()
                            .is_some_and(|ident| ident == field_name)
                    })
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no generated field {struct_name}.{field_name}"));
        field.attrs.push(syn::parse_quote! {
            #[serde(deserialize_with = "crate::lenient::known_values")]
        });
    }
}

/// Adds `#[serde(other)] Unrecognized` to every internally tagged enum.
fn add_fallback_variants(file: &mut syn::File) {
    for item in &mut file.items {
        let syn::Item::Enum(item) = item else {
            continue;
        };
        let internally_tagged = item.attrs.iter().any(|attr| {
            let text = attr.meta.to_token_stream().to_string();
            text.starts_with("serde") && text.contains("tag =") && !text.contains("content =")
        });
        if !internally_tagged {
            continue;
        }
        assert!(
            !item.variants.iter().any(|v| v.ident == FALLBACK_VARIANT),
            "{} already has a variant named {FALLBACK_VARIANT}",
            item.ident
        );
        item.variants.push(syn::parse_quote! {
            /// A message type this client does not know, from a newer daemon.
            #[serde(other)]
            Unrecognized
        });
    }
}
