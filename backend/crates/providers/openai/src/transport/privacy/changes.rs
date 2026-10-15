//! 暂存处理步骤的正文片段，规则级事务负责提交与回滚

use serde_json::Value;

use super::{Budget, MAX_REQUEST_BYTES, encoded_size};

pub(super) struct BodyChanges {
    original_size: usize,
    replacements: Vec<(String, Value)>,
    removed_bytes: usize,
    added_bytes: usize,
}

impl BodyChanges {
    pub(super) fn new(original_size: usize) -> Self {
        Self {
            original_size,
            replacements: Vec::new(),
            removed_bytes: 0,
            added_bytes: 0,
        }
    }

    pub(super) fn stage(
        &mut self,
        body: &Value,
        path: &str,
        budget: &mut Budget<'_>,
        apply: impl FnOnce(&mut Value, &mut Budget<'_>) -> Result<usize, &'static str>,
    ) -> Result<usize, &'static str> {
        let original = body.pointer(path).ok_or("目标正文片段不存在")?;
        let mut next = original.clone();
        let count = apply(&mut next, budget)?;
        if count > 0 {
            self.removed_bytes += if path.is_empty() {
                self.original_size
            } else {
                encoded_size(original, MAX_REQUEST_BYTES)?
            };
            // 片段互不重叠，其编码字节总量是最终正文大小的下界
            // 提前停止必然超限的改写，避免暂存所有膨胀后的上下文
            self.added_bytes = self
                .added_bytes
                .checked_add(encoded_size(&next, MAX_REQUEST_BYTES)?)
                .filter(|size| *size <= MAX_REQUEST_BYTES)
                .ok_or("请求或预览样本超出大小限制")?;
            self.replacements.push((path.to_owned(), next));
        }
        Ok(count)
    }

    pub(super) fn size(&self) -> Result<usize, &'static str> {
        self.original_size
            .checked_sub(self.removed_bytes)
            .and_then(|size| size.checked_add(self.added_bytes))
            .filter(|size| *size <= MAX_REQUEST_BYTES)
            .ok_or("请求或预览样本超出大小限制")
    }

    pub(super) fn commit(self, transaction: &mut BodyTransaction<'_>) {
        // 路径来自本步骤执行前的正文，片段互不包含且提交前正文尚未改写
        // 仅替换值，不增删这些路径的祖先，因此先前替换不会使后续路径失效
        for (path, next) in self.replacements {
            transaction.replace(path, next);
        }
    }
}

/// 规则内的后续步骤读取前序结果，失败或取消时恢复首次改写前的片段
pub(super) struct BodyTransaction<'a> {
    pub(super) body: &'a mut Value,
    originals: Vec<(String, Value)>,
}

impl<'a> BodyTransaction<'a> {
    pub(super) const fn new(body: &'a mut Value) -> Self {
        Self {
            body,
            originals: Vec::new(),
        }
    }

    fn replace(&mut self, path: String, next: Value) {
        let Some(target) = self.body.pointer_mut(&path) else {
            return;
        };
        let mut original = std::mem::replace(target, next);
        // 已记录祖先时无需重复保存后代，避免多步骤反复保留整份正文
        if self
            .originals
            .iter()
            .any(|(parent, _)| contains(parent, &path))
        {
            return;
        }
        self.originals.retain_mut(|(child, value)| {
            if !contains(&path, child) {
                return true;
            }
            if let Some(target) = original.pointer_mut(&child[path.len()..]) {
                *target = std::mem::take(value);
            }
            false
        });
        self.originals.push((path, original));
    }

    pub(super) fn finish(mut self) {
        self.originals.clear();
    }
}

impl Drop for BodyTransaction<'_> {
    fn drop(&mut self) {
        for (path, original) in self.originals.drain(..) {
            if let Some(target) = self.body.pointer_mut(&path) {
                *target = original;
            }
        }
    }
}

fn contains(parent: &str, child: &str) -> bool {
    child == parent
        || child
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}
