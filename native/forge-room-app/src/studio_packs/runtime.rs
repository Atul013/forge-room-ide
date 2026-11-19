//! Prepared resource snapshots contain no grants, project paths, executable commands or network clients.
use forge_room_core::packs::{self, Activation, Catalog, Kind, Profile, Widget};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock, RwLock},
};
#[derive(Clone)]
pub enum Value {
    Theme(String),
    Grammar(String),
    Snippets(usize),
    Workflow(crate::workflow_center::WorkflowLibrary),
    Skill(String),
    Profile(Profile),
    Widget(Widget),
    Readme(String),
    RoomBlueprint(forge_room_core::transport::RoomBlueprint),
}
#[derive(Clone)]
pub struct Asset {
    pub pack: String,
    pub resource: String,
    pub label: String,
    pub digest: String,
    pub kind: Kind,
    pub value: Value,
}
#[derive(Default)]
pub struct Resources {
    pub themes: Vec<Arc<crate::theme::Theme>>,
    pub grammars: Vec<(String, String)>,
    pub snippets: Vec<crate::snippets::Snippet>,
    pub assets: BTreeMap<String, Asset>,
    pub theme_refs: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}
fn slot() -> &'static RwLock<Arc<Resources>> {
    static RUNTIME: OnceLock<RwLock<Arc<Resources>>> = OnceLock::new();
    RUNTIME.get_or_init(|| RwLock::new(Arc::new(Resources::default())))
}
pub fn current() -> Arc<Resources> {
    slot().read().expect("pack resources lock").clone()
}
pub fn theme_reference(actual: &str) -> String {
    current()
        .theme_refs
        .iter()
        .find(|(_, key)| key.as_str() == actual)
        .map(|(reference, _)| reference.clone())
        .unwrap_or_else(|| actual.into())
}
pub fn validate_pack(pack: &packs::Pack) -> Result<Vec<String>, String> {
    pack.validate()?;
    let mut notes = Vec::new();
    for resource in &pack.resources {
        let result = match resource.kind {
            Kind::Theme => crate::theme::parse_import(&resource.content).map(|_| ()),
            Kind::Grammar => crate::syntax::parse_grammar(&resource.content).map(|_| ()),
            Kind::Snippets => crate::snippets::parse_pack(&resource.content).map(|_| ()),
            Kind::Workflow => {
                crate::workflow_center::WorkflowLibrary::parse(resource.content.as_bytes())
                    .map(|_| ())
            }
            Kind::Profile => Profile::parse(&resource.content).map(|_| ()),
            Kind::Widget => Widget::parse(&resource.content).map(|_| ()),
            _ => Ok(()),
        };
        result.map_err(|error| format!("{}: {error}", resource.id))?;
        notes.push(format!(
            "{}: {} · {} bytes",
            resource.id,
            resource.kind.label(),
            resource.content.len()
        ));
    }
    Ok(notes)
}
pub struct Prepared {
    pub resources: Resources,
    syntax: crate::syntax::Prepared,
}
/// Clearing damaged contributions must not depend on successfully rebuilding optional loose grammars.
pub fn disabled() -> Prepared {
    disabled_with_syntax(crate::syntax::prepare_packs(&[]))
}
fn disabled_with_syntax(syntax: Result<crate::syntax::Prepared, String>) -> Prepared {
    let mut resources = Resources::default();
    for theme in crate::theme::base_all() {
        resources
            .theme_refs
            .insert(theme.key.clone(), theme.key.clone());
    }
    let syntax = match syntax {
        Ok(syntax) => syntax,
        Err(error) => {
            resources.warnings.push(format!(
                "Optional grammar rebuild failed; built-in highlighting restored: {error}"
            ));
            crate::syntax::builtins()
        }
    };
    Prepared { resources, syntax }
}
pub fn prepare(activation: &Activation, catalog: &Catalog) -> Result<Prepared, String> {
    let order = activation.validate(catalog)?;
    let mut resources = Resources::default();
    for theme in crate::theme::base_all() {
        resources
            .theme_refs
            .insert(theme.key.clone(), theme.key.clone());
    }
    for id in order {
        let selection = &activation.enabled[&id];
        let pack = &catalog[&format!("{id}@{}", selection.version)];
        let pack_digest = pack.digest()?;
        for resource in &pack.resources {
            if !selection.resources.contains(&resource.id) {
                continue;
            }
            let key = format!("{id}/{}", resource.id);
            let value = match resource.kind {
                Kind::Theme => {
                    let mut theme = crate::theme::parse_import(&resource.content)?;
                    let actual = format!("pack-{}-{}", &pack_digest[..16], resource.id);
                    let theme_mut = Arc::make_mut(&mut theme);
                    theme_mut.key = actual.clone();
                    theme_mut.mood = "Pack theme".into();
                    theme_mut.description = format!(
                        "{} · {} · local publisher claim {}",
                        pack.key(),
                        resource.id,
                        pack.publisher
                    );
                    resources.themes.push(theme);
                    resources.theme_refs.insert(key.clone(), actual.clone());
                    Value::Theme(actual)
                }
                Kind::Grammar => {
                    let grammar = crate::syntax::parse_grammar(&resource.content)?;
                    let defaults = syntect::parsing::SyntaxSet::load_defaults_newlines();
                    for extension in &grammar.file_extensions {
                        if defaults.find_syntax_by_extension(extension).is_some() {
                            resources.warnings.push(format!(
                                "{key} overrides built-in highlighting for .{extension}"
                            ));
                        }
                    }
                    resources
                        .grammars
                        .push((key.clone(), resource.content.clone()));
                    Value::Grammar(grammar.name)
                }
                Kind::Snippets => {
                    let snippets = crate::snippets::parse_pack(&resource.content)?;
                    let count = snippets.len();
                    resources
                        .snippets
                        .extend(snippets.into_iter().map(|mut snippet| {
                            snippet.name = format!("{} ({key})", snippet.name);
                            snippet
                        }));
                    Value::Snippets(count)
                }
                Kind::Workflow => Value::Workflow(crate::workflow_center::WorkflowLibrary::parse(
                    resource.content.as_bytes(),
                )?),
                Kind::Skill => Value::Skill(resource.content.clone()),
                Kind::Profile => Value::Profile(Profile::parse(&resource.content)?),
                Kind::Widget => Value::Widget(Widget::parse(&resource.content)?),
                Kind::Readme => Value::Readme(resource.content.clone()),
                Kind::RoomBlueprint => Value::RoomBlueprint(
                    forge_room_core::transport::RoomBlueprint::parse(resource.content.as_bytes())?,
                ),
            };
            resources.assets.insert(
                key,
                Asset {
                    pack: pack.key(),
                    resource: resource.id.clone(),
                    label: resource.label.clone(),
                    digest: resource.sha256.clone(),
                    kind: resource.kind,
                    value,
                },
            );
        }
    }
    for (key, asset) in &resources.assets {
        if let Value::Profile(profile) = &asset.value
            && let Some(theme) = &profile.theme
            && !resources.theme_refs.contains_key(theme)
        {
            return Err(format!(
                "{key} needs theme {theme}; enable its resource before this profile"
            ));
        }
    }
    let syntax = crate::syntax::prepare_packs(&resources.grammars)?;
    Ok(Prepared { resources, syntax })
}
pub fn apply(prepared: Prepared) {
    *slot().write().expect("pack resources lock") = Arc::new(prepared.resources);
    crate::syntax::install(prepared.syntax);
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::*;
    #[test]
    fn disabling_resources_has_a_builtin_fallback_even_when_optional_grammar_rebuild_fails() {
        let prepared = disabled_with_syntax(Err("bad loose grammar linking".into()));
        assert!(prepared.resources.assets.is_empty());
        assert!(prepared.resources.grammars.is_empty());
        assert!(prepared.resources.snippets.is_empty());
        assert!(prepared.syntax.names.is_empty());
        assert_eq!(prepared.resources.warnings.len(), 1);
        assert!(
            prepared
                .resources
                .theme_refs
                .contains_key(crate::theme::DEFAULT_THEME)
        );
    }
    fn activation(pack: &packs::Pack) -> Activation {
        Activation {
            enabled: [(
                pack.id.clone(),
                packs::Selection {
                    version: pack.version,
                    resources: pack.resources.iter().map(|r| r.id.clone()).collect(),
                },
            )]
            .into_iter()
            .collect(),
            ..Activation::default()
        }
    }
    #[test]
    fn all_nine_resource_types_prepare_without_execution_or_global_publication() {
        let pack = author::starter();
        assert_eq!(validate_pack(&pack).unwrap().len(), 9);
        let before = Arc::as_ptr(&current());
        let catalog = [(pack.key(), pack.clone())].into_iter().collect();
        let prepared = prepare(&activation(&pack), &catalog).unwrap();
        assert_eq!(prepared.resources.assets.len(), 9);
        assert_eq!(prepared.resources.themes.len(), 1);
        assert_eq!(prepared.resources.snippets.len(), 1);
        assert!(
            prepared
                .syntax
                .names
                .iter()
                .any(|name| name == "Forge Notes (local.studio/resource-2)")
        );
        assert_eq!(Arc::as_ptr(&current()), before);
        assert!(matches!(
            prepared.resources.assets["local.studio/resource-4"].value,
            Value::Workflow(_)
        ));
        assert!(matches!(
            prepared.resources.assets["local.studio/resource-5"].value,
            Value::Skill(_)
        ));
        let disabled = prepare(&Activation::default(), &catalog).unwrap();
        assert!(disabled.resources.assets.is_empty());
        assert!(disabled.resources.grammars.is_empty());
        assert!(disabled.resources.snippets.is_empty());
    }
    #[test]
    fn semantic_errors_and_unselected_profile_theme_dependencies_fail_before_publication() {
        let mut pack = author::starter();
        let profile = Profile {
            theme: Some("local.studio/resource-1".into()),
            ..Profile::default()
        };
        pack.resources[5].content = serde_json::to_string(&profile).unwrap();
        pack.resources[5].sha256 =
            forge_room_core::workflows::digest(pack.resources[5].content.as_bytes());
        let catalog = [(pack.key(), pack.clone())].into_iter().collect();
        let mut selected = activation(&pack);
        assert!(prepare(&selected, &catalog).is_ok());
        selected
            .enabled
            .get_mut(&pack.id)
            .unwrap()
            .resources
            .remove("resource-1");
        assert!(
            prepare(&selected, &catalog)
                .err()
                .unwrap()
                .contains("needs theme")
        );
        pack.resources[2].content = "{\"Bad\":{\"prefix\":\"x\",\"body\":\"${1:script}\"}}".into();
        pack.resources[2].sha256 =
            forge_room_core::workflows::digest(pack.resources[2].content.as_bytes());
        assert!(validate_pack(&pack).is_err());
        let catalog = [(pack.key(), pack.clone())].into_iter().collect();
        assert!(prepare(&activation(&pack), &catalog).is_err());
    }
    #[test]
    fn conflicting_selected_grammar_extensions_are_rejected_as_one_transaction() {
        let mut first = author::starter();
        first.resources.retain(|r| r.kind == Kind::Grammar);
        let mut second = first.clone();
        second.id = "local.second".into();
        let catalog = [(first.key(), first.clone()), (second.key(), second.clone())]
            .into_iter()
            .collect();
        let mut selected = activation(&first);
        selected.enabled.extend(activation(&second).enabled);
        assert!(prepare(&selected, &catalog).is_err());
    }
}
