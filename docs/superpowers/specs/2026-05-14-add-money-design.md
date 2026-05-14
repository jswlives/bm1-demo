# AddMoney Command Design

## Overview

Add a `CS_RPC_CMD_ADD_MONEY_REQ / RESP` command pair. Client sends money type + amount, server looks up the player via session, adds money, and returns the updated balance.

## Proto Changes (`message.proto`)

- New enum values: `CS_RPC_CMD_ADD_MONEY_REQ = 5`, `CS_RPC_CMD_ADD_MONEY_RESP = 6`
- New messages:
  - `AddMoneyReq { PlayerBagMoneyType money_type = 1; uint32 amount = 2; }`
  - `AddMoneyResp { uint32 money_count = 1; string error_msg = 2; }`
- New oneof fields: `add_money_req = 16`, `add_money_resp = 17`

## Session & Context Changes

- Add `player_id: u64` field to `Session` (default 0 = not logged in)
- Add `SessionManager::player_id(session_id) -> Option<u64>` method
- Add `SessionManager::set_player_id(session_id, player_id)` method
- Add `player_id: u64` to `Context` so handlers can access it
- Update `LoginHandler` to set `player_id` on the session after successful login
- Update `handle_connection` to populate `ctx.player_id` from session

## Handler: AddMoneyHandler

1. Check `ctx.player_id != 0` → not logged in → error
2. Get player from `PlayerPool`
3. Call `player.add_money(money_type, amount)`
4. Return `AddMoneyResp { money_count, error_msg }`

## Files Changed

| File | Change |
|------|--------|
| `share/proto/protos/message.proto` | Add cmd enum values, request/response messages, oneof fields |
| `bm1-server/src/session.rs` | Add `player_id` field, getter/setter |
| `bm1-server/src/router.rs` | Add `player_id` to `Context` |
| `bm1-server/src/handler/login.rs` | Set session player_id on login |
| `bm1-server/src/handler/add_money.rs` | New handler |
| `bm1-server/src/handler/mod.rs` | Export `AddMoneyHandler` |
| `bm1-server/src/server.rs` | Populate `ctx.player_id`, register handler, pass session manager to login handler |
