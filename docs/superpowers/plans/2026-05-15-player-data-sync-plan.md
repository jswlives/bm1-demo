# PlayerData Delta Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add incremental PlayerData synchronization between server and client using snapshot diff and a unified PlayerDataNotify message.

**Architecture:** Server snapshots PlayerData before each handler, diffs after, sends PlayerDataNotify (delta) before the business response. Client applies deltas to its local cache via a single apply_delta function. Server-push scenarios use the same PlayerDataNotify without a preceding request.

**Tech Stack:** Rust, prost/protobuf, tokio, existing TCP codec

---

### Task 1: Add Delta messages to model.proto

**Files:**
- Modify: `share/proto/protos/model.proto`

- [ ] **Step 1: Add Delta messages to model.proto**

Append the following after the existing `PlayerData` message at the end of the file:

```protobuf
enum DeltaOp {
  DELTA_OP_UNSPECIFIED = 0;
  DELTA_OP_UPSERT = 1;
  DELTA_OP_DELETE = 2;
}

message PlayerBaseDelta {
  optional uint32 player_level = 1;
}

message PlayerBagMoneyDelta {
  PlayerBagMoneyType money_type = 1;
  uint32 money_count = 2;
}

message PlayerBagItemDelta {
  DeltaOp op = 1;
  uint32 item_id = 2;
  uint32 item_count = 3;
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

- [ ] **Step 2: Verify proto file is valid**

Run: `cargo build -p bm1-proto`
Expected: Build succeeds, no errors

- [ ] **Step 3: Commit**

```bash
git add share/proto/protos/model.proto share/protos_build/model.rs
git commit -m "feat(proto): add PlayerDataDelta and sub-delta messages to model.proto"
```

---

### Task 2: Add PlayerDataNotify message to message.proto

**Files:**
- Modify: `share/proto/protos/message.proto`

- [ ] **Step 1: Add PlayerDataNotify message, cmd enum value, and oneof field**

In `message.proto`:

1. Add to `CSRpcCmd` enum:
```protobuf
  CS_RPC_CMD_PLAYER_DATA_NOTIFY = 7;
```

2. Add new message after `AddMoneyResp`:
```protobuf
message PlayerDataNotify {
  model.PlayerDataDelta delta = 1;
  string reason = 2;
}
```

3. Add to `CSRpcMsg` oneof `payload`:
```protobuf
    PlayerDataNotify player_data_notify = 18;
```

The full `message.proto` should become:

```protobuf
syntax = "proto3";
package message;

import "model.proto";

message CSRpcMsg {
  CSRpcCmd cmd = 1;
  uint32 seq = 2;
  uint32 session_id = 3;
  oneof payload {
    LoginReq login_req = 14;
    LoginResp login_resp = 15;
    AddMoneyReq add_money_req = 16;
    AddMoneyResp add_money_resp = 17;
    PlayerDataNotify player_data_notify = 18;
  }
}

enum CSRpcCmd {
  CS_RPC_CMD_UNSPECIFIED = 0;
  CS_RPC_CMD_LOGIN_REQ = 3;
  CS_RPC_CMD_LOGIN_RESP = 4;
  CS_RPC_CMD_ADD_MONEY_REQ = 5;
  CS_RPC_CMD_ADD_MONEY_RESP = 6;
  CS_RPC_CMD_PLAYER_DATA_NOTIFY = 7;
}

message LoginReq {
  uint32 player_id = 1;
}

message LoginResp {
  model.PlayerData player_data = 1;
  string error_msg = 2;
}

message AddMoneyReq {
  model.PlayerBagMoneyType money_type = 1;
  uint32 amount = 2;
}

message AddMoneyResp {
  uint32 money_count = 1;
  string error_msg = 2;
}

