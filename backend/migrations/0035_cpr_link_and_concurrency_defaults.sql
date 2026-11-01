-- 互联公开权限随 Key 保存
alter table client_api_keys
  add column peer_sharing_json jsonb not null
  default '{"enabled":false,"sharePool":false,"shareQuota":false,"sharePlans":false}';

-- 各额度来源共用当前快照，协议入口另行限制响应大小
alter table provider_accounts drop constraint provider_accounts_quota_ck;
alter table provider_accounts add constraint provider_accounts_quota_ck check (
  provider_quota_json is null or (
    jsonb_typeof(provider_quota_json) = 'object'
    and octet_length(provider_quota_json::text) <= 8388608
  )
);

-- 初始并发策略，已保存的运行设置保持原值
alter table runtime_settings
  alter column max_concurrent_per_account set default 3,
  alter column max_waiting_per_key set default 10,
  alter column max_waiting_per_account set default 10,
  alter column openai_guardian_reserved_concurrency set default 5;

update runtime_settings
set max_concurrent_per_account = 3,
    max_waiting_per_key = 10,
    max_waiting_per_account = 10,
    openai_guardian_reserved_concurrency = 5
where config_revision = 1
  and max_concurrent_per_account = 5
  and max_waiting_per_key = 0
  and max_waiting_per_account = 0
  and concurrency_wait_timeout_seconds = 30
  and openai_guardian_reserved_concurrency = 0;
