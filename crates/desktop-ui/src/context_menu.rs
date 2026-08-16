use shell_provider_protocol::{
    CommandDescriptor, MenuEnumeration, MenuInvocation, ValidationError, validate_command_tree,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuAccessibleNode {
    pub stable_id: String,
    pub name: String,
    pub role: &'static str,
    pub enabled: bool,
    pub focused: bool,
    pub has_submenu: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuModel {
    generation: u64,
    selection_fingerprint: String,
    commands: Vec<CommandDescriptor>,
    path: Vec<usize>,
    focused: Option<usize>,
    dismissed: bool,
    enrichment_complete: bool,
}

impl MenuModel {
    pub fn new(menu: MenuEnumeration) -> Result<Self, ValidationError> {
        validate_command_tree(&menu.commands)?;
        let focused = menu.commands.iter().position(|command| command.enabled);
        Ok(Self {
            generation: menu.generation,
            selection_fingerprint: menu.selection_fingerprint,
            commands: menu.commands,
            path: Vec::new(),
            focused,
            dismissed: false,
            enrichment_complete: menu.optional_enrichment_complete,
        })
    }

    pub fn commands(&self) -> &[CommandDescriptor] {
        current_commands(&self.commands, &self.path)
    }

    pub fn move_focus(&mut self, delta: i32) {
        let commands = self.commands();
        if commands.is_empty() {
            self.focused = None;
            return;
        }
        let start = self.focused.unwrap_or(0) as i32;
        let next = (1..=commands.len()).find_map(|offset| {
            let index = (start + delta * offset as i32).rem_euclid(commands.len() as i32) as usize;
            commands[index].enabled.then_some(index)
        });
        self.focused = next;
    }

    pub fn enter_submenu(&mut self) -> bool {
        let Some(index) = self.focused else {
            return false;
        };
        if self
            .commands()
            .get(index)
            .is_none_or(|command| command.children.is_empty())
        {
            return false;
        }
        self.path.push(index);
        self.focused = self.commands().iter().position(|command| command.enabled);
        true
    }

    pub fn leave_submenu(&mut self) -> bool {
        let Some(parent) = self.path.pop() else {
            return false;
        };
        self.focused = Some(parent);
        true
    }

    pub fn invoke_focused(&self) -> Option<MenuInvocation> {
        self.invoke(self.focused?)
    }

    pub fn invoke(&self, index: usize) -> Option<MenuInvocation> {
        let command = self.commands().get(index)?;
        (command.enabled && command.children.is_empty()).then(|| MenuInvocation {
            generation: self.generation,
            selection_fingerprint: self.selection_fingerprint.clone(),
            token: command.id.0.clone(),
        })
    }

    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub fn is_dismissed(&self) -> bool {
        self.dismissed
    }

    pub fn enrichment_complete(&self) -> bool {
        self.enrichment_complete
    }

    pub fn accessible_nodes(&self) -> Vec<MenuAccessibleNode> {
        self.commands()
            .iter()
            .enumerate()
            .map(|(index, command)| MenuAccessibleNode {
                stable_id: format!("context-menu:{}:{}", self.generation, command.id.0),
                name: command.label.clone(),
                role: "menuitem",
                enabled: command.enabled,
                focused: self.focused == Some(index),
                has_submenu: !command.children.is_empty(),
            })
            .collect()
    }
}

fn current_commands<'a>(root: &'a [CommandDescriptor], path: &[usize]) -> &'a [CommandDescriptor] {
    let mut commands = root;
    for index in path {
        let Some(command) = commands.get(*index) else {
            return &[];
        };
        commands = &command.children;
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_provider_protocol::{CommandId, CommandRisk};

    fn command(id: &str, enabled: bool) -> CommandDescriptor {
        CommandDescriptor {
            id: CommandId(id.into()),
            label: id.into(),
            enabled,
            risk: CommandRisk::Normal,
            children: Vec::new(),
        }
    }

    #[test]
    fn pointer_keyboard_and_accessibility_share_typed_invocation() {
        let model = MenuModel::new(MenuEnumeration {
            generation: 7,
            selection_fingerprint: "selected".into(),
            commands: vec![command("disabled", false), command("open", true)],
            optional_enrichment_complete: false,
        })
        .unwrap();
        assert!(model.invoke(0).is_none());
        assert_eq!(model.invoke(1), model.invoke_focused());
        let nodes = model.accessible_nodes();
        assert_eq!(nodes[1].role, "menuitem");
        assert!(nodes[1].focused);
    }
}
