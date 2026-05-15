# PlayerData Delta Sync Design

## Problem

After the client modifies server-side PlayerData (add money, add items, etc.), the client's local cached PlayerData becomes stale. Need a mechanism to keep client-side PlayerData in sync with the server.

Requirements:
- Incremental (delta) sync, not full PlayerData replacement
- Support both request-response sync and server-push notifications (e.g., daily rewards)
- Universal protocol layer, not tied to a specific client type
- Reuse existing TCP protocol pipe for push notifications

## Design Decisions

| Decision | Choice | Reason |
|---|---|---|
| Sync granularity | Delta (incremental) | Bandwidth efficient, scalable |
| Delta format | Structured PlayerDataDelta (Approach B) | Type-safe, key-based array ops, extensible |
| Delta generation | Snapshot diff (Approach 1) | Zero-invasion to Player API, PlayerData is small |
| Push mechanism | Reuse existing CSRpcMsg pipe | No extra connection, reader_task already handles it |
| Delta delivery | Unified PlayerDataNotify, not per-Resp | One apply-delta logic on client, new commands auto-sync |
| Send order | Delta first, then Resp | Client cache is up-to-date when Resp arrives |

## Proto Changes

### model.proto — New Delta messages

```protobuf
enum DeltaOp {
  DELTA_OP_UNSPECIFIED = 0;
  DELTA_OP_UPSERT = 1;   // Update if exists, insert if not
  DELTA_OP_DELETE = 2;   // Remove
}

message PlayerBaseDelta {
  optional uint32 player_level = 1;  // Present = changed, absent = unchanged
}

message PlayerBagMoneyDelta {
  PlayerBagMoneyType money_type = 1;  // Key
  uint32 money_count = 2;             // New absolute value
}

message PlayerBagItemDelta {
  DeltaOp op = 1;          // UPSERT or DELETE
  uint32 item_id = 2;      // Key
  uint32 item_count = 3;   // New value (for UPSERT)
}

message PlayerBagDelta {
  repeated PlayerBagMoneyDelta money_changes = 1;
  repeated PlayerBagItemDelta item_changes = 2;
}

message PlayerDataDelta {
  optional PlayerBaseDelta base = 1;
  optional PlayerBagDelta bag = 2;
}
```

### message.proto — New notify message and cmd

```protobuf
message PlayerDataNotify {
  model.PlayerDataDelta delta = 1;
  string reason = 2;  // Push reason (e.g., "daily_reward")
}

// Add to CSRpcCmd enum:
//   CS_RPC_CMD_PLAYER_DATA_NOTIFY = 7;

// Add to CSRpcMsg oneof payload:
//   PlayerDataNotify player_data_notify = 18;
```

Existing Resp messages (AddMoneyResp, LoginResp, etc.) remain unchanged.

### Key conventions

- **Scalar fields** (player_level etc.): `optional` present = changed, absent = unchanged
- **Struct arrays** (items): Keyed by business key (`item_id`) + `DeltaOp` (UPSERT/DELETE)
- **Money arrays**: Keyed by `money_type`, new absolute value (no DeltaOp needed; money is never "deleted")
- **Value-type arrays** (`repeated int32` etc.): Full replacement when changed

## Server-Side: Snapshot Diff in handle_connection

The Handler trait stays unchanged. The diff logic lives in the server's connection handler.

```rust
// Pseudocode in handle_connection / dispatch flow
let before = player.data().clone();

let resp = router.dispatch(&ctx, msg);  // Existing logic unchanged

let delta = diff_player_data(&before, player.data());
if let Some(delta) = delta {
    // Send Delta FIRST
    send_msg(PlayerDataNotify { delta, reason: "" });
}
// Then send business response
if let Some(resp) = resp {
    send_msg(resp);
}
```

The `diff_player_data` function compares before/after PlayerData and produces `Option<PlayerDataDelta>`. Returns `None` if nothing changed (e.g., error responses that didn't modify data).

### diff_player_data logic

1. Compare `player_base.player_level` → if different, set `PlayerBaseDelta::player_level`
2. Compare `player_bag.money` by `money_type` → collect changed entries into `money_changes`
3. Compare `player_bag.items` by `item_id`:
   - Present in after but not in before → UPSERT (new item)
   - Present in both but count differs → UPSERT (updated)
   - Present in before but not in after → DELETE (removed item)
4. If all sub-deltas are empty, return `None`

## Client-Side: Unified Delta Apply

The existing `reader_task` already receives all messages. Add handling for `PlayerDataNotify`:

```rust
// In the message receive loop
match &msg.payload {
    Some(Payload::PlayerDataNotify(notify)) => {
        apply_delta(&mut player_data_cache, &notify.delta);
    }
    // Existing handlers unchanged — just business logic, no data sync
    Some(Payload::AddMoneyResp(r)) => { /* display result, play effect, etc. */ }
    ...
}
```

### apply_delta logic

1. If `delta.base` present:
   - If `player_level` present → update cache.player_base.player_level
2. If `delta.bag` present:
   - For each `money_change` → find entry by `money_type` in cache, update `money_count`
   - For each `item_change`:
     - UPSERT: find by `item_id`, update `item_count` if found, push new if not
     - DELETE: find by `item_id`, remove from list

## Send Order Guarantee

TCP preserves message order. Server sends PlayerDataNotify before Resp on the same connection, so client always receives Delta before Resp. This means:

- When client handles Resp, local cache is already up-to-date
- UI/业务 code in Resp handler can safely read from local cache

## Future: Server-Push Scenarios

For server-initiated changes (daily rewards, system events, etc.), the server just sends a `PlayerDataNotify` without a preceding Resp. The same `apply_delta` logic handles it.

```rust
// Server-push example: daily reward
let before = player.data().clone();
player.add_money(Gold, 100);
let delta = diff_player_data(&before, player.data()).unwrap();
send_msg(PlayerDataNotify { delta, reason: "daily_reward".into() });
// No Resp — this is a push, not a response
```

## Next Available Proto Field Numbers

- CSRpcCmd: next is **8**
- CSRpcMsg oneof: next is **19** (after player_data_notify = 18)
