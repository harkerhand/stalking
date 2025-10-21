# Stalking 项目架构重设计（中文）

本文档描述了对当前 `stalking` 项目的架构重设计，目标是：

- 从配置文件加载一组或多组服务器的配置。
- 在终端动态显示每台服务器的监控信息（支持简单刷新模式与可选 TUI 模式）。
- 保留并复用现有的解析逻辑（`mem`, `cpu`, `disk`, `net`），并做少量接口适配。
- 提供并发、错误处理、安全（密钥优先）和可扩展的模块划分。

---

快速清单（实现交付）

- [x] 配置格式及示例（TOML）。
- [x] 新增模块与职责划分（`config`, `agent`, `ui`, `model` 等）。
- [x] 运行时流程（主进程 -> agent -> UI）。
- [x] Monitorable trait 建议改动以支持可配置命令。
- [x] SSH 客户端增强（密钥/端口/超时）。
- [x] 迁移与实施步骤（哪些文件创建/修改）。

---

1. 设计目标

- 将监控目标与监控策略从代码中分离，改为由配置文件驱动。
- 支持多台服务器并发采样，采样结果通过通道传到 UI 进行动态显示。
- 保持现有解析函数的行为（尽量不改动解析实现），只在调用层面适配。

2. 高层架构

- `main`：启动逻辑（读取配置、初始化日志、启动 UI、启动 agent）。
- `config`：负责解析 TOML 配置并校验（`Config`, `ServerConfig`, `GlobalConfig`）。
- `ssh`：现有 `SSHClient` 扩展，支持密钥、端口、重连与超时。
- `agent`：每台服务器的工作线程/任务，定期执行配置中列出的监控项并发送事件。
- `monitor/*`：保留 `mem.rs`, `cpu.rs`, `disk.rs`, `net.rs`，对 `Monitorable` trait 做轻微适配。
- `ui`：接收监控事件并在终端绘制界面（支持 plain 与 tui 两种模式）。
- `model` 或 `types`：公共类型（`MonitorEvent`, `MonitorKind`, `MonitorPayload` 等）。

3. 配置格式（推荐 TOML）

示例 `config.toml`：

```toml
[global]
refresh = 1000          # UI 刷新间隔 ms
display = "tui"       # "plain" 或 "tui"
concurrency = 8        # 最大并发 agent 数

[monitors]
default_interval_ms = 2000

[[servers]]
name = "web-01"
host = "10.210.126.58"
port = 22
user = "harkerhand"
password = ""            # 可选，尽量用 key
key_path = "C:\\Users\\me\\.ssh\\id_rsa"
monitors = ["mem", "disk", "cpu", "net"]
poll_interval_ms = 2000

[[servers]]
name = "db-01"
host = "10.210.120.11"
user = "dbadmin"
key_path = "/home/me/.ssh/id_rsa"
monitors = ["mem", "disk"]
poll_interval_ms = 5000
```

4. Rust 数据模型建议

使用 `serde` 从 TOML 反序列化：

```rust
#[derive(Debug, Deserialize)]
struct Config { global: GlobalConfig, monitors: Option<MonitorsConfig>, servers: Vec<ServerConfig> }

#[derive(Debug, Deserialize)]
struct ServerConfig { name: String, host: String, port: Option<u16>, user: String, password: Option<String>, key_path: Option<String>, monitors: Vec<String>, poll_interval_ms: Option<u64> }
```

5. Monitorable trait 建议（兼容现有实现）

当前 trait 在 `main.rs`：

```rust
pub trait Monitorable {
    fn exec_cmd() -> &'static str;
    fn parse_from_str(s: &str) -> Result<Self> where Self: Sized;
    fn common_display(&self) -> String;
}
```

建议改为：

```rust
pub trait Monitorable: Sized {
    // 根据运行时配置（例如采样间隔、服务器信息）生成要执行的命令
    fn exec_cmd(config: &MonitorExecConfig) -> String;
    fn parse_from_str(s: &str) -> anyhow::Result<Self>;
    fn common_display(&self) -> String;
}

pub struct MonitorExecConfig { pub interval_ms: u64, pub server: ServerConfig }
```

实现兼容策略：在短期内，让 `exec_cmd(config)` 若未使用 config 时返回原先的静态命令（便于逐步迁移各个监控模块）。

6. 运行时流程（概要）

- 启动时，`main` 解析配置，建立一个事件通道：`agent -> ui`。
- 根据 `concurrency` 限制，为每台服务器启动一个 `agent`（或使用线程池）。
- `agent`：
    - 用 `SSHClient::connect_from_config(&ServerConfig)` 建立 SSH 连接（支持密钥/密码/端口）。
    - 对配置里的每个监控项，构造执行命令并通过 SSH 执行，解析输出，发送 `MonitorEvent::Sample`。
    - 发生错误时发送 `MonitorEvent::Error` 并按重试策略重连。
- `ui` 监听这些事件并维护内存中最新状态（以及可选的历史样本），定时重绘。

