use std::collections::BTreeMap;

use platform_win::common::desktop::{DesktopOrigin as PlatformOrigin, OwnedDesktopEntry};
use shell_core::ShellItemId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopOrigin {
    User,
    Public,
    Fixed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemCapabilities {
    pub folder: bool,
    pub association: bool,
    pub hidden: bool,
    pub system: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IconDescriptor {
    pub source_key: String,
    pub resource_index: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopItem {
    pub identity: ShellItemId,
    pub display_name: String,
    pub origin: DesktopOrigin,
    pub activation_token: String,
    pub icon: IconDescriptor,
    pub capabilities: ItemCapabilities,
}

impl TryFrom<OwnedDesktopEntry> for DesktopItem {
    type Error = shell_core::IdentityError;

    fn try_from(entry: OwnedDesktopEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            identity: ShellItemId::new(entry.stable_identity)?,
            display_name: entry.display_name,
            origin: match entry.origin {
                PlatformOrigin::User => DesktopOrigin::User,
                PlatformOrigin::Public => DesktopOrigin::Public,
            },
            activation_token: entry.canonical_path.to_string_lossy().into_owned(),
            icon: IconDescriptor {
                source_key: entry.canonical_path.to_string_lossy().into_owned(),
                resource_index: None,
            },
            capabilities: ItemCapabilities {
                folder: entry.folder,
                association: !entry.folder,
                hidden: entry.hidden,
                system: entry.system,
            },
        })
    }
}

pub fn merge_desktop_items(
    user: impl IntoIterator<Item = DesktopItem>,
    public: impl IntoIterator<Item = DesktopItem>,
    show_hidden: bool,
    show_system: bool,
) -> Vec<DesktopItem> {
    let mut merged = BTreeMap::new();
    for item in user.into_iter().chain(public) {
        if (!show_hidden && item.capabilities.hidden) || (!show_system && item.capabilities.system)
        {
            continue;
        }
        merged.entry(item.identity.clone()).or_insert(item);
    }
    merged.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, name: &str, origin: DesktopOrigin) -> DesktopItem {
        DesktopItem {
            identity: ShellItemId::new(id).unwrap(),
            display_name: name.into(),
            origin,
            activation_token: format!("token:{id}"),
            icon: IconDescriptor {
                source_key: id.into(),
                resource_index: None,
            },
            capabilities: ItemCapabilities {
                folder: true,
                association: false,
                hidden: false,
                system: false,
            },
        }
    }

    #[test]
    fn same_name_different_identity_stays_distinct() {
        let merged = merge_desktop_items(
            [item("user:a", "同名", DesktopOrigin::User)],
            [item("public:a", "同名", DesktopOrigin::Public)],
            false,
            false,
        );
        assert_eq!(merged.len(), 2);
        assert_ne!(merged[0].identity, merged[1].identity);
    }

    #[test]
    fn same_identity_deduplicates_without_using_display_name() {
        let merged = merge_desktop_items(
            [item("stable", "舊名稱", DesktopOrigin::User)],
            [item("stable", "新名稱", DesktopOrigin::Public)],
            false,
            false,
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].origin, DesktopOrigin::User);
    }

    #[test]
    fn unicode_hidden_and_system_rules_are_independent() {
        let mut hidden = item("hidden", "隱藏.txt", DesktopOrigin::User);
        hidden.capabilities.hidden = true;
        let mut system = item("system", "系統", DesktopOrigin::Public);
        system.capabilities.system = true;
        assert!(merge_desktop_items([hidden.clone()], [system.clone()], false, false).is_empty());
        assert_eq!(
            merge_desktop_items([hidden], [system], true, false).len(),
            1
        );
    }
}
