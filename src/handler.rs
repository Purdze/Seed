use std::sync::Arc;

use pumpkin::plugin::api::events::player::player_permission_check::PlayerPermissionCheckEvent;
use pumpkin::plugin::{BoxFuture, EventHandler};
use pumpkin::server::Server;
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
