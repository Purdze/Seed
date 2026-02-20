use std::sync::Arc;

use pumpkin::command::args::players::PlayersArgumentConsumer;
use pumpkin::command::args::simple::SimpleArgConsumer;
use pumpkin::command::args::{ConsumedArgs, FindArg};
use pumpkin::command::dispatcher::CommandError;
use pumpkin::command::tree::CommandTree;
use pumpkin::command::tree::builder::{argument, literal};
use pumpkin::command::{CommandExecutor, CommandResult, CommandSender};
use pumpkin::server::Server;
use pumpkin_util::text::TextComponent;
use tokio::sync::RwLock;

use crate::store::PermissionStore;

const ARG_GROUP_NAME: &str = "name";
const ARG_PERMISSION: &str = "permission";
const ARG_TARGET: &str = "target";

fn save_store(store: &PermissionStore) -> Result<(), CommandError> {
    store
        .save()
        .map_err(|e| CommandError::CommandFailed(TextComponent::text(e)))
}

fn format_list(items: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let collected: Vec<_> = items.into_iter().map(|s| s.as_ref().to_string()).collect();
    if collected.is_empty() {
        "(none)".to_string()
    } else {
        collected.join(", ")
    }
}

fn format_sorted_list(items: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut collected: Vec<_> = items.into_iter().map(|s| s.as_ref().to_string()).collect();
    collected.sort();
    if collected.is_empty() {
        "(none)".to_string()
    } else {
        collected.join(", ")
    }
}

struct GroupCreateExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for GroupCreateExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let name = SimpleArgConsumer::find_arg(args, ARG_GROUP_NAME)?;
            let mut store = store.write().await;
            if store.groups.contains_key(name) {
                sender
                    .send_message(TextComponent::text(format!(
                        "Group '{name}' already exists"
                    )))
                    .await;
                return Ok(0);
            }
            store.groups.insert(
                name.to_string(),
                crate::store::Group {
                    permissions: Vec::new(),
                    inheritance: Vec::new(),
                },
            );
            save_store(&store)?;
            sender
                .send_message(TextComponent::text(format!("Created group '{name}'")))
                .await;
            Ok(1)
        })
    }
}

struct GroupDeleteExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for GroupDeleteExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let name = SimpleArgConsumer::find_arg(args, ARG_GROUP_NAME)?;
            if name == "default" {
                sender
                    .send_message(TextComponent::text("Cannot delete the 'default' group"))
                    .await;
                return Ok(0);
            }
            let mut store = store.write().await;
            if store.groups.remove(name).is_none() {
                sender
                    .send_message(TextComponent::text(format!("Group '{name}' not found")))
                    .await;
                return Ok(0);
            }
            save_store(&store)?;
            sender
                .send_message(TextComponent::text(format!("Deleted group '{name}'")))
                .await;
            Ok(1)
        })
    }
}

#[derive(Clone, Copy)]
enum GroupPermOp {
    Add,
    Remove,
}

struct GroupPermExecutor(Arc<RwLock<PermissionStore>>, GroupPermOp);

impl CommandExecutor for GroupPermExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        let op = self.1;
        Box::pin(async move {
            let group_name = SimpleArgConsumer::find_arg(args, ARG_GROUP_NAME)?;
            let permission = SimpleArgConsumer::find_arg(args, ARG_PERMISSION)?;
            let mut store = store.write().await;
            let Some(group) = store.groups.get_mut(group_name) else {
                sender
                    .send_message(TextComponent::text(format!(
                        "Group '{group_name}' not found"
                    )))
                    .await;
                return Ok(0);
            };
            let perm_str = permission.to_string();
            match op {
                GroupPermOp::Add => {
                    if group.permissions.contains(&perm_str) {
                        sender
                            .send_message(TextComponent::text(format!(
                                "Group '{group_name}' already has permission '{permission}'"
                            )))
                            .await;
                        return Ok(0);
                    }
                    group.permissions.push(perm_str);
                    save_store(&store)?;
                    sender
                        .send_message(TextComponent::text(format!(
                            "Added permission '{permission}' to group '{group_name}'"
                        )))
                        .await;
                }
                GroupPermOp::Remove => {
                    let Some(pos) = group.permissions.iter().position(|p| p == &perm_str) else {
                        sender
                            .send_message(TextComponent::text(format!(
                                "Group '{group_name}' does not have permission '{permission}'"
                            )))
                            .await;
                        return Ok(0);
                    };
                    group.permissions.remove(pos);
                    save_store(&store)?;
                    sender
                        .send_message(TextComponent::text(format!(
                            "Removed permission '{permission}' from group '{group_name}'"
                        )))
                        .await;
                }
            }
            Ok(1)
        })
    }
}

