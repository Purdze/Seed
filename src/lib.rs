#![allow(clippy::async_yields_async, clippy::new_without_default)]

mod commands;
mod handler;
mod store;

use std::sync::Arc;

use pumpkin::plugin::Context;
use pumpkin_api_macros::{plugin_impl, plugin_method};
use tokio::sync::RwLock;

use store::PermissionStore;

static mut STORE: Option<Arc<RwLock<PermissionStore>>> = None;

#[plugin_method]
fn on_load(&mut self, server: Arc<Context>) -> Result<(), String> {
    let data_folder = server.get_data_folder();
    let store = PermissionStore::load(data_folder)?;
    let store = Arc::new(RwLock::new(store));

    unsafe { STORE = Some(store.clone()) };

    let handler = Arc::new(handler::SeedPermissionHandler {
        store: store.clone(),
    });
    server
        .register_event::<pumpkin::plugin::api::events::player::player_permission_check::PlayerPermissionCheckEvent, _>(
            handler,
            pumpkin::plugin::EventPriority::Normal,
            true,
        )
        .await;

    let chat_handler = Arc::new(handler::SeedChatHandler {
        store: store.clone(),
    });
    server
        .register_event::<pumpkin::plugin::api::events::player::player_chat::PlayerChatEvent, _>(
            chat_handler,
            pumpkin::plugin::EventPriority::Normal,
            true,
        )
        .await;

    let tree = commands::build_command_tree(store);
    server.register_command(tree, "seed:admin").await;

    server.log("Seed v1.0.0 loaded!");
    Ok(())
}

#[plugin_method]
fn on_unload(&mut self, server: Arc<Context>) -> Result<(), String> {
    let store = unsafe { STORE.take() };
    if let Some(store) = store {
        if let Err(e) = store.blocking_read().save() {
            server.log(format!("Failed to save on unload: {e}"));
        }
    }
    server.log("Seed unloaded!");
    Ok(())
}

#[plugin_impl]
pub struct SeedPlugin;

impl SeedPlugin {
    pub fn new() -> Self {
        Self
    }
}