7. 事件与消息格式

推荐定义如下事件：

```rust
enum MonitorEvent {
    Sample { server: String, kind: MonitorKind, payload: MonitorPayload, timestamp: DateTime },
    Error { server: String, kind: Option<MonitorKind>, error: String, timestamp: DateTime }
}
```

`MonitorPayload` 为对不同监控类型（`MemInfo`, `CpuInfo`, `DiskInfo`, `NetInfo`）的枚举包装。

8. UI 选项

- Plain 模式：跨平台实现最简单，采用 ANSI 清屏然后按服务器块输出（兼容 Windows 10+）。
- TUI 模式（推荐）：使用 `tui` + `crossterm`，提供服务器列表、详情面板、历史折线图/火花线（需要内存保存历史样本）。

实现策略：先实现 Plain 模式保证端到端通路，然后基于该通路增加 TUI。

9. 并发模型

- 采用线程（std::thread）+ 通道（crossbeam-channel 或 std::sync::mpsc）。
- 每台服务器一个长期线程（对阻塞型 `ssh2` 最简单可靠），内部顺序执行该服务器所有监控项。
- 若需要更细粒度控制，可在 agent 内为每个监控项单独调度任务。

10. SSH 改进建议

- 新增 `connect_from_config(cfg: &ServerConfig)`，支持：
    - 密钥认证（userauth_pubkey_file）
    - 自定义端口
    - socket/read/write 超时
    - 连接重试与指数退避
- 将 `exec::<T>()` 改为通用 `exec_command(cmd: &str) -> Result<String>`，并让 agent 根据 monitor 的 `exec_cmd`
  来调用并交给对应的解析函数。

11. 错误处理与鲁棒性

- 解析错误：记录并发送 Error 事件，不中断其它监控项。
- SSH 连接错误：短时重试若干次，失败后记录为不可用并延迟更长时间尝试重连。
- 配置校验在启动时进行（重复 server 名称、未知监控名、缺失认证信息等）。

12. 安全注意

- 建议不要把明文密码放到版本库中。支持使用环境变量或 `env://VAR_NAME` 协议在配置文件中引用 secrets。
- 优先使用密钥对认证并建议对私钥文件应用合适的文件权限。

13. 迁移与实现步骤（具体文件与改动）

推荐分步迁移：先实现最小可行产品（Plain UI + 多服务器配置 + SSH 密钥支持）

- 新建： `src/config.rs` （解析 TOML，导出 Config、ServerConfig、MonitorKind）。
- 新建： `src/agent.rs` （agent 工作循环，采样并发送事件）。
- 新建： `src/ui.rs` （先实现 Plain UI）。
- 新建： `src/model.rs` 或 `src/types.rs`（事件类型、MonitorPayload 枚举）。
- 修改： `src/ssh.rs`（增加 `connect_from_config`、`exec_command`、密钥/端口/超时支持）。
- 修改： `src/main.rs`（改为读取配置并按配置启动 agent 与 UI）。
- 逐步修改： `src/{mem,cpu,disk,net}.rs` 的 `Monitorable` 实现以兼容 `exec_cmd(config)`。

14. 测试建议

- 单元测试：对 `config::load` 做正常/异常配置测试；对现有解析函数（`parse_from_str`）添加更多边界用例。
- 集成测试：提供 `--once` 模式让 agent 运行一次并退出（便于 CI）。
- 模拟测试：抽象 SSH 后端为 trait，编写 MockBackend 返回固定输出以测试 agent 与 ui。

15. 依赖建议（Cargo.toml）

- serde = { version = "1.0", features = ["derive"] }
- toml = "0.5"
- anyhow (已有)
- ssh2 (已有)
- crossbeam-channel = "0.5" 或使用 std 的 channel
- log + env_logger
- 可选： tui + crossterm（用于 TUI）
- 可选： chrono（时间戳）

16. CLI 与运行示例

- 运行带 TUI：

```bash
# Windows cmd 示例
stalking.exe --config config.toml --display tui
```

- 仅运行一次（用于调试/CI）：

```bash
stalking.exe --config config.toml --once
```

17. 质量检查（质量门）

- 构建： `cargo build` 通过
- 类型/静态检查： `cargo check` / `cargo clippy`
- 单元测试： `cargo test`（新增测试覆盖解析与配置）
- 端到端 smoke 测试： 使用 `--once` 对 localhost 或 mock backend 执行一个周期

18. 需求覆盖映射

- 从配置文件加载服务器：已在配置设计中覆盖（TOML + `config.rs`） — Done
- 终端动态显示：满足（Plain + TUI 方案） — Done
- 复用现有解析逻辑：保留并建议小改动以兼容新 trait — Done
- 并发与稳定性：线程 + 通道 + 重连策略 — Done

19. 后续建议与可选增强

- 将监控数据导出为时间序列（Prometheus/Influx）以便持久化与告警。
- 支持基于标签/组的视图和过滤。
- 支持按需对单台服务器执行即时命令（remote shell）。
