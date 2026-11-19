//! Prepared resource snapshots contain no grants, project paths, executable commands or network clients.
use forge_room_core::packs::{Activation, Catalog, Kind, Profile, Widget};
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
pub struct Prepared {
    pub resources: Resources,
    syntax: crate::syntax::Prepared,
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
        for resource in &pack.resources {
            if !selection.resources.contains(&resource.id) {
                continue;
            }
            let key = format!("{id}/{}", resource.id);
            let value = match resource.kind {
                Kind::Theme => {
                    let mut theme = crate::theme::parse_import(&resource.content)?;
                    let actual = format!("pack-{}-{}", &pack.digest()?[..16], resource.id);
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
        if let Value::Profile(profile) = &asset.value {
            if let Some(theme) = &profile.theme {
                if !resources.theme_refs.contains_key(theme) {
                    return Err(format!(
                        "{key} needs theme {theme}; enable its resource before this profile"
                    ));
                }
            }
        }
    }
    let syntax = crate::syntax::prepare_packs(&resources.grammars)?;
    Ok(Prepared { resources, syntax })
}
pub fn apply(prepared: Prepared) {
    *slot().write().expect("pack resources lock") = Arc::new(prepared.resources);
    crate::syntax::install(prepared.syntax);
}