message PlayerDataNotify {
  model.PlayerDataDelta delta = 1;
  string reason = 2;
}
```

- [ ] **Step 2: Regenerate proto types**

Run: `cargo build -p bm1-proto`
Expected: Build succeeds. `share/protos_build/message.rs` now contains `PlayerDataNotify`, `PlayerDataNotify` in the Payload oneof, and `CsRpcCmd::PlayerDataNotify`.

- [ ] **Step 3: Verify full workspace builds**

Run: `cargo build`
Expected: Full workspace builds with no errors. Some unused import warnings may appear — that's fine.

- [ ] **Step 4: Commit**

```bash
git add share/proto/protos/message.proto share/protos_build/message.rs share/protos_build/model.rs
git commit -m "feat(proto): add PlayerDataNotify message and CS_RPC_CMD_PLAYER_DATA_NOTIFY"
```

---

### Task 3: Implement diff_player_data with tests

**Files:**
- Create: `bm1-server/src/model/delta.rs`
- Modify: `bm1-server/src/model/mod.rs`

- [ ] **Step 1: Write failing tests for diff_player_data**

Create `bm1-server/src/model/delta.rs`:

```rust
use bm1_proto::model::{
    DeltaOp, PlayerBagDelta, PlayerBagItemDelta, PlayerBagMoneyDelta, PlayerBagMoneyType,
    PlayerBaseDelta, PlayerData, PlayerDataDelta,
};

pub fn diff_player_data(before: &PlayerData, after: &PlayerData) -> Option<PlayerDataDelta> {
    let base = diff_base(before, after);
    let bag = diff_bag(before, after);

    if base.is_none() && bag.is_none() {
        return None;
    }

    Some(PlayerDataDelta { base, bag })
}

fn diff_base(before: &PlayerData, after: &PlayerData) -> Option<PlayerBaseDelta> {
    let before_level = before.player_base.as_ref().map(|b| b.player_level).unwrap_or(0);
    let after_level = after.player_base.as_ref().map(|b| b.player_level).unwrap_or(0);

    if before_level == after_level {
        return None;
    }

    Some(PlayerBaseDelta {
        player_level: Some(after_level),
    })
}

fn diff_bag(before: &PlayerData, after: &PlayerData) -> Option<PlayerBagDelta> {
    let money_changes = diff_money(before, after);
    let item_changes = diff_items(before, after);

    if money_changes.is_empty() && item_changes.is_empty() {
        return None;
    }

    Some(PlayerBagDelta {
        money_changes,
        item_changes,
    })
}

fn diff_money(before: &PlayerData, after: &PlayerData) -> Vec<PlayerBagMoneyDelta> {
    let before_money = before.player_bag.as_ref().map(|b| &b.money).unwrap_or(&EMPTY_MONEY);
    let after_money = after.player_bag.as_ref().map(|b| &b.money).unwrap_or(&EMPTY_MONEY);

    let mut changes = Vec::new();

    for am in after_money {
        let before_count = before_money
            .iter()
            .find(|m| m.money_type == am.money_type)
            .map(|m| m.money_count)
            .unwrap_or(0);

        if am.money_count != before_count {
            changes.push(PlayerBagMoneyDelta {
                money_type: am.money_type,
                money_count: am.money_count,
            });
        }
    }

    // Money types removed entirely (count went to 0 or entry removed)
    for bm in before_money {
        let exists_in_after = after_money
            .iter()
            .any(|m| m.money_type == bm.money_type);
        if !exists_in_after && bm.money_count > 0 {
            changes.push(PlayerBagMoneyDelta {
                money_type: bm.money_type,
                money_count: 0,
            });
        }
    }

    changes
}

fn diff_items(before: &PlayerData, after: &PlayerData) -> Vec<PlayerBagItemDelta> {
    let before_items = before.player_bag.as_ref().map(|b| &b.items).unwrap_or(&EMPTY_ITEMS);
    let after_items = after.player_bag.as_ref().map(|b| &b.items).unwrap_or(&EMPTY_ITEMS);

    let mut changes = Vec::new();

    // Items in after that are new or changed → UPSERT
    for ai in after_items {
        let before_count = before_items
            .iter()
            .find(|i| i.item_id == ai.item_id)
            .map(|i| i.item_count)
            .unwrap_or(0);

        if ai.item_count != before_count {
            changes.push(PlayerBagItemDelta {
                op: DeltaOp::Upsert as i32,
                item_id: ai.item_id,
                item_count: ai.item_count,
            });
        }
    }

    // Items in before but not in after → DELETE
    for bi in before_items {
        let exists_in_after = after_items.iter().any(|i| i.item_id == bi.item_id);
        if !exists_in_after {
            changes.push(PlayerBagItemDelta {
                op: DeltaOp::Delete as i32,
                item_id: bi.item_id,
                item_count: 0,
            });
        }
    }

    changes
}

