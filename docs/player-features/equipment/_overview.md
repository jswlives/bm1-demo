# Equipment — Overview

> Context-recovery entry. Read this before touching any code for this feature.

**Status**: done
**Slug**: equipment

## Commands

| Cmd | Name | i32 | Handler |
|-----|------|-----|---------|
| 12 | BuyEquipReq | 12 | BuyEquipHandler |
| 13 | BuyEquipResp | 13 | — |
| 14 | UpgradeEquipReq | 14 | UpgradeEquipHandler |
| 15 | UpgradeEquipResp | 15 | — |

## Model Files

| File | Purpose |
|------|---------|
| `bm1-server/src/model/player.rs` | Player equip methods (buy_equip, upgrade_equip) |

## Handler Files

| File | Purpose |
|------|---------|
| `bm1-server/src/handler/buy_equip.rs` | BuyEquipHandler |
| `bm1-server/src/handler/upgrade_equip.rs` | UpgradeEquipHandler |

## Proto Fields Used

- `message.proto`: oneof field numbers 23-26
- `model.proto`: PlayerData field 4 (player_equip), new messages: PlayerEquip, PlayerEquipData, PlayerEquipDelta, PlayerEquipDataDelta

## Iterations

| # | Date | File | Summary |
|---|------|------|---------|
| 1 | 2026-05-15 | [001-buy-and-upgrade.md](001-buy-and-upgrade.md) | Buy and upgrade equipment with gold |

## Related Features

- [[../player-economy/_overview]] — equipment operations cost gold
