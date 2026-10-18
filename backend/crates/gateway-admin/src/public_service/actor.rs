//! 公开服务写入主体由受信宿主边界设置，插件输入不能覆盖

use crate::model::usage_data::UsageActor;
use crate::model::{AdminError, MutationActor, MutationContext};
use std::future::Future;

tokio::task_local! {
    static ACTOR: Option<UsageActor>;
}

/// 与 middleware Origin 独立传递，嵌套服务不会丢失真实调用主体
pub async fn scope_actor<F: Future>(actor: UsageActor, future: F) -> F::Output {
    scope_captured(Some(actor), future).await
}

pub(super) async fn scope_captured<F: Future>(actor: Option<UsageActor>, future: F) -> F::Output {
    ACTOR.scope(actor, future).await
}

pub(super) fn current_actor() -> Result<UsageActor, crate::model::usage_data::UsageDataError> {
    ACTOR.try_with(Clone::clone).ok().flatten().ok_or_else(|| {
        crate::model::usage_data::UsageDataError::invalid(
            "usage mutation requires a trusted caller identity",
        )
    })
}

pub(super) fn mutation_context() -> Result<MutationContext, AdminError> {
    let actor = ACTOR
        .try_with(Clone::clone)
        .ok()
        .flatten()
        .ok_or_else(|| AdminError::invalid("服务写入缺少可信调用主体"))?;
    super::CURRENT
        .try_with(|origin| {
            let (actor, request_id) = match actor {
                UsageActor::Plugin { instance_id, .. } => (
                    MutationActor::System,
                    format!(
                        "plugin:{instance_id}:request:{}:call:{}",
                        origin.request_id, origin.call_id
                    ),
                ),
                UsageActor::Admin { subject } => (
                    MutationActor::AdminSession {
                        admin_user_id: subject,
                    },
                    origin.request_id.clone(),
                ),
                UsageActor::System => (MutationActor::System, origin.request_id.clone()),
            };
            MutationContext { actor, request_id }
        })
        .map_err(|_| AdminError::invalid("服务写入缺少调用上下文"))
}
