/*
 * Copyright 2022-2023 Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use cedar_policy_core::entities::JSONValue;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use smol_str::SmolStr;
use std::collections::{BTreeMap, HashMap};

use crate::Result;

/// A SchemaFragment describe the types for a given instance of Cedar.
/// SchemaFragments are composed of Entity Types and Action Types. The
/// schema fragment is split into multiple namespace definitions, eac including
/// a namespace name which is applied to all entity types (and the implicit
/// `Action` entity type for all actions) in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaFragment(
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub  HashMap<SmolStr, NamespaceDefinition>,
);

impl SchemaFragment {
    /// Create a `SchemaFragment` from a JSON value (which should be an object
    /// of the appropriate shape).
    pub fn from_json_value(json: serde_json::Value) -> Result<Self> {
        serde_json::from_value(json).map_err(Into::into)
    }

    /// Create a `SchemaFragment` directly from a file.
    pub fn from_file(file: impl std::io::Read) -> Result<Self> {
        serde_json::from_reader(file).map_err(Into::into)
    }
}

/// A single namespace definition from a SchemaFragment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde_as]
#[serde(deny_unknown_fields)]
#[doc(hidden)]
pub struct NamespaceDefinition {
    #[serde(default)]
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    #[serde(rename = "commonTypes")]
    pub common_types: HashMap<SmolStr, SchemaType>,
    #[serde(rename = "entityTypes")]
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub entity_types: HashMap<SmolStr, EntityType>,
    #[serde(with = "::serde_with::rust::maps_duplicate_key_is_error")]
    pub actions: HashMap<SmolStr, ActionType>,
}

impl NamespaceDefinition {
    pub fn new(
        entity_types: impl IntoIterator<Item = (SmolStr, EntityType)>,
        actions: impl IntoIterator<Item = (SmolStr, ActionType)>,
    ) -> Self {
        Self {
            common_types: HashMap::new(),
            entity_types: entity_types.into_iter().collect(),
            actions: actions.into_iter().collect(),
        }
    }
}

impl std::fmt::Display for NamespaceDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(
            &serde_json::to_string_pretty(&self).expect("failed to serialize NamespaceContents"),
        )
    }
}

/// Entity types describe the relationships in the entity store, including what
/// entities can be members of groups of what types, and what attributes
/// can/should be included on entities of each type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityType {
    #[serde(default)]
    #[serde(rename = "memberOfTypes")]
    pub member_of_types: Vec<SmolStr>,
    #[serde(default)]
    pub shape: AttributesOrContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttributesOrContext(
    // We use the usual `SchemaType` deserialization, but it will ultimately
    // need to be a `Record` or type def which resolves to a `Record`.
    pub SchemaType,
);

impl AttributesOrContext {
    pub fn into_inner(self) -> SchemaType {
        self.0
    }
}

impl Default for AttributesOrContext {
    fn default() -> Self {
        Self(SchemaType::Type(SchemaTypeVariant::Record {
            attributes: BTreeMap::new(),
            additional_attributes: false,
        }))
    }
}

/// An action type describes a specific action entity.  It also describes what
/// kinds of entities it can be used on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionType {
    /// This maps attribute names to
    /// `cedar_policy_core::entities::json::jsonvalue::JSONValue` which is the
    /// canonical representation of a cedar value as JSON.
    #[serde(default)]
    pub attributes: Option<HashMap<SmolStr, JSONValue>>,
    #[serde(default)]
    #[serde(rename = "appliesTo")]
    pub applies_to: Option<ApplySpec>,
    #[serde(default)]
    #[serde(rename = "memberOf")]
    pub member_of: Option<Vec<ActionEntityUID>>,
}

/// The apply spec specifies what principals and resources an action can be used
/// with.  This specification can either be done through containing to entity
/// types. The fields of this record are optional so that they can be omitted to
/// declare that the apply spec for the principal or resource is undefined,
/// meaning that the action can be applied to any principal or resource. This is
/// different than providing an empty list because the empty list is interpreted
/// as specifying that there are no principals or resources that an action
/// applies to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySpec {
    #[serde(default)]
    #[serde(rename = "resourceTypes")]
    pub resource_types: Option<Vec<SmolStr>>,
    #[serde(default)]
    #[serde(rename = "principalTypes")]
    pub principal_types: Option<Vec<SmolStr>>,
    #[serde(default)]
    pub context: AttributesOrContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionEntityUID {
    pub id: SmolStr,

    #[serde(rename = "type")]
    #[serde(default)]
    pub ty: Option<SmolStr>,
}

impl ActionEntityUID {
    pub fn default_type(id: SmolStr) -> Self {
        Self { id, ty: None }
    }
}

impl std::fmt::Display for ActionEntityUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ty) = &self.ty {
            write!(f, "{}::", ty)?
        } else {
            write!(f, "Action::")?
        }
        write!(f, "\"{}\"", self.id)
    }
}

/// A restricted version of the `Type` enum containing only the types which are
/// exposed to users.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
// This enum is `untagged` with these variants as a workaround to a serde
// limitation. It is not possible to have the known variants on one enum, and
// then, have catch-all variant for any unrecognized tag in the same enum that
// captures the name of the unrecognized tag.
#[serde(untagged)]
pub enum SchemaType {
    Type(SchemaTypeVariant),
    TypeDef {
        #[serde(rename = "type")]
        type_name: SmolStr,
    },
}

impl From<SchemaTypeVariant> for SchemaType {
    fn from(variant: SchemaTypeVariant) -> Self {
        Self::Type(variant)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(deny_unknown_fields)]
pub enum SchemaTypeVariant {
    String,
    Long,
    Boolean,
    Set {
        element: Box<SchemaType>,
    },
    Record {
        #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
        attributes: BTreeMap<SmolStr, TypeOfAttribute>,
        #[serde(rename = "additionalAttributes")]
        #[serde(default = "additional_attributes_default")]
        additional_attributes: bool,
    },
    Entity {
        name: SmolStr,
    },
    Extension {
        name: SmolStr,
    },
}

// The possible tags for a SchemaType as written in a schema JSON document. Used
// to forbid declaring a custom typedef with the same name as a builtin type.
// This must be kept up to date with the variants for `SchemaTypeVariant` and
// their actual serialization by serde. There is crate that looks like it could
// do this automatically, but it returns an empty slice for the variants names
// of `SchemaTypeVariant`.
// https://docs.rs/serde-aux/latest/serde_aux/serde_introspection/fn.serde_introspect.html
pub(crate) static SCHEMA_TYPE_VARIANT_TAGS: &[&str] = &[
    "String",
    "Long",
    "Boolean",
    "Set",
    "Record",
    "Entity",
    "Extension",
];

impl SchemaType {
    /// Is this `SchemaType` an extension type, or does it contain one
    /// (recursively)? Returns `None` if this is a `TypeDef` because we can't
    /// easily properly check the type of a typedef, accounting for namespaces,
    /// without first converting to a `Type`.
    pub fn is_extension(&self) -> Option<bool> {
        match self {
            Self::Type(SchemaTypeVariant::Extension { .. }) => Some(true),
            Self::Type(SchemaTypeVariant::Set { element }) => element.is_extension(),
            Self::Type(SchemaTypeVariant::Record { attributes, .. }) => {
                attributes
                    .values()
                    .fold(Some(false), |a, e| match e.ty.is_extension() {
                        Some(true) => Some(true),
                        Some(false) => a,
                        None => None,
                    })
            }
            Self::Type(_) => Some(false),
            Self::TypeDef { .. } => None,
        }
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for SchemaType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<SchemaType> {
        use cedar_policy_core::ast::Name;
        use std::collections::HashSet;

        Ok(SchemaType::Type(match u.int_in_range::<u8>(1..=8)? {
            1 => SchemaTypeVariant::String,
            2 => SchemaTypeVariant::Long,
            3 => SchemaTypeVariant::Boolean,
            4 => SchemaTypeVariant::Set {
                element: Box::new(u.arbitrary()?),
            },
            5 => {
                let attributes = {
                    let attr_names: HashSet<String> = u.arbitrary()?;
                    attr_names
                        .into_iter()
                        .map(|attr_name| Ok((attr_name.into(), u.arbitrary()?)))
                        .collect::<arbitrary::Result<_>>()?
                };
                SchemaTypeVariant::Record {
                    attributes,
                    additional_attributes: u.arbitrary()?,
                }
            }
            6 => {
                let name: Name = u.arbitrary()?;
                SchemaTypeVariant::Entity {
                    name: name.to_string().into(),
                }
            }
            7 => SchemaTypeVariant::Extension {
                name: "ipaddr".into(),
            },
            8 => SchemaTypeVariant::Extension {
                name: "decimal".into(),
            },
            n => panic!("bad index: {n}"),
        }))
    }
    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, None) // Unfortunately, we probably can't be more precise than this
    }
}

/// Used to describe the type of a record or entity attribute. It contains a the
/// type of the attribute and whether the attribute is required. The type is
/// flattened for serialization, so, in JSON format, this appears as a regular
/// type with one extra property `required`.
///
/// Note that we can't add #[serde(deny_unknown_fields)] here because we are
/// using #[serde(tag = "type")] in ty:SchemaType which is flattened here.
/// The way serde(flatten) is implemented means it may be possible to access
/// fields incorrectly if a struct contains two structs that are flattened
/// (`<https://github.com/serde-rs/serde/issues/1547>`). This shouldn't apply to
/// us as we're using flatten only once
/// (`<https://github.com/serde-rs/serde/issues/1600>`). This should be ok because
/// unknown fields for TypeOfAttribute should be passed to SchemaType where
/// they will be denied (`<https://github.com/serde-rs/serde/issues/1600>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TypeOfAttribute {
    #[serde(flatten)]
    pub ty: SchemaType,
    #[serde(default = "record_attribute_required_default")]
    pub required: bool,
}

/// Defines the default value for `additionalAttributes` on records and
/// entities
fn additional_attributes_default() -> bool {
    false
}

/// Defines the default value for `required` on record and entity attributes.
fn record_attribute_required_default() -> bool {
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_entity_type_parser1() {
        let user = r#"
        {
            "memberOfTypes" : ["UserGroup"]
        }
        "#;
        let et = serde_json::from_str::<EntityType>(user).expect("Parse Error");
        assert_eq!(et.member_of_types, vec!["UserGroup"]);
        assert_eq!(
            et.shape.into_inner(),
            SchemaType::Type(SchemaTypeVariant::Record {
                attributes: BTreeMap::new(),
                additional_attributes: false
            })
        );
    }

    #[test]
    fn test_entity_type_parser2() {
        let src = r#"
              { }
        "#;
        let et = serde_json::from_str::<EntityType>(src).expect("Parse Error");
        assert_eq!(et.member_of_types.len(), 0);
        assert_eq!(
            et.shape.into_inner(),
            SchemaType::Type(SchemaTypeVariant::Record {
                attributes: BTreeMap::new(),
                additional_attributes: false
            })
        );
    }

    #[test]
    fn test_entity_type_parser3() {
        let src = r#"
        {
            "memberOf" : ["UserGroup"],
            "shape": {
                "type": "Record",
                "attributes": {
                    "name": { "type": "String", "required": false},
                    "name": { "type": "String", "required": true},
                    "age": { "type": "Long", "required": false}
                }
            }
        }
        "#;
        let et = serde_json::from_str::<EntityType>(src);
        match et {
            Ok(_) => panic!("serde_json parsing should have failed"),
            Err(e) => {
                assert_eq!(e.classify(), serde_json::error::Category::Data);
            }
        }
    }

    #[test]
    fn test_action_type_parser1() {
        let src = r#"
              {
                "appliesTo" : {
                  "resourceTypes": ["Album"],
                  "principalTypes": ["User"]
                },
                "memberOf": [{"id": "readWrite"}]
              }
        "#;
        let at: ActionType = serde_json::from_str(src).expect("Parse Error");
        let spec = ApplySpec {
            resource_types: Some(vec!["Album".into()]),
            principal_types: Some(vec!["User".into()]),
            context: AttributesOrContext::default(),
        };
        assert_eq!(at.applies_to, Some(spec));
        assert_eq!(
            at.member_of,
            Some(vec![ActionEntityUID {
                ty: None,
                id: "readWrite".into()
            }])
        );
    }

    #[test]
    fn test_action_type_parser2() {
        let src = r#"
              { }
        "#;
        let at: ActionType = serde_json::from_str(src).expect("Parse Error");
        assert_eq!(at.applies_to, None);
        assert!(at.member_of.is_none());
    }

    #[test]
    fn test_schema_file_parser() {
        let src = serde_json::json!(
        {
            "entityTypes": {

              "User": {
                "memberOfTypes": ["UserGroup"]
              },
              "Photo": {
                "memberOfTypes": ["Album", "Account"]
              },

              "Album": {
                "memberOfTypes": ["Album", "Account"]
              },
              "Account": { },
              "UserGroup": { }
           },

           "actions": {
              "readOnly": { },
              "readWrite": { },
              "createAlbum": {
                "appliesTo" : {
                  "resourceTypes": ["Account", "Album"],
                  "principalTypes": ["User"]
                },
                "memberOf": [{"id": "readWrite"}]
              },
              "addPhotoToAlbum": {
                "appliesTo" : {
                  "resourceTypes": ["Album"],
                  "principalTypes": ["User"]
                },
                "memberOf": [{"id": "readWrite"}]
              },
              "viewPhoto": {
                "appliesTo" : {
                  "resourceTypes": ["Photo"],
                  "principalTypes": ["User"]
                },
                "memberOf": [{"id": "readOnly"}, {"id": "readWrite"}]
              },
              "viewComments": {
                "appliesTo" : {
                  "resourceTypes": ["Photo"],
                  "principalTypes": ["User"]
                },
                "memberOf": [{"id": "readOnly"}, {"id": "readWrite"}]
              }
            }
          });
        let schema_file: NamespaceDefinition = serde_json::from_value(src).expect("Parse Error");

        assert_eq!(schema_file.entity_types.len(), 5);
        assert_eq!(schema_file.actions.len(), 6);
    }

    #[test]
    fn test_parse_namespaces() {
        let src = r#"
        {
            "foo::foo::bar::baz": {
                "entityTypes": {},
                "actions": {}
            }
        }"#;
        let schema: SchemaFragment = serde_json::from_str(src).expect("Parse Error");
        let (namespace, _descriptor) = schema.0.into_iter().next().unwrap();
        assert_eq!(namespace, "foo::foo::bar::baz".to_string());
    }

    #[test]
    #[should_panic]
    fn test_schema_file_with_misspelled_required() {
        let src = r#"
        {
            "entityTypes": {
                "User": {
                    "memberOf": [ "Group" ],
                    "shape": {
                        "type": "Record",
                        "additionalAttributess": false,
                        "attributes": {
                            "name": { "type": "String", "required": true},
                            "age": { "type": "Long", "required": true},
                            "favorite": { "type": "Entity", "name": "Photo", "requiredddddd": false}
                        },
                        "required": false
                    }
                }
            },
            "actions": []
        }
        "#;
        let schema: NamespaceDefinition = serde_json::from_str(src).expect("Expected valid schema");
        println!("{:#?}", schema);
    }

    #[test]
    #[should_panic]
    fn test_schema_file_with_misspelled_attribute() {
        let src = r#"
        {
            "entityTypes": [
                "User": {
                    "memberOf": [ "Group" ],
                    "shape": {
                        "type": "Record",
                        "additionalAttributess": false,
                        "attributes": {
                            "name": { "type": "String", "required": true},
                            "age": { "type": "Long", "required": true},
                            "favorite": { "type": "Entity", "nameeeeee": "Photo", "required": false}
                        },
                        "required": false
                    }
                }
            ],
            "actions": []
        }
        "#;
        let schema: NamespaceDefinition = serde_json::from_str(src).expect("Expected valid schema");
        println!("{:#?}", schema);
    }

    #[test]
    #[should_panic]
    fn test_schema_file_with_extra_attribute() {
        let src = r#"
        {
            "entityTypes": [
                "User": {
                    "memberOf": [ "Group" ],
                    "shape": {
                        "type": "Record",
                        "additionalAttributess": false,
                        "attributes": {
                            "name": { "type": "String", "required": true},
                            "age": { "type": "Long", "required": true},
                            "favorite": { "type": "Entity", "name": "Photo", "required": false, "extra": "Should not exist"}
                        },
                        "required": false
                    }
                }
            ],
            "actions": []
        }
        "#;
        let schema: NamespaceDefinition = serde_json::from_str(src).expect("Expected valid schema");
        println!("{:#?}", schema);
    }
}
