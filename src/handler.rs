use std::sync::Arc;

use pumpkin::plugin::api::events::Cancellable;
use pumpkin::plugin::api::events::player::player_chat::PlayerChatEvent;
use pumpkin::plugin::api::events::player::player_permission_check::PlayerPermissionCheckEvent;
use pumpkin::plugin::{BoxFuture, EventHandler};
use pumpkin::server::Server;
use pumpkin_util::text::TextComponent;
use tokio::sync::RwLock;

use crate::store::PermissionStore;

pub struct SeedPermissionHandler {
    pub store: Arc<RwLock<PermissionStore>>,
}

impl EventHandler<PlayerPermissionCheckEvent> for SeedPermissionHandler {
    fn handle_blocking<'a>(
        &'a self,
        _server: &'a Arc<Server>,
        event: &'a mut PlayerPermissionCheckEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let store = self.store.read().await;
            if let Some(result) =
                store.check_permission(&event.player.gameprofile.id, &event.permission)
            {
                event.result = result;
            }
        })
    }
}

pub struct SeedChatHandler {
    pub store: Arc<RwLock<PermissionStore>>,
}

impl EventHandler<PlayerChatEvent> for SeedChatHandler {
    fn handle_blocking<'a>(
        &'a self,
        server: &'a Arc<Server>,
        event: &'a mut PlayerChatEvent,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let prefix = {
                let store = self.store.read().await;
                store.resolve_prefix(&event.player.gameprofile.id)
            };

            let Some(prefix) = prefix else {
                return;
            };

            let prefix_formatted = prefix.replace('&', "§");
            let name = &event.player.gameprofile.name;
            let message = &event.message;
            let formatted = format!("{prefix_formatted} <{name}> {message}");
            let component = TextComponent::from_legacy_string(&formatted);

            event.set_cancelled(true);

            for player in server.get_all_players() {
                player.send_system_message(&component).await;
            }
        })
    }
}
