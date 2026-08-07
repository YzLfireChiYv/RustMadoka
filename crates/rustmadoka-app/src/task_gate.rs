//! 进程内游戏账号任务门闩 — channel + 引继码
//!
//! # 职责
//! - 同一游戏身份同时只能有一个打服任务
//! - **组队 Raid**：一次占用多把锁；登记 **owner_group**（仅发起用户组可停止）
//!
//! 文档:
//! - `docs/tech/INSTANCE_AND_CLI.md`
//! - `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §1.2
//! - `docs/PLAN_INSTANCE_CLI_PORT.md`
//!
//! 仅存在于 Owner 内存；不落盘（避免延迟与脏锁）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 进行中的打服任务登记
#[derive(Debug, Clone)]
struct Running {
    task: String,
    /// 发起任务的用户组；组队/跨组停止权用。单号任务可填当前组。
    owner_group: String,
}

#[derive(Debug, Default)]
struct Inner {
    map: HashMap<String, Running>,
}

/// 共享任务门闩
#[derive(Clone, Default)]
pub struct TaskGate {
    inner: Arc<Mutex<Inner>>,
}

/// RAII：Drop 时释放该游戏账号
pub struct TaskGuard {
    gate: TaskGate,
    key: String,
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Ok(mut g) = self.gate.inner.lock() {
            g.map.remove(&self.key);
        }
    }
}

/// 多账号占用（组队 Raid）；Drop 时全部释放
pub struct MultiTaskGuard {
    gate: TaskGate,
    keys: Vec<String>,
    pub owner_group: String,
}

impl Drop for MultiTaskGuard {
    fn drop(&mut self) {
        if let Ok(mut g) = self.gate.inner.lock() {
            for k in &self.keys {
                g.map.remove(k);
            }
        }
    }
}

impl TaskGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// 键：channel(小写) + 引继码

    pub fn account_key(channel: &str, migration_code: &str) -> String {
        format!(
            "{}:{}",
            channel.trim().to_lowercase(),
            migration_code.trim()
        )
    }

    /// 前端可见的游戏身份短指纹（不含明文引继；跨用户组同卡进度匹配用）

    pub fn game_id_hash(channel: &str, migration_code: &str) -> String {
        use sha2::{Digest, Sha256};
        let raw = Self::account_key(channel, migration_code);
        let mut h = Sha256::new();
        h.update(raw.as_bytes());
        hex::encode(h.finalize())[..16].to_string()
    }

    /// 尝试开始任务；若该引继码已有打服任务则 Err

    pub fn try_begin(
        &self,
        channel: &str,
        migration_code: &str,
        task: impl Into<String>,
    ) -> Result<TaskGuard, String> {
        self.try_begin_owned(channel, migration_code, task, "")
    }

    /// 带发起用户组的占用（停止权归属）

    pub fn try_begin_owned(
        &self,
        channel: &str,
        migration_code: &str,
        task: impl Into<String>,
        owner_group: impl Into<String>,
    ) -> Result<TaskGuard, String> {
        let key = Self::account_key(channel, migration_code);
        let task = task.into();
        let owner_group = owner_group.into();
        let mut g = self
            .inner
            .lock()
            .map_err(|_| "任务门闩内部锁异常".to_string())?;
        if let Some(r) = g.map.get(&key) {
            return Err(format!(
                "该游戏账号（服={channel} 引继码）正在执行「{}」（发起用户组={}），不能同时再执行「{task}」。",
                r.task,
                if r.owner_group.is_empty() {
                    "（未记录）"
                } else {
                    r.owner_group.as_str()
                }
            ));
        }
        g.map.insert(
            key.clone(),
            Running {
                task,
                owner_group,
            },
        );
        Ok(TaskGuard {
            gate: self.clone(),
            key,
        })
    }

    /// 组队：一次锁定多个游戏身份；任一把锁失败则全部不占用

    pub fn try_begin_many(
        &self,
        accounts: &[(String, String)],
        task: impl Into<String>,
        owner_group: impl Into<String>,
    ) -> Result<MultiTaskGuard, String> {
        let task = task.into();
        let owner_group = owner_group.into();
        let keys: Vec<String> = accounts
            .iter()
            .map(|(ch, mig)| Self::account_key(ch, mig))
            .collect();
        // 去重
        let mut uniq = keys.clone();
        uniq.sort();
        uniq.dedup();
        if uniq.len() != keys.len() {
            return Err("组队参与列表含重复游戏身份（同一 channel+引继）".into());
        }

        let mut g = self
            .inner
            .lock()
            .map_err(|_| "任务门闩内部锁异常".to_string())?;
        for (i, key) in keys.iter().enumerate() {
            if let Some(r) = g.map.get(key) {
                return Err(format!(
                    "无法启动组队：账号 {} 正在执行「{}」",
                    accounts[i].0, r.task
                ));
            }
        }
        for key in &keys {
            g.map.insert(
                key.clone(),
                Running {
                    task: task.clone(),
                    owner_group: owner_group.clone(),
                },
            );
        }
        Ok(MultiTaskGuard {
            gate: self.clone(),
            keys,
            owner_group,
        })
    }

    /// 查询忙碌任务与发起用户组

    pub fn is_busy(&self, channel: &str, migration_code: &str) -> Option<(String, String)> {
        let key = Self::account_key(channel, migration_code);
        self.inner.lock().ok().and_then(|g| {
            g.map
                .get(&key)
                .map(|r| (r.task.clone(), r.owner_group.clone()))
        })
    }

    /// 仅 **owner_group** 可请求停止语义上的校验（真正中止靠 RunControlFlags；此处供 API 鉴权）

    pub fn owner_group_of(&self, channel: &str, migration_code: &str) -> Option<String> {
        self.is_busy(channel, migration_code).map(|(_, og)| og)
    }

    /// 校验请求用户组是否有权停止该账号当前任务

    pub fn may_stop(
        &self,
        request_group: &str,
        channel: &str,
        migration_code: &str,
    ) -> Result<(), String> {
        match self.is_busy(channel, migration_code) {
            None => Err("该游戏账号当前没有进行中的任务".into()),
            Some((task, owner)) => {
                if owner.is_empty() {
                    return Err(format!(
                        "任务「{task}」未记录发起用户组，拒绝停止（请升级后重试）"
                    ));
                }
                if owner != request_group {
                    return Err(format!(
                        "仅发起用户组「{owner}」可停止该账号任务「{task}」；当前请求组为「{request_group}」"
                    ));
                }
                Ok(())
            }
        }
    }
}
