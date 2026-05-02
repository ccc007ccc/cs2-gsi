# cs2-gsi

[![crates.io](https://img.shields.io/crates/v/cs2-gsi.svg)](https://crates.io/crates/cs2-gsi)
[![docs.rs](https://docs.rs/cs2-gsi/badge.svg)](https://docs.rs/cs2-gsi)
[![CI](https://github.com/ccc007ccc/cs2-gsi/actions/workflows/ci.yml/badge.svg)](https://github.com/ccc007ccc/cs2-gsi/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

[English](README.md) · **简体中文**

Rust 写的异步 **Counter-Strike 2 Game State Integration** 监听器。

> [`antonpup/CounterStrike2GSI`][upstream] 的 Rust 移植与重新设计。

```rust
use cs2_gsi::{cfg::GsiCfg, events::PlayerDied, GameStateListener};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    GsiCfg::for_localhost("MyApp", 4000).write_to_cs2()?;

    let gsl = GameStateListener::new(4000);
    gsl.on(|e: &PlayerDied| println!("☠ {} 死了", e.player.name));
    gsl.start().await?;

    tokio::signal::ctrl_c().await?;
    gsl.stop().await?;
    Ok(())
}
```

---

## 这个 crate 解决什么问题？

CS2 自带 [Game State Integration][gsi-docs] 功能 —— 它会把比赛实时状态以 JSON 形式 POST 到你配置的 HTTP 端点。要让这堆 JSON 真正可用，你需要四样东西：

1. 一个 HTTP 监听器。
2. 一个能精准映射 CS2 实际下发 JSON 的数据模型（带它所有怪癖：数字字段以字符串编码、字段命名不一致、子对象可能缺失）。
3. 一个 diff 引擎，把 _快照_ 转换成可消费的事件（`PlayerDied`、`BombPlanted`、`RoundConcluded`、`KillFeed`、…）。
4. 一种把 `gamestate_integration_*.cfg` 放到正确位置的方式，否则 CS2 不会主动推送。

`cs2-gsi` 一次解决全部四件事 —— 强类型、异步、API 表面干净。

| 能力 | 状态 |
|---|:-:|
| HTTP 监听器（hyper 1.x + tokio）| ✅ |
| 强类型 `GameState` 模型 | ✅ |
| 40+ 推导事件，带 `previous` / `new` | ✅ |
| 合成的 `KillFeed` 事件 | ✅ |
| 自动写入 `gamestate_integration_*.cfg` | ✅ |
| Steam 库 + `appmanifest_730.acf` 发现（Win/Linux/macOS）| ✅ |
| `auth { token "..." }` 块支持 | ✅ |
| 优雅启停、热注册 handler | ✅ |

---

## 安装

```toml
[dependencies]
cs2-gsi = "0.1"
tokio   = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
```

Cargo features（默认全开）：

| Feature | 用途 |
|---|---|
| `cfg-writer` | `GsiCfg` 构建器 + `gamestate_integration_*.cfg` 写入 |
| `steam-discover` | 通过 Steam 的 `libraryfolders.vdf` 定位 CS2 安装目录 |

如果只需要 HTTP 监听器和数据模型（最小依赖），关掉两个 feature：

```toml
cs2-gsi = { version = "0.1", default-features = false }
```

---

## 架构

```text
┌────────────┐   POST JSON    ┌──────────────────┐
│ CS2 客户端 │───────────────▶│ GameStateListener│
└────────────┘                │   (this crate)   │
                              │   • hyper server │
                              │   • parse → State│
                              │   • diff w/ prev │
                              └────────┬─────────┘
                                       │ 强类型事件
              ┌────────────────────────┼─────────────────────────┐
              ▼                        ▼                         ▼
       PlayerDied{..}          RoundPhaseUpdated{..}      KillFeed{killer,victim,..}
       PlayerGotKill{..}       BombPlanted{..}            ...
```

每个 payload 都会被解析成 [`GameState`]、与上一个 state 做 diff、生成强类型事件流。Handler **同步** 在监听器的 tokio task 上执行（沿用上游 C# 库的行为）—— 如果 handler 要做重活，请在 handler 内 `tokio::spawn`。

---

## 订阅事件

```rust
use cs2_gsi::{events::*, GameEvent, GameStateListener};

let gsl = GameStateListener::new(4000);

// 强类型回调。
gsl.on(|e: &PlayerDied| { /* ... */ });
gsl.on(|e: &BombPlanted| { /* ... */ });
gsl.on(|e: &KillFeed| {
    println!(
        "{} ({}{}) 击杀了 {}",
        e.killer.name,
        e.weapon.as_deref().unwrap_or("unknown"),
        if e.is_headshot { ", HS" } else { "" },
        e.victim.name,
    );
});

// 或者一个统一处理所有事件的 handler：
gsl.on_any(|evt: &GameEvent| match evt {
    GameEvent::RoundPhaseUpdated(p) => println!("phase {:?} → {:?}", p.previous, p.new),
    _ => {}
});
```

diff 引擎可触发的事件（非穷举）：

- **玩家**：`PlayerUpdated`、`PlayerDied`、`PlayerRespawned`、`PlayerTookDamage`、`PlayerGotKill`、`PlayerHealthChanged`、`PlayerArmorChanged`、`PlayerHelmetChanged`、`PlayerFlashAmountChanged`、`PlayerSmokedAmountChanged`、`PlayerBurningAmountChanged`、`PlayerMoneyAmountChanged`、`PlayerEquipmentValueChanged`、`PlayerRoundKillsChanged`、`PlayerRoundHeadshotKillsChanged`、`PlayerRoundTotalDamageChanged`、`PlayerActivityChanged`、`PlayerTeamChanged`、`PlayerActiveWeaponChanged`、`PlayerStatsChanged`
- **回合 / 比赛**：`RoundUpdated`、`RoundPhaseUpdated`、`RoundStarted`、`RoundConcluded`、`FreezetimeStarted`、`FreezetimeOver`、`MatchStarted`、`GameOver`、`BombStateUpdated`
- **地图**：`MapUpdated`、`GamemodeChanged`、`LevelChanged`、`MapPhaseChanged`、`TeamScoreChanged`
- **炸弹（顶层）**：`BombUpdated`、`BombPlanting`、`BombPlanted`、`BombDefusing`、`BombDefused`、`BombExploded`、`BombDropped`、`BombPickedUp`
- **合成事件**：`KillFeed`（一次 diff 窗口内匹配 kill ↔ death）
- **元事件**：`NewGameState`、`AuthUpdated`、`ProviderUpdated`

---

## 生成 cfg 文件

CS2 必须知道往哪推 payload。`cs2-gsi` 生成的 cfg 与上游 CounterStrike2GSI 库形态一致：

```rust
use cs2_gsi::cfg::GsiCfg;

// 1. 自动发现并写入 ...\Counter-Strike Global Offensive\game\csgo\cfg\
GsiCfg::for_localhost("MyApp", 4000)
    .with_auth("token", "secret-shared-with-myself")
    .write_to_cs2()?;

// 2. 或者只渲染成字符串，自己决定放哪。
let kv = GsiCfg::for_localhost("MyApp", 4000).render();
println!("{kv}");
```

输出（节选）：

```text
"MyApp Integration Configuration"
{
    "uri"          "http://localhost:4000/"
    "timeout"      "5.0"
    "buffer"       "0.1"
    "throttle"     "0.1"
    "heartbeat"    "10.0"
    "data"
    {
        "allgrenades              "        "1"
        "allplayers_id            "        "1"
        "allplayers_match_stats   "        "1"
        ...
    }
}
```

---

## 示例

```bash
cargo run --example quickstart   # PlayerDied + 回合阶段
cargo run --example full_dump    # 把每一个事件 debug-print 出来
```

---

## MSRV 与平台

- **Rust 1.86** 或以上。dev-dependency `reqwest 0.12` 间接依赖 `idna_adapter`
  与 `icu_*` 系列，这些库的最新版要求 Rust 1.86。库本身只用基础 2021-edition
  特性，老工具链下 `cargo build`（不带 dev-deps）仍可编译。
- 主要在 Windows 10/11 上测试（CS2 的主要运行平台）。Linux 与 macOS 在监听器 / 解析器 / Steam 发现层面都支持，分别使用 `~/.steam/steam` 和 `~/Library/Application Support/Steam`。

---

## 与上游 (C#) 的差异

`cs2-gsi` **不是** 1:1 绑定 —— 而是面向 idiomatic Rust 的重新实现：

| 上游 (C#) | 本 crate |
|---|---|
| `event` / `+= handler` | `gsl.on(|e: &PlayerDied| ...)` |
| `PascalCase` 事件类型 | `PascalCase` 类型保持一致 |
| 70+ 事件 | ~45 个事件，覆盖所有常用场景 |
| `EventDispatcher` 同步调用 handler | 同上：handler 同步在 listener task 上执行 |
| `GenerateGSIConfigFile(name)` | `GsiCfg::for_localhost(name, port).write_to_cs2()` |
| `.NET HttpListener`（单线程） | `hyper` 1.x + `tokio`（多线程 accept） |
| 数字字段消费麻烦 | 自动从 JSON 字符串强制转换 |

如果你需要的上游事件还没暴露，提个 issue —— 加新 variant 是机械工作（一个 diff 分支、一个事件结构体）。

---

## 许可

可任选：

- **Apache License, Version 2.0**（[LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0）
- **MIT license**（[LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT）

### 贡献

除非你显式声明，任何你有意提交以纳入本项目的贡献，将按 Apache-2.0 协议中的定义双授权（MIT 与 Apache-2.0），不附加任何额外条款。

## 致谢

灵感与行为兼容来自 [`antonpup/CounterStrike2GSI`][upstream]。

[upstream]: https://github.com/antonpup/CounterStrike2GSI
[gsi-docs]: https://developer.valvesoftware.com/wiki/Counter-Strike_Global_Offensive_Game_State_Integration