struct GroupInfoExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for GroupInfoExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let name = SimpleArgConsumer::find_arg(args, ARG_GROUP_NAME)?;
            let store = store.read().await;
            let Some(group) = store.groups.get(name) else {
                sender
                    .send_message(TextComponent::text(format!("Group '{name}' not found")))
                    .await;
                return Ok(0);
            };
            let perms = format_list(&group.permissions);
            let inheritance = format_list(&group.inheritance);
            let effective = format_sorted_list(store.resolve_group_permissions(name));
            sender
                .send_message(TextComponent::text(format!(
                    "Group '{name}':\n  Permissions: {perms}\n  Inheritance: {inheritance}\n  Effective: {effective}"
                )))
                .await;
            Ok(1)
        })
    }
}

struct GroupListExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for GroupListExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let store = store.read().await;
            let mut names: Vec<_> = store.groups.keys().map(|s| s.as_str()).collect();
            names.sort();
            sender
                .send_message(TextComponent::text(format!(
                    "Groups ({}): {}",
                    names.len(),
                    names.join(", ")
                )))
                .await;
            Ok(1)
        })
    }
}

struct PlayerSetGroupExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for PlayerSetGroupExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let players = PlayersArgumentConsumer::find_arg(args, ARG_TARGET)?;
            let group_name = SimpleArgConsumer::find_arg(args, ARG_GROUP_NAME)?;
            let mut store = store.write().await;
            if !store.groups.contains_key(group_name) {
                sender
                    .send_message(TextComponent::text(format!(
                        "Group '{group_name}' not found"
                    )))
                    .await;
                return Ok(0);
            }
            for player in players {
                let pd =
                    store.get_or_create_player(player.gameprofile.id, &player.gameprofile.name);
                pd.group = group_name.to_string();
                sender
                    .send_message(TextComponent::text(format!(
                        "Set {}'s group to '{group_name}'",
                        player.gameprofile.name
                    )))
                    .await;
            }
            save_store(&store)?;
            Ok(1)
        })
    }
}

#[derive(Clone, Copy)]
enum PlayerPermOp {
    AddExtra,
    RemoveExtra,
    Deny,
    Undeny,
}

impl PlayerPermOp {
    fn target_list(self, pd: &mut crate::store::PlayerData) -> &mut Vec<String> {
        match self {
            Self::AddExtra | Self::RemoveExtra => &mut pd.extra_permissions,
            Self::Deny | Self::Undeny => &mut pd.denied_permissions,
        }
    }

    fn is_add(self) -> bool {
        matches!(self, Self::AddExtra | Self::Deny)
    }

    fn action_past(self) -> &'static str {
        match self {
            Self::AddExtra => "Added permission",
            Self::RemoveExtra => "Removed permission",
            Self::Deny => "Denied permission",
            Self::Undeny => "Removed denial of",
        }
    }

    fn already_msg(self) -> &'static str {
        match self {
            Self::AddExtra => "already has extra permission",
            Self::Deny => "already has denied",
            Self::RemoveExtra => "does not have extra permission",
            Self::Undeny => "does not have denied",
        }
    }
}

struct PlayerPermExecutor(Arc<RwLock<PermissionStore>>, PlayerPermOp);

impl CommandExecutor for PlayerPermExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        let op = self.1;
        Box::pin(async move {
            let players = PlayersArgumentConsumer::find_arg(args, ARG_TARGET)?;
            let permission = SimpleArgConsumer::find_arg(args, ARG_PERMISSION)?;
            let perm_str = permission.to_string();
            let mut store = store.write().await;
            for player in players {
                let name = &player.gameprofile.name;
                let pd = store.get_or_create_player(player.gameprofile.id, name);
                let list = op.target_list(pd);
                if op.is_add() {
                    if list.contains(&perm_str) {
                        sender
                            .send_message(TextComponent::text(format!(
                                "{name} {} '{permission}'",
                                op.already_msg()
                            )))
                            .await;
                    } else {
                        list.push(perm_str.clone());
                        sender
                            .send_message(TextComponent::text(format!(
                                "{} '{permission}' for {name}",
                                op.action_past()
                            )))
                            .await;
                    }
                } else {
                    let Some(pos) = list.iter().position(|p| p == &perm_str) else {
                        sender
                            .send_message(TextComponent::text(format!(
                                "{name} {} '{permission}'",
                                op.already_msg()
                            )))
                            .await;
                        continue;
                    };
                    list.remove(pos);
                    sender
                        .send_message(TextComponent::text(format!(
                            "{} '{permission}' for {name}",
                            op.action_past()
                        )))
                        .await;
                }
            }
            save_store(&store)?;
            Ok(1)
        })
    }
}

