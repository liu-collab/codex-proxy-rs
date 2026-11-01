-- 默认授权名称不代表用户命名，清空后沿用邮箱展示
-- 已有改名审计或没有邮箱的账号保留原值
with migrated as (
  update provider_accounts as account
  set name = '', updated_at = greatest(now(), account.updated_at)
  where account.authentication_kind = 'oauth'
    and nullif(btrim(account.email), '') is not null
    and (
      (account.provider_kind = 'openai' and account.name in (
        'openai OAuth', 'OpenAI 账号', 'OpenAI 賬號', 'OpenAI account',
        'Codex 账号', 'Codex 賬號', 'Codex account'
      ))
      or (account.provider_kind = 'xai' and account.name in (
        'xai OAuth', 'xAI 账号', 'xAI 賬號', 'xAI account'
      ))
    )
    and not exists (
      select 1 from admin_audit_events as audit
      where audit.entity_kind = 'provider_account'
        and audit.entity_ref = account.id
        and 'name' = any(audit.changed_fields)
    )
  returning account.id
)
update runtime_settings
set config_revision = config_revision + 1, updated_at = greatest(now(), updated_at)
where exists (select 1 from migrated);
