# Equipment: Buy and Upgrade

**Date**: 2026-05-15
**Iteration**: 001
**Commit range**: (current working tree)

## Requirements

- Add `PlayerEquip` model placed inside `PlayerEquipData` in `PlayerData` (not PlayerBag)
- Users can own multiple equipment pieces, each with `equip_id` and `equip_level` (initial level 1)
- Equipment can be upgraded (costs 100 gold per level)
- Equipment can be purchased (costs 500 gold)
- Two new commands: BuyEquip and UpgradeEquip

## Design Decisions

- **PlayerEquipData wraps repeated PlayerEquip**, placed on PlayerData as field 4 — follows the same pattern as PlayerSkillData/PlayerSkill, keeps PlayerBag for items/money only
- **Buy and Upgrade are separate commands** (not a single "operate" command) — follows existing SkillUnlock/SkillUpgrade pattern, keeps handlers focused
- **validate-then-mutate pattern in upgrade_equip**: checks ownership with immutable find_equip before mutable sub_gold, avoiding borrow conflict with ensure_equip().equips.iter_mut()

## Proto Changes

### message.proto

| Cmd | Enum | i32 |
|-----|------|-----|
| 12 | CS_RPC_CMD_BUY_EQUIP_REQ | 12 |
| 13 | CS_RPC_CMD_BUY_EQUIP_RESP | 13 |
| 14 | CS_RPC_CMD_UPGRADE_EQUIP_REQ | 14 |
| 15 | CS_RPC_CMD_UPGRADE_EQUIP_RESP | 15 |

Oneof field numbers: 23, 24, 25, 26

### model.proto

New messages: `PlayerEquip`, `PlayerEquipData`, `PlayerEquipDelta`, `PlayerEquipDataDelta`
PlayerData new field: `player_equip = 4` (PlayerEquipData)
PlayerDataDelta new field: `equip = 4` (optional PlayerEquipDataDelta)

## Model Changes

| Action | File | Notes |
|--------|------|-------|
| Modify | `bm1-server/src/model/player.rs` | Added ensure_equip, find_equip, equip_level, buy_equip, upgrade_equip methods |
| Modify | `bm1-server/src/model/delta.rs` | Added diff_equip function, extended diff_player_data with equip field |
| Modify | `bm1-server/src/model/player_pool.rs` | Updated all PlayerData literals with player_equip field |

## Handler Changes

| Action | File | Notes |
|--------|------|-------|
| New | `bm1-server/src/handler/buy_equip.rs` | BuyEquipHandler — validates gold, checks not owned, adds equip |
| New | `bm1-server/src/handler/upgrade_equip.rs` | UpgradeEquipHandler — validates ownership, deducts gold, levels up |

## Commands Registered

| Cmd i32 | Request | Response | Handler |
|---------|---------|----------|---------|
| 12 | BuyEquipReq | BuyEquipResp | BuyEquipHandler |
| 14 | UpgradeEquipReq | UpgradeEquipResp | UpgradeEquipHandler |

## Test Coverage

- BuyEquipHandler: not logged in, success, already owned, insufficient gold
- UpgradeEquipHandler: not logged in, success, not owned, insufficient gold
- Player model: buy success, buy already owned, buy insufficient gold, upgrade success, upgrade not owned, upgrade insufficient gold
- Delta: equip-level detection included in existing multi-change tests

## Related Features

- [[../player-economy/_overview]] — both buy and upgrade deduct gold via Player::sub_gold
- [[../skills/_overview]] — same gold-cost pattern as skill operations
