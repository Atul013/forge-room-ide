//! Skills are inert, exact text snapshots. Metadata never grants capabilities.
use super::{Kind, Pack, Resource, Version, API};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillDocument {
    pub name: String,
    pub description: String,
    pub content: String,
}
impl SkillDocument {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > Kind::Skill.max_bytes() {
            return Err("SKILL.md exceeds 32 KiB; no bytes imported".into());
        }
        let content = std::str::from_utf8(bytes).map_err(|_| "SKILL.md must be UTF-8")?;
        Resource::new(
            "instructions".into(),
            "Skill instructions".into(),
            Kind::Skill,
            content.into(),
        )?;
        let mut lines = content.lines();
        if lines.next() != Some("---") {
            return Err("SKILL.md needs frontmatter with name and description".into());
        }
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;
        let mut closed = false;
        let mut block: Option<(&str, bool, usize)> = None;
        for line in lines {
            if line == "---" {
                closed = true;
                break;
            }
            if line.starts_with([' ', '\t']) {
                if let Some((key, folded, indent)) = block.as_mut() {
                    let leading = line
                        .chars()
                        .take_while(|ch| *ch == ' ' || *ch == '\t')
                        .count();
                    let value = if *key == "name" {
                        name.as_mut().unwrap()
                    } else {
                        description.as_mut().unwrap()
                    };
                    if line.trim().is_empty() {
                        if !*folded && !value.ends_with('\n') {
                            value.push('\n');
                        }
                    } else if leading >= *indent {
                        if !value.is_empty() {
                            value.push(if *folded { ' ' } else { '\n' });
                        }
                        value.push_str(line.trim());
                    }
                }
                continue;
            }
            block = None;
            let Some((key, raw_value)) = line.split_once(':') else {
                continue;
            };
            // Only root-level keys can supply the two displayed identity fields.
            if key.trim() != key {
                continue;
            }
            let target = match key {
                "name" => &mut name,
                "description" => &mut description,
                _ => continue,
            };
            if target.is_some() {
                return Err(format!("Duplicate Skill metadata: {key}"));
            }
            let value = raw_value.trim();
            if matches!(value, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
                let (folded, indent) = (
                    value.starts_with('>'),
                    line.len() - line.trim_start().len() + 2,
                );
                block = Some((key, folded, indent));
                *target = Some(String::new());
            } else if value.is_empty() {
                return Err("Use nonempty name and description metadata".into());
            } else {
                let scalar = if (value.starts_with('\'') && value.ends_with('\''))
                    || (value.starts_with('"') && value.ends_with('"'))
                {
                    &value[1..value.len() - 1]
                } else {
                    value
                };
                *target = Some(scalar.to_owned());
            }
        }
        if !closed {
            return Err("Unclosed Skill frontmatter".into());
        }
        let name = name.ok_or("Missing Skill name")?;
        let description = description.ok_or("Missing Skill description")?;
        crate::workflows::text(&name, 160, true)?;
        crate::workflows::text(&description, 4000, true)?;
        Ok(Self {
            name,
            description,
            content: content.into(),
        })
    }

    pub fn pack(
        &self,
        id: &str,
        version: Version,
        publisher: &str,
        license: &str,
        source: &str,
    ) -> Result<Pack, String> {
        let pack = Pack {
            format: 1, api: API, id: id.into(), version,
            name: self.name.clone(), publisher: publisher.into(), summary: self.description.clone(),
            license: license.into(), license_text: String::new(), source: source.into(),
            readme: "Imported SKILL.md snapshot. Publisher, source and license are unverified claims. Companion scripts and references are not imported or executed.".into(),
            dependencies: vec![],
            resources: vec![Resource::new("instructions".into(), self.name.clone(), Kind::Skill, self.content.clone())?],
        };
        pack.validate()?;
        Ok(pack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skill_import_preserves_exact_bytes_and_claims() {
        let bytes = b"---\r\nname: review\r\ndescription: Review evidence\r\nallowed-tools: shell\r\n---\r\nRun nothing automatically.\r\n";
        let doc = SkillDocument::parse(bytes).unwrap();
        let pack = doc
            .pack(
                "local.review",
                Version([1, 0, 0]),
                "Author claim",
                "UNSPECIFIED",
                "Local file claim",
            )
            .unwrap();
        assert_eq!(pack.resources[0].content.as_bytes(), bytes);
        assert_eq!(pack.resources[0].kind, Kind::Skill);
        assert!(pack.dependencies.is_empty());
        assert_eq!(
            Pack::parse(&serde_json::to_vec(&pack).unwrap()).unwrap(),
            pack
        );
    }
    #[test]
    fn skill_metadata_supports_multiline_and_ignores_nested_claims() {
        let doc = SkillDocument::parse(b"---\nmetadata:\n  name: spoof\nname: review\ndescription: >\n  Review evidence\n  before accepting.\n---\nInstructions").unwrap();
        assert_eq!(doc.name, "review");
        assert_eq!(doc.description, "Review evidence before accepting.");
        assert!(
            SkillDocument::parse(b"---\nmetadata:\n  name: spoof\ndescription: d\n---").is_err()
        );
        let literal = SkillDocument::parse(
            b"---\nname: review\ndescription: |-\n  First line.\n  Second line.\n---\nBody",
        )
        .unwrap();
        assert_eq!(literal.description, "First line.\nSecond line.");
    }
    #[test]
    fn skill_import_refuses_ambiguous_or_unbounded_metadata() {
        for bytes in [
            b"no header".as_slice(),
            b"---\nname: a\nname: b\ndescription: d\n---",
            b"---\nname: a\ndescription: |\n---",
            b"---\nname: a\ndescription: d",
            b"---\nname: a\ndescription: d\n---\n\0",
        ] {
            assert!(SkillDocument::parse(bytes).is_err());
        }
        assert!(SkillDocument::parse(&vec![b'x'; 32769]).is_err());
        assert!(SkillDocument::parse(&[255]).is_err());
    }
}