static EMPTY_MONEY: Vec<bm1_proto::model::PlayerBagMoney> = Vec::new();
static EMPTY_ITEMS: Vec<bm1_proto::model::PlayerBagItem> = Vec::new();

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::model::{PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerBase};

    fn make_player_data(level: u32, gold: u32, diamond: u32, items: Vec<(u32, u32)>) -> PlayerData {
        PlayerData {
            player_base: Some(PlayerBase {
                player_id: 1,
                player_name: "test".into(),
                player_level: level,
            }),
            player_bag: Some(PlayerBag {
                items: items.into_iter().map(|(id, count)| PlayerBagItem { item_id: id, item_count: count }).collect(),
                money: vec![
                    PlayerBagMoney { money_type: PlayerBagMoneyType::Gold as i32, money_count: gold },
                    PlayerBagMoney { money_type: PlayerBagMoneyType::Diamond as i32, money_count: diamond },
                ],
            }),
        }
    }

    #[test]
    fn test_no_change_returns_none() {
        let data = make_player_data(1, 100, 50, vec![]);
        assert!(diff_player_data(&data, &data).is_none());
    }

    #[test]
    fn test_level_change() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_base.as_mut().unwrap().player_level = 3;

        let delta = diff_player_data(&before, &after).unwrap();
        assert_eq!(delta.base.unwrap().player_level, Some(3));
        assert!(delta.bag.is_none());
    }

    #[test]
    fn test_money_change() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().money[0].money_count = 200;

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.money_changes.len(), 1);
        assert_eq!(bag.money_changes[0].money_type, PlayerBagMoneyType::Gold as i32);
        assert_eq!(bag.money_changes[0].money_count, 200);
    }

    #[test]
    fn test_item_added() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().items.push(PlayerBagItem { item_id: 1001, item_count: 5 });

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Upsert as i32);
        assert_eq!(bag.item_changes[0].item_id, 1001);
        assert_eq!(bag.item_changes[0].item_count, 5);
    }

    #[test]
    fn test_item_removed() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let after = make_player_data(1, 100, 50, vec![]);

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Delete as i32);
        assert_eq!(bag.item_changes[0].item_id, 1001);
    }

    #[test]
    fn test_item_count_changed() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().items[0].item_count = 10;

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Upsert as i32);
        assert_eq!(bag.item_changes[0].item_count, 10);
    }

    #[test]
    fn test_multiple_changes() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let mut after = before.clone();
        after.player_base.as_mut().unwrap().player_level = 2;
        after.player_bag.as_mut().unwrap().money[0].money_count = 150;
        after.player_bag.as_mut().unwrap().items[0].item_count = 8;
        after.player_bag.as_mut().unwrap().items.push(PlayerBagItem { item_id: 2001, item_count: 1 });

        let delta = diff_player_data(&before, &after).unwrap();
        assert_eq!(delta.base.unwrap().player_level, Some(2));
        let bag = delta.bag.unwrap();
        assert_eq!(bag.money_changes.len(), 1);
        assert_eq!(bag.item_changes.len(), 2); // update 1001 + add 2001
    }
}
```

- [ ] **Step 2: Export delta module in mod.rs**

In `bm1-server/src/model/mod.rs`, change to:

```rust
pub mod delta;
pub mod player;
pub mod player_pool;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p bm1-server -- delta`
Expected: All 7 delta tests pass

- [ ] **Step 4: Commit**

```bash
git add bm1-server/src/model/delta.rs bm1-server/src/model/mod.rs
git commit -m "feat(server): add diff_player_data with snapshot diff and tests"
```

---

### Task 4: Integrate snapshot diff into server handle_connection

**Files:**
- Modify: `bm1-server/src/server.rs`

- [ ] **Step 1: Modify handle_connection to snapshot-diff and send PlayerDataNotify before Resp**

Replace the dispatch + send block in `handle_connection` (the section after `let ctx = Context { session_id, player_id };`) with:

```rust
                let ctx = Context { session_id, player_id };

                // Snapshot player data before handler (if player is logged in)
                let before = if player_id > 0 {
                    let pool = crate::model::player_pool::PlayerPool::global().read().unwrap();
                    pool.get(player_id).map(|p| p.data().clone())
                } else {
                    None
                };

                if let Some(mut resp) = router.dispatch(&ctx, msg) {
                    // Bind player_id to session after successful login
                    if login_player_id.is_some() && player_id == 0 {
                        if let Some(Payload::LoginResp(ref login_resp)) = resp.payload {
                            if login_resp.error_msg.is_empty() {
                                mgr.lock().await.set_player_id(session_id, login_player_id.unwrap());
                            }
                        }
                    }

                    // Snapshot diff: send PlayerDataNotify BEFORE Resp
                    if let Some(before_data) = before {
                        let after_data = {
                            let pool = crate::model::player_pool::PlayerPool::global().read().unwrap();
                            pool.get(player_id).map(|p| p.data().clone())
                        };
                        if let Some(after_data) = after_data {
                            if let Some(delta) = crate::model::delta::diff_player_data(&before_data, &after_data) {
                                let notify = CsRpcMsg {
                                    cmd: CsRpcCmd::PlayerDataNotify as i32,
                                    seq: 0,
                                    session_id,
                                    payload: Some(Payload::PlayerDataNotify(
                                        bm1_proto::message::PlayerDataNotify {
                                            delta: Some(delta),
                                            reason: String::new(),
                                        },
                                    )),
                                };
                                let _ = write_tx.send(notify).await;
                            }
                        }
                    }

                    resp.session_id = session_id;
                    let _ = write_tx.send(resp).await;
                }