struct PlayerInfoExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for PlayerInfoExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let players = PlayersArgumentConsumer::find_arg(args, ARG_TARGET)?;
            let store = store.read().await;
            for player in players {
                let uuid = player.gameprofile.id;
                let name = &player.gameprofile.name;
                let Some(pd) = store.players.get(&uuid) else {
                    sender
                        .send_message(TextComponent::text(format!(
                            "Player '{name}' ({uuid}): group=default (no custom data)"
                        )))
                        .await;
                    continue;
                };
                let extras = format_list(&pd.extra_permissions);
                let denied = format_list(&pd.denied_permissions);
                let mut effective: Vec<_> = store
                    .resolve_group_permissions(&pd.group)
                    .into_iter()
                    .collect();
                for ep in &pd.extra_permissions {
                    if !effective.contains(ep) {
                        effective.push(ep.clone());
                    }
                }
                let effective = format_sorted_list(effective);
                sender
                    .send_message(TextComponent::text(format!(
                        "Player '{name}' ({uuid}):\n  Group: {}\n  Extra: {extras}\n  Denied: {denied}\n  Effective: {effective}",
                        pd.group
                    )))
                    .await;
            }
            Ok(1)
        })
    }
}

struct ReloadExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for ReloadExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let mut store = store.write().await;
            let data_folder = store.data_folder.clone();
            match PermissionStore::load(data_folder) {
                Ok(new_store) => {
                    *store = new_store;
                    sender
                        .send_message(TextComponent::text("Seed configuration reloaded"))
                        .await;
                    Ok(1)
                }
                Err(e) => {
                    sender
                        .send_message(TextComponent::text(format!("Reload failed: {e}")))
                        .await;
                    Ok(0)
                }
            }
        })
    }
}

struct SaveExecutor(Arc<RwLock<PermissionStore>>);

impl CommandExecutor for SaveExecutor {
    fn execute<'a>(
        &'a self,
        sender: &'a CommandSender,
        _server: &'a Server,
        _args: &'a ConsumedArgs<'a>,
    ) -> CommandResult<'a> {
        let store = self.0.clone();
        Box::pin(async move {
            let store = store.read().await;
            match store.save() {
                Ok(()) => {
                    sender
                        .send_message(TextComponent::text("Seed configuration saved"))
                        .await;
                    Ok(1)
                }
                Err(e) => {
                    sender
                        .send_message(TextComponent::text(format!("Save failed: {e}")))
                        .await;
                    Ok(0)
                }
            }
        })
    }
}

pub fn build_command_tree(store: Arc<RwLock<PermissionStore>>) -> CommandTree {
    CommandTree::new(["seed"], "Seed permission plugin commands")
        .then(
            literal("group")
                .then(
                    literal("create").then(
                        argument(ARG_GROUP_NAME, SimpleArgConsumer)
                            .execute(GroupCreateExecutor(store.clone())),
                    ),
                )
                .then(
                    literal("delete").then(
                        argument(ARG_GROUP_NAME, SimpleArgConsumer)
                            .execute(GroupDeleteExecutor(store.clone())),
                    ),
                )
                .then(
                    literal("addperm").then(
                        argument(ARG_GROUP_NAME, SimpleArgConsumer).then(
                            argument(ARG_PERMISSION, SimpleArgConsumer)
                                .execute(GroupPermExecutor(store.clone(), GroupPermOp::Add)),
                        ),
                    ),
                )
                .then(
                    literal("removeperm").then(
                        argument(ARG_GROUP_NAME, SimpleArgConsumer).then(
                            argument(ARG_PERMISSION, SimpleArgConsumer)
                                .execute(GroupPermExecutor(store.clone(), GroupPermOp::Remove)),
                        ),
                    ),
                )
                .then(
                    literal("info").then(
                        argument(ARG_GROUP_NAME, SimpleArgConsumer)
                            .execute(GroupInfoExecutor(store.clone())),
                    ),
                )
                .then(literal("list").execute(GroupListExecutor(store.clone()))),
        )
        .then(
            literal("player")
                .then(
                    literal("setgroup").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer).then(
                            argument(ARG_GROUP_NAME, SimpleArgConsumer)
                                .execute(PlayerSetGroupExecutor(store.clone())),
                        ),
                    ),
                )
                .then(
                    literal("addperm").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer)
                            .then(argument(ARG_PERMISSION, SimpleArgConsumer).execute(
                                PlayerPermExecutor(store.clone(), PlayerPermOp::AddExtra),
                            )),
                    ),
                )
                .then(
                    literal("removeperm").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer).then(
                            argument(ARG_PERMISSION, SimpleArgConsumer).execute(
                                PlayerPermExecutor(store.clone(), PlayerPermOp::RemoveExtra),
                            ),
                        ),
                    ),
                )
                .then(
                    literal("deny").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer).then(
                            argument(ARG_PERMISSION, SimpleArgConsumer)
                                .execute(PlayerPermExecutor(store.clone(), PlayerPermOp::Deny)),
                        ),
                    ),
                )
                .then(
                    literal("undeny").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer).then(
                            argument(ARG_PERMISSION, SimpleArgConsumer)
                                .execute(PlayerPermExecutor(store.clone(), PlayerPermOp::Undeny)),
                        ),
                    ),
                )
                .then(
                    literal("info").then(
                        argument(ARG_TARGET, PlayersArgumentConsumer)
                            .execute(PlayerInfoExecutor(store.clone())),
                    ),
                ),
        )
        .then(literal("reload").execute(ReloadExecutor(store.clone())))
        .then(literal("save").execute(SaveExecutor(store)))
}
