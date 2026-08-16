use serde::{Deserialize, Serialize};

use crate::{CommandDescriptor, MAX_COLLECTION_ITEMS, Validate, ValidationError};

pub const MAX_MENU_DEPTH: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MenuContext {
    pub selection_fingerprint: String,
    pub selection_count: usize,
    pub background: bool,
    pub can_open: bool,
    pub can_rename: bool,
    pub can_delete: bool,
    pub can_show_properties: bool,
}

impl Validate for MenuContext {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.selection_fingerprint.trim().is_empty() {
            return Err(ValidationError::Empty("menu.selection_fingerprint"));
        }
        if self.selection_fingerprint.len() > crate::MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("menu.selection_fingerprint"));
        }
        if self.selection_count > MAX_COLLECTION_ITEMS {
            return Err(ValidationError::CollectionTooLarge("menu.selection"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MenuEnumeration {
    pub generation: u64,
    pub selection_fingerprint: String,
    pub commands: Vec<CommandDescriptor>,
    pub optional_enrichment_complete: bool,
}

impl Validate for MenuEnumeration {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.selection_fingerprint.trim().is_empty() {
            return Err(ValidationError::Empty("menu.selection_fingerprint"));
        }
        validate_command_tree(&self.commands)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MenuInvocation {
    pub generation: u64,
    pub selection_fingerprint: String,
    pub token: String,
}

impl Validate for MenuInvocation {
    fn validate(&self) -> Result<(), ValidationError> {
        for (field, value) in [
            (
                "menu.selection_fingerprint",
                self.selection_fingerprint.as_str(),
            ),
            ("menu.token", self.token.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ValidationError::Empty(field));
            }
            if value.len() > crate::MAX_TEXT_BYTES {
                return Err(ValidationError::TextTooLong(field));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MenuInvocationResult {
    pub command_id: String,
}

pub fn validate_command_tree(commands: &[CommandDescriptor]) -> Result<(), ValidationError> {
    fn visit(
        commands: &[CommandDescriptor],
        depth: usize,
        count: &mut usize,
    ) -> Result<(), ValidationError> {
        if depth > MAX_MENU_DEPTH {
            return Err(ValidationError::OutOfRange("menu.depth"));
        }
        for command in commands {
            *count = count.saturating_add(1);
            if *count > MAX_COLLECTION_ITEMS {
                return Err(ValidationError::CollectionTooLarge("menu.commands"));
            }
            command.validate()?;
            visit(&command.children, depth + 1, count)?;
        }
        Ok(())
    }
    let mut count = 0;
    visit(commands, 1, &mut count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandId, CommandRisk};

    fn command(id: &str) -> CommandDescriptor {
        CommandDescriptor {
            id: CommandId(id.into()),
            label: id.into(),
            enabled: true,
            risk: CommandRisk::Normal,
            children: Vec::new(),
        }
    }

    #[test]
    fn menu_round_trip_and_depth_limit_are_deterministic() {
        let menu = MenuEnumeration {
            generation: 1,
            selection_fingerprint: "selection".into(),
            commands: vec![command("open")],
            optional_enrichment_complete: false,
        };
        let json = serde_json::to_string(&menu).unwrap();
        assert_eq!(
            serde_json::from_str::<MenuEnumeration>(&json).unwrap(),
            menu
        );
        let mut nested = command("leaf");
        for index in 0..MAX_MENU_DEPTH {
            let mut parent = command(&format!("level-{index}"));
            parent.children.push(nested);
            nested = parent;
        }
        assert!(matches!(
            validate_command_tree(&[nested]),
            Err(ValidationError::OutOfRange("menu.depth"))
        ));
    }
}
