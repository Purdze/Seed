# Seed

A permission plugin for [Pumpkin MC](https://github.com/Pumpkin-MC/Pumpkin). Pumpkin's built-in permission system only stores player permissions in memory (lost on restart) and has no group/role system. Seed adds persistent TOML-based storage with groups, inheritance, and per-player overrides.

## Features

- **Persistent storage** - Permissions survive server restarts via TOML files
- **Group system** - Define groups with sets of permissions (e.g. `default`, `moderator`, `admin`)
- **Inheritance** - Groups can inherit permissions from other groups
- **Per-player overrides** - Grant extra permissions or deny specific ones per player
- **Wildcard support** - Use `*` to grant all permissions
- **Chat prefixes** - Configurable per-group and per-player chat prefixes with color code support (`&` notation)

## Installation

1. Build the plugin with `cargo build --release`
2. Copy `target/release/seed.dll` (Windows) or `target/release/libseed.so` (Linux) into your Pumpkin server's `plugins/` directory
3. Start the server

On first load, Seed creates a `plugins/seed/` folder with a `default` group. All `/seed` commands require the `seed:admin` permission, so only the server console can manage permissions initially. Use the console to create groups and assign players as needed.

## Configuration

### `plugins/seed/groups.toml`

Defines permission groups. Each group has a list of permissions and can inherit from other groups. Only the `default` group is created on first load:

```toml
[default]
permissions = ["minecraft:command.help", "minecraft:command.list"]
inheritance = []
```

You can add more groups via commands or by editing the file directly. For example, a typical setup:

```toml
[default]
permissions = ["minecraft:command.help", "minecraft:command.list"]
inheritance = []

[moderator]
permissions = ["minecraft:command.kick", "minecraft:command.ban"]
inheritance = ["default"]
prefix = "&9[Mod]"

[admin]
permissions = ["seed:admin", "*"]
inheritance = ["moderator"]
prefix = "&c[Admin]"
```

Giving a group `seed:admin` allows its members to use `/seed` commands in-game.

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
| `/seed group info <group>` | Show a group's permissions, inheritance, prefix, and effective permissions |
| `/seed group setprefix <group> <prefix>` | Set the chat prefix for a group (supports `&` color codes) |
| `/seed group clearprefix <group>` | Remove the chat prefix from a group |
| `/seed group list` | List all groups |

### Player Management

| Command | Description |
|---|---|
| `/seed player setgroup <player> <group>` | Assign a player to a group |
| `/seed player addperm <player> <permission>` | Grant an extra permission to a player |
| `/seed player removeperm <player> <permission>` | Remove an extra permission from a player |
| `/seed player deny <player> <permission>` | Deny a specific permission for a player (overrides group) |
| `/seed player undeny <player> <permission>` | Remove a denied permission from a player |
| `/seed player setprefix <player> <prefix>` | Set a per-player chat prefix (overrides group prefix) |
| `/seed player clearprefix <player>` | Remove a player's prefix override |
| `/seed player info <player>` | Show a player's group, prefix, extras, denials, and effective permissions |

### Utility

| Command | Description |
|---|---|
| `/seed reload` | Reload configuration from disk |
| `/seed save` | Force save configuration to disk |

## Examples

### Initial setup (from console)

```
/seed group create admin
/seed group addperm admin seed:admin
/seed group addperm admin *
/seed player setgroup Steve admin
```

This creates an `admin` group with full permissions (including the ability to use `/seed` commands), then makes Steve an admin. Steve can now manage permissions in-game.

### Creating a custom group

```
/seed group create vip
/seed group addperm vip minecraft:command.fly
/seed group addperm vip minecraft:command.gamemode

/seed player setgroup Alex vip
/seed player deny Alex minecraft:command.gamemode
/seed player info Alex
```

This creates a `vip` group with fly and gamemode permissions, assigns Alex to it, then denies gamemode specifically for Alex. Alex can fly but cannot change gamemode.

### Setting up chat prefixes

```
/seed group setprefix admin &c[Admin]
/seed group setprefix vip &a[VIP]
/seed player setprefix Steve &6[Owner]
```

Groups with a prefix will have it shown before the player's name in chat (e.g. `[Admin] <Steve> Hello!`). Player-level prefixes override group prefixes. Color codes use `&` notation (converted to `§` at display time). If no prefix is set, Pumpkin's default chat format is used.
