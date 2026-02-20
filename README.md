# Seed

A permission plugin for [Pumpkin MC](https://github.com/Pumpkin-MC/Pumpkin). Pumpkin's built-in permission system only stores player permissions in memory (lost on restart) and has no group/role system. Seed adds persistent TOML-based storage with groups, inheritance, and per-player overrides.

## Features

- **Persistent storage** - Permissions survive server restarts via TOML files
- **Group system** - Define groups with sets of permissions (e.g. `default`, `moderator`, `admin`)
- **Inheritance** - Groups can inherit permissions from other groups
- **Per-player overrides** - Grant extra permissions or deny specific ones per player
- **Wildcard support** - Use `*` to grant all permissions

## Installation

1. Build the plugin with `cargo build --release`
2. Copy `target/release/seed.dll` (Windows) or `target/release/libseed.so` (Linux) into your Pumpkin server's `plugins/` directory
3. Start the server

On first load, Seed creates a `plugins/seed/` folder with default configuration files.

## Configuration

### `plugins/seed/groups.toml`

Defines permission groups. Each group has a list of permissions and can inherit from other groups.

```toml
[default]
permissions = ["minecraft:command.help", "minecraft:command.list"]
inheritance = []

[moderator]
permissions = ["minecraft:command.kick", "minecraft:command.ban"]
inheritance = ["default"]

[admin]
permissions = ["*"]
inheritance = ["moderator"]
```

### `plugins/seed/players.toml`

Stores per-player group assignments and permission overrides.

```toml
[players."550e8400-e29b-41d4-a716-446655440000"]
username = "Steve"
group = "moderator"
extra_permissions = ["some:custom.perm"]
denied_permissions = ["minecraft:command.ban"]
```

## How Permissions Resolve

When a permission is checked for a player, Seed evaluates in this order:

1. **Denied** - If the permission is in the player's `denied_permissions`, it is blocked
2. **Extra** - If the permission is in the player's `extra_permissions`, it is granted
3. **Group chain** - Walk the player's group and its inheritance tree; if the permission (or `*`) is found, it is granted
4. **Default** - If none of the above match, Seed does not interfere and Pumpkin's default behavior applies

Players not in the store are treated as members of the `default` group.

## Commands

All commands are under `/seed` and require the `seed:admin` permission.

### Group Management

| Command | Description |
|---|---|
| `/seed group create <name>` | Create a new empty group |
| `/seed group delete <name>` | Delete a group (cannot delete `default`) |
| `/seed group addperm <group> <permission>` | Add a permission to a group |
| `/seed group removeperm <group> <permission>` | Remove a permission from a group |
| `/seed group info <group>` | Show a group's permissions, inheritance, and effective permissions |
| `/seed group list` | List all groups |

### Player Management

| Command | Description |
|---|---|
| `/seed player setgroup <player> <group>` | Assign a player to a group |
| `/seed player addperm <player> <permission>` | Grant an extra permission to a player |
| `/seed player removeperm <player> <permission>` | Remove an extra permission from a player |
| `/seed player deny <player> <permission>` | Deny a specific permission for a player (overrides group) |
| `/seed player undeny <player> <permission>` | Remove a denied permission from a player |
| `/seed player info <player>` | Show a player's group, extras, denials, and effective permissions |

### Utility

| Command | Description |
|---|---|
| `/seed reload` | Reload configuration from disk |
| `/seed save` | Force save configuration to disk |

## Examples

```
/seed group create vip
/seed group addperm vip minecraft:command.fly
/seed group addperm vip minecraft:command.gamemode

/seed player setgroup Steve vip
/seed player deny Steve minecraft:command.gamemode
/seed player info Steve
```

This creates a `vip` group with fly and gamemode permissions, assigns Steve to it, then denies gamemode specifically for Steve. Steve can fly but cannot change gamemode.