```

- [ ] **Step 2: Update imports at top of server.rs**

Add to the imports:

```rust
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, PlayerDataNotify};
```

(Remove the existing `CsRpcCmd, CsRpcMsg` import line and replace with this one that also includes `PlayerDataNotify`.)

- [ ] **Step 3: Build and run existing tests**

Run: `cargo build && cargo test`
Expected: Build succeeds, all existing tests pass

- [ ] **Step 4: Commit**

```bash
git add bm1-server/src/server.rs
git commit -m "feat(server): integrate snapshot diff, send PlayerDataNotify before Resp"
```

---

### Task 5: Implement apply_delta on client

**Files:**
- Modify: `client/src/main.rs`

- [ ] **Step 1: Add apply_delta function**

Add this function in `client/src/main.rs` before `fn read_line`:

```rust
fn apply_delta(cache: &mut PlayerData, delta: &bm1_proto::model::PlayerDataDelta) {
    if let Some(base_delta) = &delta.base {
        if let Some(level) = base_delta.player_level {
            if let Some(base) = cache.player_base.as_mut() {
                base.player_level = level;
            }
        }
    }

    if let Some(bag_delta) = &delta.bag {
        let bag = cache.player_bag.get_or_insert_with(bm1_proto::model::PlayerBag::default);

        for money_change in &bag_delta.money_changes {
            if let Some(existing) = bag.money.iter_mut().find(|m| m.money_type == money_change.money_type) {
                existing.money_count = money_change.money_count;
            } else {
                bag.money.push(bm1_proto::model::PlayerBagMoney {
                    money_type: money_change.money_type,
                    money_count: money_change.money_count,
                });
            }
        }

        for item_change in &bag_delta.item_changes {
            let op = bm1_proto::model::DeltaOp::try_from(item_change.op).unwrap_or(bm1_proto::model::DeltaOp::Unspecified);
            match op {
                bm1_proto::model::DeltaOp::Upsert => {
                    if let Some(existing) = bag.items.iter_mut().find(|i| i.item_id == item_change.item_id) {
                        existing.item_count = item_change.item_count;
                    } else {
                        bag.items.push(bm1_proto::model::PlayerBagItem {
                            item_id: item_change.item_id,
                            item_count: item_change.item_count,
                        });
                    }
                }
                bm1_proto::model::DeltaOp::Delete => {
                    bag.items.retain(|i| i.item_id != item_change.item_id);
                }
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 2: Add PlayerDataNotify handling in the main loop**

After the existing `PLAYER_DATA.set(RwLock::new(None)).unwrap();` line and before the main loop, the reader_task and Connection already receive all messages. We need to modify the `reader_task` so it also handles PlayerDataNotify.

Actually, the current client design uses `conn.recv()` which reads from a channel fed by `reader_task`. The `reader_task` already forwards all messages. So we just need to handle PlayerDataNotify in the main loop.

However, the current design only calls `conn.recv()` after sending a request. For server-push notifications, we need to handle messages asynchronously. The simplest approach for the CLI client: check for pending notifications before showing the menu.

Add a helper function before `fn read_line`:

```rust
async fn drain_notifications(conn: &mut Connection) {
    while let Ok(msg) = conn.read_rx.try_recv() {
        if let Some(Payload::PlayerDataNotify(notify)) = &msg.payload {
            if let Some(cache) = PLAYER_DATA.get() {
                if let Some(guard) = cache.write().ok().as_mut() {
                    if let Some(data) = guard.as_mut() {
                        if let Some(delta) = &notify.delta {
                            apply_delta(data, delta);
                            println!("<<< [DataSync] reason={}", if notify.reason.is_empty() { "request" } else { &notify.reason });
                        }
                    }
                }
            }
        }
    }
}
```

Wait, `read_rx` is private in `Connection`. We need to either make it accessible or add a `try_recv` method.

Add a `try_recv` method to `Connection`:

```rust
impl Connection {
    async fn send(&self, msg: &CsRpcMsg) -> Result<()> {
        self.write_tx.send(msg.clone()).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CsRpcMsg> {
        let msg = self
            .read_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed"))?;
        Ok(msg)
    }

    fn try_recv(&mut self) -> Option<CsRpcMsg> {
        self.read_rx.try_recv().ok()
    }
}
```

Then add the `drain_notifications` function:

```rust
fn handle_notify(msg: &CsRpcMsg) {
    if let Some(Payload::PlayerDataNotify(notify)) = &msg.payload {
        if let Some(cache) = PLAYER_DATA.get() {
            if let Ok(mut guard) = cache.write() {
                if let Some(data) = guard.as_mut() {
                    if let Some(delta) = &notify.delta {
                        apply_delta(data, delta);
                        println!("<<< [DataSync] reason={}", if notify.reason.is_empty() { "request" } else { &notify.reason });
                    }
                }
            }
        }
    }
}
```

And call it in the main loop at the start of each iteration, before `print_menu`:

```rust
        // Drain pending notifications (e.g., server-push PlayerDataNotify)
        while let Some(msg) = conn.try_recv() {
            handle_notify(&msg);
        }

        print_menu(session_id);
```

Also, after `conn.recv()` in the Login handler, check if the next message is a PlayerDataNotify:

After the line `session_id = resp.session_id;` in the Login branch, add:

```rust
                    // Consume any PlayerDataNotify that follows
                    while let Some(msg) = conn.try_recv() {
                        handle_notify(&msg);
                    }
```

- [ ] **Step 3: Build the client**

Run: `cargo build -p bm1-client`
Expected: Build succeeds

- [ ] **Step 4: Manual integration test**

1. Start server: `cargo run -p bm1-server`
2. Start client: `cargo run -p bm1-client`
3. Login with player_id 1
4. Verify cached PlayerData shows gold=1000, diamond=100
5. Send AddMoneyReq (via adding a new menu option, or temporarily adding test code)

Note: The client currently doesn't have an AddMoney menu option. If needed, add one temporarily for testing, or verify via the existing Login flow that notifications are received correctly. The real test is: after a request that modifies PlayerData, the client's cached data should update via PlayerDataNotify.

For now, verify:
1. Login works (no regression)
2. After login, if any PlayerDataNotify is received, it's handled correctly
3. The print_menu shows updated data after notifications

- [ ] **Step 5: Commit**

```bash
git add client/src/main.rs
git commit -m "feat(client): add apply_delta and PlayerDataNotify handling"
```

---

### Task 6: Add AddMoney menu option to client for end-to-end testing

**Files:**
- Modify: `client/src/main.rs`

- [ ] **Step 1: Add AddMoney option to the client menu**

Update `print_menu` to include:

```rust
fn print_menu(session_id: u32) {
    println!();
    if session_id > 0 {
        println!("=== session_id: {} ===", session_id);
    }
    let cache = PLAYER_DATA.get();
    if let Some(guard) = cache.and_then(|c| c.read().ok()) {
        if let Some(data) = guard.as_ref() {
            let base = data.player_base.as_ref();
            println!("  [cached] id={} name={} level={}",
                base.map(|b| b.player_id).unwrap_or(0),
                base.map(|b| b.player_name.as_str()).unwrap_or("-"),
                base.map(|b| b.player_level).unwrap_or(0),
            );
            if let Some(bag) = data.player_bag.as_ref() {
                for m in &bag.money {
                    let name = match m.money_type {
                        1 => "gold",
                        2 => "diamond",
                        _ => "?",
                    };
                    println!("  [cached] {}={}", name, m.money_count);
                }
            }
        }
    }
    println!("[1] Login      - 登录");
    println!("[2] AddMoney   - 加金币");
    println!("[0] Exit       - 退出");
    println!();
}
```

Add a new match arm in the main loop (after the `"1"` login branch, before `"0"`):

```rust
            "2" => {
                if session_id == 0 {
                    println!("请先登录");
                    continue;
                }
                let input = read_line("输入金额: ");
                let amount: u32 = match input.parse() {
                    Ok(a) => a,
                    Err(_) => {
                        println!("无效金额");
                        continue;
                    }
                };

                seq += 1;
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::AddMoneyReq as i32,
                    seq,
                    session_id,
                    payload: Some(Payload::AddMoneyReq(AddMoneyReq {
                        money_type: 1, // Gold
                        amount,
                    })),
                };
                conn.send(&req).await?;
                println!(">>> 发送 AddMoneyReq: gold +{}", amount);

                // First receive PlayerDataNotify (sent before Resp)
                while let Some(msg) = conn.try_recv() {
                    handle_notify(&msg);
                }

                let resp = conn.recv().await?;
                if let Some(Payload::AddMoneyResp(r)) = &resp.payload {
                    if r.error_msg.is_empty() {
                        println!("<<< 加币成功: gold={}", r.money_count);
                    } else {
                        println!("<<< 加币失败: {}", r.error_msg);
                    }
                }

                // Drain any additional notifications
                while let Some(msg) = conn.try_recv() {
                    handle_notify(&msg);
                }
            }
```

Add the missing import at the top:

```rust
use bm1_proto::message::{AddMoneyReq, CsRpcCmd, CsRpcMsg, LoginReq};
```

(Remove the existing `use bm1_proto::message::{CsRpcCmd, CsRpcMsg, LoginReq};` and replace.)

- [ ] **Step 2: Build client**

Run: `cargo build -p bm1-client`
Expected: Build succeeds

- [ ] **Step 3: End-to-end test**

1. Terminal 1: `cargo run -p bm1-server`
2. Terminal 2: `cargo run -p bm1-client`
3. In client: choose [1], enter player_id `1`
4. Verify login success, cached data shows gold=1000
5. Choose [2], enter amount `100`
6. Verify:
   - `[DataSync] reason=request` notification appears
   - `加币成功: gold=1100` appears
   - Next menu display shows `[cached] gold=1100` (updated via delta)
7. Choose [2] again, enter amount `50`
8. Verify: gold=1150 in cached display

- [ ] **Step 4: Commit**

```bash
git add client/src/main.rs
git commit -m "feat(client): add AddMoney menu option for e2e delta sync testing"
```

---

### Task 7: Update CLAUDE.md with new command and proto field numbers

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the Registered Commands table**

Change the table from:

```markdown
### Registered Commands

| CSRpcCmd | i32 | Handler |
|---|---|---|
| LoginReq | 3 | LoginHandler |
| LoginResp | 4 | — |
| AddMoneyReq | 5 | AddMoneyHandler |
| AddMoneyResp | 6 | — |

Next available oneof field number: **18**.
```

To:

```markdown
### Registered Commands

| CSRpcCmd | i32 | Handler |
|---|---|---|
| LoginReq | 3 | LoginHandler |
| LoginResp | 4 | — |
| AddMoneyReq | 5 | AddMoneyHandler |
| AddMoneyResp | 6 | — |
| PlayerDataNotify | 7 | (server-push, no handler) |

Next available cmd: **8**. Next available oneof field number: **19**.
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with PlayerDataNotify command"
```
